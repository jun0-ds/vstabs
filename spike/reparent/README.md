# reparent-spike

> 30분짜리 feasibility 검증. **빌드물은 git 안 들어감 (`.gitignore`로 차단).**

## 답할 질문

`SetParent`로 실행 중인 VS Code 창을 컨테이너 창에 박았을 때:

1. 시각적으로 들어가는가?
2. 한글 IME 입력이 동작하는가?
3. GPU 렌더(에디터 스크롤, syntax highlight 애니메이션)가 멀쩡한가?
4. 포커스/키보드 입력이 흐르는가?
5. 컨테이너 이동·리사이즈에 child가 따라오는가?
6. VS Code 창을 닫으면 우아하게 처리되는가?

이 6개 중 하나라도 깨지면 → reparent 설계 폐기, 강화 sibling 또는 다른 방향.

## 빌드 환경

- **Windows (WSL 아님)**. winit/windows-rs는 Windows native 타깃 필요.
- Rust toolchain 필요. 없으면 https://rustup.rs/ 한 줄 설치.

## 실행

```powershell
# 1) VS Code 아무 창이나 하나 띄워둔다 (local/WSL/SSH 무관)

# 2) 빌드 + 실행
cd \\wsl.localhost\Ubuntu\path\to\vstabs\spike\reparent
cargo run --release
```

→ 1280x800 컨테이너가 뜨고, 2초 후 VS Code 창이 그 안으로 빨려들어옴.
→ 콘솔에 `SetParent succeeded. hwnd=...` 또는 실패 사유 출력.

## 테스트 체크리스트

컨테이너 안의 VS Code에서 직접 시도:

- [ ] 한글 입력 (`한/영` 키 → 아무거나 타이핑)
- [ ] 자동완성 팝업이 컨테이너 안에 그려지는가 (또는 잘리는가)
- [ ] 사이드바 토글 (`Ctrl+B`) 시 화면이 깜빡이거나 깨지는가
- [ ] 에디터 스크롤이 부드러운가 (GPU)
- [ ] 컨테이너 이동/리사이즈 시 VS Code가 따라오는가
- [ ] Claude Code IDE 패널을 열어서 채팅 입력 가능한가 (한글 포함)

## 종료

컨테이너 창 X 클릭 → `SetParent(hwnd, None)`로 VS Code를 다시 top-level로 복원하고 종료.

비정상 종료(crash, kill)면 VS Code가 invisible/orphan 상태가 될 수 있음. 이 경우 작업관리자에서 VS Code 종료 후 재기동.

## 결과 기록

이 spike는 production 코드 아님. 결과만 `docs/spike-reparent-result.md`(또는 design.md의 Open questions 섹션)에 기록하고 본 작업으로 넘어감.
