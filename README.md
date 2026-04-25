# vstabs

> L1 project tab bar for desktop VS Code — manage local, WSL, and SSH instances as switchable tabs in a single launcher.

**Status:** Pre-alpha. Design phase.

## What it is

VS Code follows the rule "1 window = 1 workspace." When you work across multiple projects — some on your local machine, some in WSL, some on remote servers — you end up with a swarm of windows scattered across your taskbar with no shared organizing layer.

vstabs is a thin wrapper that adds the missing layer: a horizontal tab bar where each tab represents one VS Code window (local / WSL / SSH). Click a tab to bring that window forward; the wrapper does not embed VS Code and does not touch its internals.

```
┌──────────────────────────────────────────────────────────────────┐
│ 🏠 project-main (WSL)  📊 sample-app (WSL)  🖥️ gpu-dev (SSH)  ☁️ … │  ← vstabs (L1)
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│              [ active VS Code window — L2 ]                      │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

## What it is not

- Not a VS Code fork or extension
- Not a terminal multiplexer (use [Zellij](https://zellij.dev/) for that)
- Not a web IDE (use [code-server](https://github.com/coder/code-server) if you need browser access)
- Not a session manager — VS Code already remembers its own state per window

## Why it exists

The "L1 project tab bar" UX has been requested for years across IDEs:
- [VS Code #61710](https://github.com/Microsoft/vscode/issues/61710) — Nesting/grouping of tabs (out-of-scope)
- [VS Code #153826](https://github.com/microsoft/vscode/issues/153826) — Tabs for multiple windows (most-upvoted ever)
- [Zed #45901](https://github.com/zed-industries/zed/discussions/45901) — JetBrains-style project tabs
- [Cursor forum — Multi-Project Workspace](https://forum.cursor.com/t/multi-project-workspace/92547)

None of these have shipped. vstabs solves the problem from the outside: the OS already manages windows, VS Code already manages files — vstabs only adds the missing tab bar that sits on top.

## Status & roadmap

- [ ] **v0.0** — AHK prototype to validate the UX
- [ ] **v0.1** — Tauri rewrite, persistent project registry, environment icons
- [ ] **v0.2** — Two-way sync (VS Code window state → tab bar)
- [ ] Future — lib-x memory tooltips on tab hover

See [`docs/design.md`](docs/design.md) for the full design.

## License

BSD 3-Clause — see [`LICENSE`](LICENSE).

---

## 한국어 부록

VS Code는 "1 창 = 1 워크스페이스" 모델이라, 로컬·WSL·원격 SSH 프로젝트를 여러 개 띄우면 작업표시줄에 창이 흩어져 어느 게 어느 프로젝트인지 한눈에 안 보입니다.

vstabs는 그 위에 얇은 탭바 한 줄을 얹습니다. 각 탭이 하나의 VS Code 창을 가리키고, 클릭하면 해당 창이 활성화됩니다. 환경(local/WSL/SSH)별로 색·아이콘이 달라 시각적으로 즉시 구분됩니다.

VS Code 내부는 손대지 않습니다. 파일 편집·디버그·터미널·LSP·Claude Code 통합 모두 VS Code가 그대로 합니다. vstabs는 창 관리만 합니다.
