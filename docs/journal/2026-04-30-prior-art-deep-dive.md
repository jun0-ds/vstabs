---
title: "vstabs 탐색 중간기록 — 4 spike fail → prior art 깊이 분석 → OS 차원 root cause → 최종 모델 결정"
date: 2026-04-30
status: closed
final_decision: "code-server-revived ADR — Tauri container + multi-WebView2 + per-project code-server"
---

# 탐색 중간기록 — 2026-04-26 ~ 04-30

이 문서는 narrative 중간 기록입니다. 결정은 ADR로 따로 박혀 있고(`../decisions/`), 이건 **무엇을 시도했고 왜 막혔는지** 흐름을 한 번에 볼 수 있도록 정리한 것입니다.

## 시도한 것 4가지 (모두 폐기)

### 1. AHK v0.0 prototype (sibling 탭바)
- 화면 상단 탭바, VS Code는 별도 top-level
- **사용자 거부**: "이게 내가 원한 화면이 아니다"
- 의도 불일치: 사용자가 원한 건 컨테이너 안에 VS Code

### 2. Reparent spike — `SetParent`로 VS Code를 컨테이너 child화
- 시각적으론 들어감
- ❌ 키보드 입력 안 됨
- ❌ Ctrl+C 종료 시 VS Code 동반 사망
- ❌ 작업 중인 외부 VS Code 창 강탈
- → ADR [`reparent-rejected`](../decisions/2026-04-26-reparent-rejected.md)
- 부산물 ADR: [`spawn-and-track-ownership`](../decisions/2026-04-26-spawn-and-track-ownership.md)

### 3. Sibling-slave spike — VS Code를 별도 창으로 spawn + SetWindowPos로 위치 슬레이브
- spawn 정상, 위치 동기화 정상
- ❌ 두 창이 OS 레벨에서 분리된 채 시각적으로 떠 있음 (감싸는 시각 못 만듦)
- → 사용자: "embed가 아니고 그냥 좌표고정인데요?"

### 4. Borderless spike — sibling-slave + VS Code의 WS_CAPTION/WS_THICKFRAME 비트 제거
- frame 일부 사라짐
- ❌ VS Code 자체 custom title bar는 그대로 남음
- ❌ 본질적으로 두 OS 창 분리 (VS Code의 X/min/max 버튼 그대로 보임)
- → 사용자: "근본적으로 감싸는 형태가 되지 못했는데요. 왜 자꾸 좌표 기반으로 가는 거죠?"
- 이 시점에 모든 spike 폐기 결정

### Verification: code-server (브라우저 안 VS Code) — Tauri WebView 후보
- 설치 + Anthropic Claude Code 확장 설치 + 사이드바 패널 정상 + 로컬 jsonl 세션 액세스 모두 동작
- ❌ **한영키 안 먹음** — 브라우저 IME가 OS-level Hangul 토글을 못 받음
- → ADR [`code-server-rejected`](../decisions/2026-04-28-code-server-rejected.md)
- (한영키 우회 방법 재검토는 본 문서 작성 후 별도 진행 — `## 진행 중` 섹션 참조)

## Prior art 본격 조사 — 4가지 결정적 발견

### 발견 1. macOS는 OS 차원에서 이미 풀어 둠
- macOS Sierra (2016) — `NSWindow.addTabbedWindow()` API + WindowServer 레벨 관리
- **NSDocument 기반 앱은 0줄 코드 변경으로 자동 탭 지원**
- JetBrains IDE의 `Window | Merge All Project Windows` 기능 — **macOS only**, JetBrains 공식: *"This feature was introduced on Mac because the system Mac API supports it. Right now, JetBrains is counting the demand for this feature implementation for Linux/Windows"*
- Zed의 `use_system_window_tabs: true` — 같은 macOS 기능 활용
- → **사용자가 그린 L0/L1/L2 그림은 macOS에서는 OS 표준 기능**

