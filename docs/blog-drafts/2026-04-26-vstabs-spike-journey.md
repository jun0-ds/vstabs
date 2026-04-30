---
title: "1주짜리 Tauri 작업을 30분짜리 Rust spike로 살린 이야기 — vstabs reparent 검증기"
date: 2026-04-26
status: draft
tags: [vstabs, tauri, rust, win32, spike, electron, vscode]
---

# 1주짜리 Tauri 작업을 30분짜리 Rust spike로 살린 이야기

> vstabs v0.1 본 작업 들어가기 전, "VS Code를 컨테이너 창 안에 박는 게 가능한가?"
> 한 줄짜리 가설을 한 줄짜리 Win32 호출로 검증하니, 1주 작업의 전제가 통째로 무너졌다.
> 무너뜨린 게 spike가 한 일이고, 살린 것도 spike다.

## TL;DR

- **목표**: VS Code 여러 인스턴스(local / WSL / SSH)를 **하나의 탭바 컨테이너**로 묶는 데스크톱 앱(`vstabs`) 만들기
- **첫 시도**: AHK v0.0 prototype — 화면 상단 탭바, VS Code 창은 sibling. 사용자가 보고 "이게 내가 원한 게 아니다"
- **진짜 의도**: 컨테이너 창 *안에* VS Code가 들어있는 형태 (JetBrains 멀티 프로젝트 탭처럼)
- **검증**: Tauri로 1주 빌드 들어가기 전, 30분 Rust spike로 `SetParent` reparent feasibility 확인
- **결과**: 시각적으로는 들어가지만 ① 키보드 입력 안 됨 ② Ctrl+C로 컨테이너 죽으면 VS Code 동반 사망 ③ 작업 중인 외부 창 강탈
- **결정**: reparent 영구 폐기 → "spawn-and-track" 소유권 모델 + sibling-slave 방식으로 전환
- **얻은 것**: ADR 두 장, spike 두 개, 추측 → 사실 두 단계, **버려진 1주**

---

## 0. 무대 설정

VS Code는 "1 window = 1 workspace" 룰을 지킨다. 멀티 프로젝트(local 하나, WSL 하나, SSH 하나)를 동시에 쓰면 작업표시줄이 의미 없는 회색 아이콘으로 가득 찬다. 어느 게 어느 프로젝트인지 한눈에 안 보인다.

