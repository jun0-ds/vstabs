# vstabs

> **One Windows desktop window. N project tabs. Local + WSL + SSH backends.**
> A thin Tauri shell that wraps `code-server` instances behind a project-aware
> tab bar — the multi-project tab UX that VS Code on Windows has been missing
> for [seven years](https://github.com/microsoft/vscode/issues/153826).

[![License: BSD-3](https://img.shields.io/badge/License-BSD_3--Clause-blue.svg)](LICENSE)

```
┌────────────────────────────────────────────────────────────┐  ← single OS window
│ 🏠 sample-local  🐧 sample-wsl  ☁️ sample-cloud  +        │  ← L1: vstabs tab bar
├────────────────────────────────────────────────────────────┤
│                                                            │
│         WebView2 (Edge / Chromium) — active project        │  ← L2: code-server
│         Sidebar · Editor · Terminal · Claude Code panel    │     in WebView2
│         Native IME, native keyboard, native everything     │
│                                                            │
└────────────────────────────────────────────────────────────┘
        ↑ taskbar shows ONE icon, alt-tab shows ONE entry
```

## Why

Desktop VS Code on Windows enforces *"1 window = 1 workspace."* When you work
across multiple projects (some local, some WSL, some on a remote dev box), each
becomes a separate top-level window — the taskbar fills with indistinguishable
icons, alt-tab becomes a guessing game, and there is no visual hierarchy.

This is not a VS Code shortcoming. macOS solves it via NSWindow tabbing
(JetBrains and Zed expose it as `Window | Merge All Project Windows`). Windows
has no equivalent OS API — Microsoft tried with "Sets" in 2018 and shelved it
in 2019. JetBrains, Zed, Cursor, and Fleet all leave the gap open on Windows.

vstabs closes the gap **from outside the editor**: a small Tauri container
hosts N WebView2 instances, each connected to a per-project `code-server`
backend. To the OS it is one window with one taskbar entry. To you, it is the
JetBrains-on-Mac tab UX, brought to Windows.

See [`docs/design.md`](docs/design.md) for the full architecture and the four
ADRs in [`docs/decisions/`](docs/decisions/) for how the model arrived.

## Status

| Layer | State |
|---|---|
| Model | ✅ verified (Tauri + WebView2 + per-project code-server) |
| Local + WSL backends | ✅ v0.1 |
| SSH backend (Tailscale-friendly) | ✅ v0.2 D |
| JSON registry + add-project UI | ⏳ v0.2 A |
| Idle suspend, global hotkey | ⏳ v0.2 B/C |
| macOS / Linux ports | future (macOS already has the OS-level solution) |

This is a personal-scale tool, MIT-spirited, BSD-3 licensed. Production-grade
for one-developer workflows; not yet shaped for teams.

## How it differs

- **Not a VS Code fork or extension.** vstabs runs on top of unmodified
  upstream `code-server`.
- **Not a web IDE.** Looks like a desktop app, behaves like one — native IME,
  native keyboard shortcuts, native window controls. The browser is hidden
  inside Tauri's WebView2.
- **Not a terminal multiplexer.** If your workflow is terminal-first, use
  [Zellij](https://zellij.dev/) or [tmux](https://github.com/tmux/tmux) instead.
- **Not a session manager.** VS Code already remembers its own state per
  workspace; vstabs only routes you to the right one.
- **Not a launcher with a hotkey.** Switching tabs is < 50 ms (warm WebView);
  no spawn delay after the first activation.

## Requirements

- Windows 10 (1809+) or Windows 11 — WebView2 runtime ships with the OS
- Rust toolchain (stable) for building from source — see [`BUILD.md`](BUILD.md)
- `code-server` installed wherever the project lives (local, WSL distro, SSH host)

## Quick start

1. Build the binary — see [`BUILD.md`](BUILD.md). One-liner if you have Rust + Tauri CLI:
   ```powershell
   git clone https://github.com/jun0-ds/vstabs && cd vstabs
   cargo tauri build
   ```
   Or [cross-compile from WSL](BUILD.md#cross-compile-from-wsllinux-to-windows-advanced)
   if you don't want a Windows toolchain on your laptop.

2. Edit [`src-tauri/src/registry.rs`](src-tauri/src/registry.rs) to point at your
   own folders / WSL distro / SSH hosts. (The bundled defaults are placeholders.)
   v0.2 A will replace this with `%APPDATA%\vstabs\projects.json` and a UI.

3. Install `code-server` on every host that backs a tab:
   ```bash
   curl -fsSL https://code-server.dev/install.sh | sh -s -- \
     --method standalone --prefix $HOME/.local
   ```

4. Run `vstabs.exe`. Click a tab → backend lazy-spawns → editor appears.

## SSH backend

For SSH tabs, vstabs runs the user's `ssh` client with a dynamic local port
forward and a one-line remote command, so the **ssh client's lifetime owns
both the tunnel and the remote `code-server`**. Killing vstabs cleanly
terminates the remote process — no orphans on the dev box.

The default security model is documented in
[`docs/design.md`](docs/design.md#ssh-backend-security-model-v02-default):
SSH-tunnel-only, no public exposure of `code-server`, Tailscale assumed for
transport. Setting up Cloudflare-tunneled `code-server` URLs is an instant
data-leak vector and vstabs deliberately does not support it.

## Documentation

- [`BUILD.md`](BUILD.md) — build, run, troubleshoot, cross-compile recipe
- [`docs/design.md`](docs/design.md) — architecture, JTBDs, registry schema, risks
- [`docs/decisions/`](docs/decisions/) — four ADRs covering the spike journey
- [`docs/journal/`](docs/journal/) — narrative of how the architecture was found
- [`docs/blog-drafts/`](docs/blog-drafts/) — long-form write-up of the spike
  journey (publish-when-polished)
- [`spike/`](spike/) — four shelved spikes preserved as learning artifacts

## Contributing

Issues and PRs welcome, but expect slow review cycles — this is a one-person
side project. Before opening a feature PR, please open an issue first to align
on whether it fits the JTBD set in [`docs/design.md`](docs/design.md).

## License

[BSD 3-Clause](LICENSE).

---

## 한국어 요약

Windows에서 VS Code가 "1 창 = 1 워크스페이스"를 강제해서 프로젝트가 여러 개면
작업표시줄이 같은 아이콘으로 가득 찹니다. macOS는 NSWindow tabbing으로 OS 차원
에서 풀어주지만 (JetBrains, Zed 모두 macOS에서 multi-project tab 지원), Windows
는 같은 OS API가 없어 어떤 IDE도 못 풀고 있습니다 — Microsoft도 2018년 Sets로
시도했다가 2019년에 폐기했습니다.

vstabs는 외부 wrapper로 이 갭을 메웁니다. 작은 Tauri 데스크톱 앱 안에 N개의
WebView2를 띄우고, 각 WebView가 자기 프로젝트의 `code-server`에 붙습니다. OS
입장에선 한 윈도우, 한 작업표시줄 항목. 사용자 입장에선 macOS의 JetBrains 같은
multi-project 탭 UX. SSH 탭은 Tailscale 전제 + ssh 터널만 사용하므로 외부 도메인
노출 없이 원격 프로젝트도 탭 하나로 다룹니다.

자세한 설계는 [`docs/design.md`](docs/design.md), 결정 흐름은
[`docs/decisions/`](docs/decisions/) 참조.