### 발견 2. Microsoft가 "Sets"로 정확히 같은 걸 만들려다 폐기 (2018→2019)
- Sets 슬로건: *"native tabs any application could use"*
- Windows 10 19H1에 출시 예정이었음
- 2019년 4월 조용히 cancelled
- 폐기 이유:
  - 너무 복잡해짐 (탭마다 Edge browser engine embed까지 포함)
  - 새 Chromium-based Edge가 Sets 구현을 더 어렵게 함
  - 결과: 보편적 tabbing 포기, **per-app tabbing**으로 후퇴 (Terminal, Notepad, File Explorer만 자체 탭)
- → **OS owner인 Microsoft 자신도 못 한 깊이의 문제**

### 발견 3. Third-party wrapping은 시장 검증된 dead-end
- **WindowTabs** — discontinued, modern Electron 앱에서 깨짐
- **TidyTabs** — 시판 중이지만 Electron 호환성 들쑥날쑥
- 모두 child window reparenting 모델 → 우리 spike와 동일한 fail 패턴
- VS Code Issue [#153826](https://github.com/microsoft/vscode/issues/153826) — 가장 많은 표 받은 이슈, 7년째 미해결

### 발견 4. Chromium의 외부 reparent 거부는 설계 의도
- VS Code = Electron = Chromium
- Chromium은 자기 윈도우의 input/IME/GPU compositing을 강하게 own
- SetParent로 child화 시 IME composition window의 anchor가 깨지고 input event 라우팅이 부모로 빠짐
- → **Chromium 보안/안정성 모델의 일부**. 버그 아님. Sandbox isolation 정신과 일관.
- 우회는 Chromium fork 필요 (Cursor도 안 한 깊이)

## 시장 현황 정리 — 누가 어떻게 푸는가

| 도구 | OS | multi-project tab |
|---|---|---|
| JetBrains (IntelliJ 등) | macOS | ✅ Window > Merge All Project Windows |
| JetBrains | Windows / Linux | ❌ "demand counting 중" |
| Zed | macOS | ✅ use_system_window_tabs |
| Zed | Windows / Linux | ❌ 미지원, issue 다수 |
| VS Code | All | ❌ 7년 미해결 |
| Cursor / Windsurf | All | ❌ VS Code fork지만 이 부분은 안 손댐 |
| Fleet (JetBrains) | All | ❌ "Merge Windows" 사용자 요청 있음 |

→ **Windows에서 multi-project tab은 어떤 IDE도 못 하고 있음**. OS 차원 제약.

## 진짜 root cause (3겹)

1. **OS 아키텍처 차이**: macOS Cocoa 단일 framework vs Windows의 5개 framework 단편화 (Win32/WinForms/WPF/UWP/WinUI). macOS는 한 API 추가가 system-wide; Windows는 모든 framework 호환 어려움.
2. **Microsoft의 의지 부족**: Sets 폐기 이후 per-app tabbing으로 후퇴. 자체 앱(Terminal/Notepad/Explorer)만 탭, system-wide API는 안 만듦.
3. **Modern app architecture (Chromium)이 외부 reparent 거부**: 보안/안정성 설계. 우회 = fork 필요.

## 사용자 통찰의 의미 재해석

처음 사용자가 그린 L0/L1/L2 그림:
- L0 = vstabs 컨테이너 frame
- L1 = 가로 탭바 (프로젝트 = 1급)
- L2 = 활성 VS Code 콘텐츠

이건 **Windows OS의 한계 자체를 우회하는 그림**. IDE 개선이 아니라 OS 기능 보강. 1인 프로젝트 범위 밖.

## 4 path 결론

| | 무엇 | 비용 | 현실성 |
|---|---|---|---|
| 1 | macOS 이전 → JetBrains/Zed | OS가 이미 풂 | 환경 변경 의지 |
| 2 | Windows에서 "한 창 안" 포기 → AHK v0.0 + 시각 강화 + AUMID 그룹화 | 1주 | **현실적** |
| 3 | VS Code fork (Chromium까지) | 수십 인년 | 1인 불가 |
| 4 | Microsoft Sets v2 기다림 | 무한정 | 7년 무소식 |

## 결정적 반전 — Cross-browser 검증으로 IME 결론 뒤집힘

사용자 제안 (2026-04-30): "크로미움 말고 그냥 다른 브라우저에서도 테스트 해보면 되는 거 아닌가요?"

5분 검증으로 모든 게 뒤집혔습니다.

### 검증 절차
- WSL에서 code-server 재가동
- Windows host의 native browser 두 개에서 직접 접속:
  - **Firefox** — 한글 입력 ✅, Claude Code 한글 채팅 ✅, UI 일부 layout 깨짐 ⚠️
  - **Chrome** — 한글 입력 ✅, Claude Code 한글 채팅 ✅, UI 깔끔 ✅

### 진단 정정
이전 "한영키 안 먹어요"는 **Playwright headless `chromium-headless-shell` 환경 한계**였습니다. Headless 자동화의 keyboard event는 OS IME mode toggle을 trigger 못 함. 이걸 "구조적 한계"로 잘못 일반화.

→ **Native browser에서 IME 정상 동작 확인**. code-server 모델의 유일한 deal-breaker가 사라짐.

## 최종 모델 결정 — Tauri 컨테이너 + multi-WebView2 + per-project code-server

처음에 사용자가 말한 *"VS Code를 wrapping하는 브라우저처럼 작동"*의 정확한 구현입니다. 그 비유 자체가 정답이었음 — 우리는 4번 spike와 prior art 일주를 거쳐 그 자리로 돌아옴.

상세는 [`../decisions/2026-04-30-code-server-revived.md`](../decisions/2026-04-30-code-server-revived.md). 핵심 그림:

```
┌──────────────────────────────────────────────┐  vstabs (Tauri app)
│ 🏠 project-main  📊 lib-x  🖥 gpu-dev  +  │  L1 — 탭바 (Tauri UI)
├──────────────────────────────────────────────┤
│                                              │
│   WebView2 (Chromium = OS native IME)        │  L2 — 활성 프로젝트
│   loads localhost:{port-N}                   │     (code-server backend)
│                                              │
└──────────────────────────────────────────────┘
```

- vstabs = 1개 OS 윈도우 → 작업표시줄 1개 + alt-tab 1개 (JTBD #6 만족)
- 탭마다 별도 code-server 인스턴스 (JTBD #2, #3 만족)
- WebView2 = OS native IME (한국어 입력 동작)
- 처음에 거부한 사유 모두 무너짐: Claude Code 통합 ✅(spike 검증), IME ✅(cross-browser 검증)

## 학습 — 왜 이렇게 늦게 도착했나

1. **Spike 결과 일반화의 함정**: 한 번의 negative result(Playwright headless)를 "구조적 한계"로 단정. Cross-browser 검증으로 5분에 깨질 결론을 ADR로 박음.
2. **좌표 기반 anchoring**: reparent 폐기 후 sibling-slave → borderless로 후퇴할 때마다 "한 패치만 더"식 sunk-cost 추구. Prior art 한 번 진지하게 읽었으면 절약했을 시간.
3. **사용자가 root을 잡음**: "근본적으로 감싸는 형태가 못 됐다", "왜 자꾸 좌표 기반?", "다른 브라우저 테스트는?" — 매 굴절점에서 사용자의 한 줄이 reasoning을 reset. 비용 든 학습이지만 모델은 검증된 자리에 도착.

## 다음 (이번 세션 마무리)

- [x] 본 journal 갱신 (closed)
- [x] code-server-rejected ADR Superseded 표시
- [x] code-server-revived ADR 작성
- [ ] design.md 전면 재작성 — 이번 turn에 진행
- [ ] v0.1 spike (마지막) — Tauri 안 두 WebView2 + 두 code-server 포트 + 탭 전환 검증. ~1–2일.