JetBrains는 한 프로세스가 여러 프로젝트 뷰를 그려서 이걸 푼다. VS Code는 그렇게 안 만든다. GitHub 이슈 [#153826](https://github.com/microsoft/vscode/issues/153826)이 7년째 가장 많은 표를 받고 있지만, 안 들어간다.

`vstabs`는 OS 레이어에서 푸는 시도다. VS Code 내부는 안 건드리고, 위에 얇은 탭바를 얹는다. 처음엔 그게 내가 그린 그림이었다.

## 1. AHK v0.0 — 첫 번째 의도 갭

설계 문서에 박아둔 것:

> "VS Code embed/reparent 거부 — Electron reparent는 렌더링/IME/GPU 깨짐 위험. **sibling 관계로 해결**."

이걸 추측 기반으로 결정해놓고 v0.0 AutoHotkey 프로토타입을 만들었다. 화면 상단에 가로 탭바, 클릭하면 해당 VS Code 창을 `WinActivate`로 앞으로 가져오는 단순한 wrapper. 200줄짜리.

```ahk
^!1::ActivateProject(1)  ; Ctrl+Alt+1 → project-main
^!2::ActivateProject(2)  ; Ctrl+Alt+2 → lib-x
; ...
```

사용자가 실행해보고 보낸 메시지:

> "이렇게 뜨는거 확인했음. 문제는 이게 내가 원하는 화면이 아니었다는거야.
> 윈도우 os 내부에서 별도의 윈도우창을 띄우고 그 안에 탭을 넣고
> 그 안에서 vscode window가 실행되도록 하는게 목적이었음"

설계 문서가 미리 닫아둔 길이 사용자가 진짜 원했던 길이었다. **"위험하다"는 추측에 근거해 닫은 결정이 의도를 이긴 셈**.

## 2. AHK 탈피, "프론트 붙여서 만들자"

사용자가 말한 한 줄:

> "ahk를 탈피합시다. clance 했던것처럼 그냥 프론트 붙여서 프로그램 만들면 안돼?"

`clance`는 사용자가 이전에 만든 시스템 모니터 위젯 — Tauri 기반. 즉 "Tauri 같은 데스크톱 앱 프레임워크로 만들자"는 의미.

설계 문서의 v0.1도 마침 Tauri였다. 그러면 v0.0 AHK 단계 건너뛰고 Tauri로 직행하면 되는가?

**여기서 멈춰야 했다.** 왜냐하면:

- AHK든 Tauri든 reparent의 위험성은 똑같이 안고 간다. SetParent는 둘 다 그냥 `windows` 크레이트 한 줄 호출.
- v0.1 본 작업은 1주짜리. reparent가 깨지면 그 1주가 통째로 버려진다.
- 설계 문서의 reparent 거부 결정은 **추측이었다**. 실제로 해본 적이 없다.

그래서 1주 들어가기 전 30분짜리 spike를 박았다. **추측을 사실로 바꾸는 검증.**

## 3. Spike 설계 — 답해야 할 질문 6개

`SetParent`로 실행 중인 VS Code 창을 컨테이너 창의 child로 박았을 때:

1. 시각적으로 들어가는가?
2. 한글 IME가 동작하는가?
3. GPU 렌더가 멀쩡한가?
4. 포커스/키보드 입력이 흐르는가?
5. 컨테이너 이동/리사이즈에 child가 따라오는가?
6. VS Code 닫기가 우아하게 처리되는가?

이 6개 중 하나라도 깨지면 reparent 설계 폐기. 별도 trade-off 따질 필요 없음.

스택 선택:
- **winit 0.30** — 컨테이너 창 만들기. Tauri 안 씀(검증에 WebView 불필요).
- **windows 0.58** — Win32 API 호출 (`SetParent`, `EnumWindows`, `SetWindowPos`).
- **빌드물은 git 안 들어감** — `.gitignore`에 `target/`, `Cargo.lock`.

## 4. 첫 번째 시행착오 — Rust crate API 변화

작성한 spike 코드를 사용자가 빌드 시도. 컴파일 에러 6개:

```
error[E0599]: no method named `hwnd` found for reference `&Window`
   --> src\main.rs:70:39
    |
70  |         let container_hwnd = HWND(win.hwnd() as *mut _);
    |                                       ^^^^ method not found in `&Window`
```

```
error[E0271]: type mismatch resolving `<Option<HWND> as TypeKind>::TypeKind == CopyType`
    --> src\main.rs:89:42
     |
  89 |             match SetParent(vscode_hwnd, Some(container_hwnd)) {
     |                                          ^^^^^^^^^^^^^^^^^^^^ expected `CopyType`, found `InterfaceType`
```

원인:

1. **winit 0.30이 `WindowExtWindows::hwnd()` 메서드를 제거**했다. 0.29까지는 있었는데, 0.30에서는 `raw-window-handle = "0.6"`을 통해 일원화됨. 마이그레이션:

   ```rust
   use raw_window_handle::{HasWindowHandle, RawWindowHandle};

   fn window_hwnd(win: &Window) -> HWND {
       let handle = win.window_handle().unwrap().as_raw();
       match handle {
           RawWindowHandle::Win32(h) => HWND(h.hwnd.get() as *mut _),
           _ => panic!("not Win32"),
       }
   }
   ```

2. **windows 0.58의 `SetParent` 시그니처가 `Param<HWND>` 트레이트 기반으로 바뀜**. 이전에 `Some(parent)`로 넘기던 게 이제는 `parent` 직접. `None`(=desktop으로 보내기)는 `HWND(std::ptr::null_mut())`로 표현:

   ```rust
   // before (windows 0.52 패턴)
   SetParent(child, Some(parent))
   SetParent(child, None)         // detach

   // after (windows 0.58)
   SetParent(child, parent)
   SetParent(child, HWND(std::ptr::null_mut()))
   ```

LLM이 학습 데이터로 갖고 있는 crate 사용법은 보통 이전 메이저 버전. **새 버전 쓸 거면 변경된 API 시그니처를 한 번 검증하고 들어가야 한다**. 그게 안 되면 첫 빌드에서 에러 5~6개가 따라온다. (오늘 그랬다.)

패치 후 빌드 성공:

```
Finished `release` profile [optimized] target(s) in 5.34s
Running `target\release\reparent-spike.exe`
```

## 5. Spike 실행 — 보이지 않는 깨짐 3개

콘솔 로그:

```
[spike] Container ready. Will attempt SetParent in 2s.
Make sure at least one VS Code window is already open.
[spike] candidate: Write vstabs v0.0 AutoHo… - project-main [WSL: Ubuntu] - Visual Studio Code
[spike] SetParent succeeded. hwnd=HWND(0x400b4c)
```

시각적으로는 컨테이너 안에 VS Code가 들어갔다. **6개 중 1번은 통과**. 그리고 사용자가 다음 메시지를 보냈다:

> "작동은 하는데
> - 타자입력이 안되는거 같음
> - ctrl v로 종료시 기존에 켜있던 vscode 자체가 전부 꺼짐
> - 지금 실행중인 컨트롤타워 vscode가 그 안으로 끌려들어감
> - 기타 확인하지 못한 문제도 있을것임"

3개가 동시에 깨졌다. 각각이 단독으로 reparent 폐기 사유.

### 깨짐 1 — 키보드 입력 불가

VS Code(Electron)는 자기 윈도우가 **top-level**이라고 가정하고 빌드돼 있다. `SetParent`로 child로 만드는 순간:

- 키보드 메시지가 부모 창의 wndproc으로 라우팅됨
- Chromium의 IME composition window가 잘못된 client area에 anchor됨
- 포커스 체인이 OS shell의 peer가 아니라 부모의 sub-element로 인식됨

이걸 `AttachThreadInput` + `SetFocus`로 강제 라우팅할 수 있다고는 한다. 하지만 그러면 IME context가 더 깨지고, Chromium의 합성 가정이 어긋나면서 syntax highlight 같은 게 부분적으로만 동작한다. **2시간 더 파봐도 production-grade 안정성은 안 나온다.**

### 깨짐 2 — 동반 사망

`Ctrl+C`로 spike 프로세스를 강제 종료했더니 컨테이너 안의 VS Code가 함께 죽었다.

이유: Win32에서 child window는 부모의 lifetime에 종속된다. 부모 hwnd가 destroy되면 OS가 child도 destroy한다. spike의 정상 종료 경로(X 클릭 → `WindowEvent::CloseRequested` → `SetParent(child, NULL)`로 detach)는 `Ctrl+C`엔 안 탄다.

production이라면 `SetConsoleCtrlHandler`로 SIGINT 시 detach하는 코드를 박을 수 있다. 하지만 더 큰 문제는 **abnormal exit (crash, kill, BSOD) 모두 동일한 증상**이라는 것. VS Code는 한 인스턴스가 죽으면 unsaved 파일 모두 잃는다. 외부 wrapper의 안정성에 사용자 작업물 lifetime을 묶는 건 받아들일 수 없다.

### 깨짐 3 — 작업 중 창 강탈

가장 충격적이었던 건 이거다. spike의 enumeration 코드가 단순했다:

```rust
unsafe extern "system" fn cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
    // ...
    let title = String::from_utf16_lossy(&buf[..copied as usize]);
    if title.ends_with(" - Visual Studio Code") {
        state.found = Some(hwnd);
        return BOOL(0);  // 첫 매치 잡고 멈춤
    }
    TRUE
}
```

"실행 중인 첫 번째 VS Code 창"을 잡았는데, 그게 사용자가 작업 중이던 창이었다. 강제로 reparent되어서 그 창의 입력이 죽고, 컨테이너 종료 시 함께 죽고, 결과적으로 **사용자가 작업 중이던 코드를 vstabs가 망가뜨렸다**.

enumeration 로직만 고쳐서 "원하는 창만 잡기"는 가능하다. 그런데 사용자의 다음 메시지가 진짜 통찰이었다.

## 6. 모델 차원의 깨달음

> "기본값으로 열리도록 하지 말고 내가 원하는 경로로 vscode프로젝트를 실행할수잇게 하는게 중요한기능인듯"

이건 enumeration 버그 리포트가 아니라 **소유권 모델에 대한 발언**이다.

설계 문서의 JTBD #2가 이미 답을 갖고 있었다: **"멘탈 모델 = 도구 모델: 프로젝트 = 1급 객체"**. 그런데 spike 구현은 무의식적으로 다른 모델을 따랐다 — "지금 떠있는 모든 VS Code 창"이 단위였던 것. 두 모델은 다르다:

| 단위 | 모델 |
|---|---|
| 윈도우 (window-centric) | "OS에 떠있는 모든 VS Code 창을 vstabs가 잡아채서 관리" |
| 프로젝트 (project-centric) | "vstabs 레지스트리에 등록된 N개 entry만 관리. 각 entry는 vstabs가 직접 spawn" |

spike는 첫 번째 모델로 짰고, 그래서 외부 창을 강탈할 수 있었다. 두 번째 모델이면 **구조적으로 강탈이 불가능**하다 — vstabs는 자기가 spawn한 자식의 PID/hwnd만 안다.

이게 reparent 결정과 별개의 두 번째 ADR이 됐다 ([2026-04-26-spawn-and-track-ownership.md](../decisions/2026-04-26-spawn-and-track-ownership.md)).

## 7. 다음 단계 — sibling-slave

reparent를 폐기한 후의 후보:

1. **Strong sibling (slave)** — 컨테이너는 탭바만 그림. VS Code는 여전히 top-level 창이지만, 컨테이너의 위치/크기 변경에 따라 `SetWindowPos`로 강제 동기화. 시각적으론 한 창처럼 보이고, 기술적으론 sibling이라 IME / GPU / lifetime 다 native.
2. **code-server embed** — Tauri WebView 안에 code-server. 진짜 embed. 단점: Claude Code IDE 통합 깨짐 (사용자의 핵심 자산), 서버 운영 필요.
3. **그냥 sibling 탭바** — AHK v0.0 모델 그대로 Tauri MVP로. 한 창 안 들어감. 의도와 다름.

1번을 다음 spike로. 핵심 변경:

- **컨테이너가 직접 spawn** — 외부 창 안 잡음
- **hwnd-diff로 자기 자식 식별** — spawn 전 VS Code hwnd 집합 → spawn 후 집합 → 차집합
- **컨테이너 종료 시 자식 살림** — reparent 안 했으니 detach 불필요. 그냥 untrack

[spike/sibling-slave/src/main.rs](../../spike/sibling-slave/src/main.rs)에 ~200줄로 구현. 아직 실행 결과는 없음.

## 8. 회고 — 30분 spike의 가치

오늘 30분짜리 코드 한 덩어리가 막은 손실:

- **1주 Tauri 본 작업** (reparent 모델로 짰을 경우 키보드 입력 부재가 후반에 발견)
- **사용자 신뢰** (production에서 외부 작업 창이 강탈되면 영구 신뢰 상실)
- **잘못된 기술 부채** (`AttachThreadInput`, IME hook 같은 우회 코드를 1주짜리로 묻어두고 v0.2에서 발견했을 시나리오)

대신 들어간 비용:

- spike 코드 30분 작성 + 컴파일 에러 패치 10분
- 사용자가 빌드 + 실행 5분
- ADR 두 장 작성 20분

**1주 vs 1시간**. spike는 "구현"이 아니라 "측정"이라는 사실을 다시 확인했다. 측정 결과가 negative여도 그건 기술 부채가 아니라 자산이다 — `SetParent`가 실제로 어떻게 깨지는지 본 사람만이 그 다음 결정에서 추측 없이 움직일 수 있다.

## 9. spike 구현 노트 (재현용)

### 가장 짧은 reparent 검증 코드

`Cargo.toml`:
```toml
[dependencies]
winit = "0.30"
raw-window-handle = "0.6"
windows = { version = "0.58", features = [
    "Win32_Foundation",
    "Win32_UI_WindowsAndMessaging",
] }
```

핵심 호출 (전체는 [spike/reparent/src/main.rs](../../spike/reparent/src/main.rs)):

```rust
let original = SetWindowLongPtrW(vscode_hwnd, GWL_STYLE, 0);
SetWindowLongPtrW(vscode_hwnd, GWL_STYLE, (WS_CHILD.0 | WS_VISIBLE.0) as isize);
match SetParent(vscode_hwnd, container_hwnd) {
    Ok(_) => println!("attached"),
    Err(e) => eprintln!("failed: {e:?}"),
}
```

### Tauri 없이 winit으로 spike하는 이유

이번 검증의 본질은 *Win32 reparent의 작동 여부*지 *WebView가 그 위에 올라가는가*가 아니다. Tauri는 WebView 부트 시간이 추가되고 빌드도 더 오래 걸린다. winit은 native window만 띄우고 끝나서 1초 안에 검증 가능. 검증 통과한 가설만 Tauri 본 작업에 옮겨붙인다.

### Hwnd-diff vs PID-tree

`code` CLI는 `cmd /C code <path>` → `Code.exe` launcher → `Code.exe` 메인 → renderer 프로세스 트리를 만든다. `Command::spawn`이 받는 PID는 `cmd`의 것이지 실제 VS Code 창 owner가 아니다. 두 가지 선택:

1. **PID 트리 walk** — `Process32First/Next`로 자손 enumerate, hwnd → PID 역매핑. 정확하지만 코드 양 많음.
2. **Hwnd diff** — spawn 전후 VS Code hwnd 집합 차집합. 5줄. v0.1 prototype에 충분.

후자로 시작. race가 실제로 발생하면 v0.2에서 전자로 업그레이드.

## 10. 끝나지 않은 것

- sibling-slave spike 실행 결과 — 다음 세션에서
- v0.1 Tauri 설계 — sibling-slave 결과 후 design.md 다시 씀
- ADR 두 장의 design.md fold-back

이 글은 그 사이 어딘가에 멈춘 시점의 스냅샷이다.

---

## 참조

- ADR 1 — [Reparent rejected (spike-validated)](../decisions/2026-04-26-reparent-rejected.md)
- ADR 2 — [Spawn-and-track ownership](../decisions/2026-04-26-spawn-and-track-ownership.md)
- spike 1 — [reparent/](../../spike/reparent/)
- spike 2 — [sibling-slave/](../../spike/sibling-slave/)
- 설계 — [design.md](../design.md)
- 관련 GitHub 이슈 — [VS Code #153826](https://github.com/microsoft/vscode/issues/153826), [Zed #45901](https://github.com/zed-industries/zed/discussions/45901)
