# vstabs design

> **Status:** v0.1 architecture (rev 2, 2026-04-30 — full rewrite after spike journey)
> **Decision trail:** [`decisions/`](decisions/) — read in date order
> **Narrative of how we got here:** [`journal/2026-04-30-prior-art-deep-dive.md`](journal/2026-04-30-prior-art-deep-dive.md)

This document is the canonical architecture. Earlier revisions of `design.md` (sibling tab bar, container reparent, etc.) were *hypothesized* designs that did not survive spikes; they are not preserved here. The decision ADRs preserve them for history.

## Problem statement

Desktop VS Code on Windows follows the rule **"1 window = 1 workspace"**. When working across multiple projects (some local, some WSL, some SSH), each becomes a separate VS Code window. The OS taskbar groups them under one icon but offers no project-level visual hierarchy, no environment differentiation, and no fast switching by project name.

This is not a VS Code shortcoming — it's a Windows OS limit. The same problem exists in JetBrains, Zed, Cursor, Windsurf, and Fleet **on Windows**. macOS solves it for free via NSWindow tabbing (System API since Sierra 2016), so JetBrains and Zed have it on Mac via `Window | Merge All Project Windows` and `use_system_window_tabs`. Microsoft tried to add the equivalent ("Sets", 2018) and cancelled it in 2019. The most-upvoted VS Code issue ever ([#153826](https://github.com/microsoft/vscode/issues/153826)) is 7 years unresolved on this same point.

Multi-root workspaces don't solve it — they merge all folders into one workspace, sharing search/LSP/terminal scope, which defeats independent project contexts.

## Jobs to be done

Six JTBDs anchor the design. After the spike journey, two are now recognized as **must-have**, the others as **boost**:

| # | JTBD | Priority |
|---|---|---|
| **2** | Mental model = tool model — a project is a first-class object (name + path + environment), not a folder path | **must-have** |
| **6** | Cognitive offloading — at the OS layer, vstabs collapses N project windows into 1 visual unit (1 taskbar icon, 1 alt-tab entry) | **must-have** |
| 4 | Visual switch map — all available projects always visible in a tab bar; switching is recognition, not recall | boost |
| 5 | Environment as first-class — local / WSL / SSH visually distinct (color, icon) | boost |
| 1 | Context entry cost ≈ 0 — returning to a project should restore its full state | boost (handled by VS Code; vstabs only wakes the tab) |
| 3 | Concurrency + isolation — multiple projects alive in parallel, one visible at a time, no cross-project state bleed | boost |

User's verbatim re-articulation (which became the root cause): **"OS 내에서 들여다봐야 할 창의 수를 계층화"** — collapse the count of windows the user has to attend to at the OS level, while keeping each project instantly identifiable.

The metaphor that closed the loop: **"VS Code를 wrapping하는 브라우저처럼"** — a desktop wrapper whose chrome (tab bar) is provided by vstabs and whose content (the editor) is rendered inside, exactly like a browser hosts web pages.

## Non-goals

- Embedding or modifying VS Code internals (rejected — see [`decisions/2026-04-26-reparent-rejected.md`](decisions/2026-04-26-reparent-rejected.md))
- Replacing VS Code's session state, file management, or extensions
- Cross-platform parity in v0.x (Windows-first; macOS already has the OS-level solution via NSWindow tabbing, Linux is future work)
- Forking VS Code or Chromium (Cursor's depth is not in scope)

## Architecture

```
┌─────────────────────────────────────────────────────────┐  vstabs (Tauri app, ~5 MB)
│ 🏠 project-main  📊 lib-x  🖥 gpu-dev  +             │  L1 — Tab bar (Tauri UI)
├─────────────────────────────────────────────────────────┤
│                                                         │
│         WebView2 (Chromium = system Edge)               │  L2 — Active project
│         loads http://127.0.0.1:{port-N}                 │     content
│         backed by per-project code-server instance      │
│                                                         │
└─────────────────────────────────────────────────────────┘
        ↑ one OS window = one taskbar icon, one alt-tab entry
```

vstabs is a **single OS window**. Inside it, the Tauri shell hosts a tab bar (web UI rendered by Tauri's primary WebView) plus N additional WebView2 instances — one per registered project — each loading its own code-server backend on a dedicated localhost port. Active tab's WebView is shown; others are hidden but kept alive (warm switch).

This is not reparenting (which Chromium rejects), not sibling slaving (which leaves two visible frames), not borderless stripping (which leaves VS Code's custom title bar). It is **the wrapping-browser model literally implemented**: container + WebViews + per-page backend.

### Key components

```
vstabs (Tauri shell, Rust)
├─ Tab bar UI (Svelte/React in Tauri's primary WebView)
├─ WebView pool — one WebView2 per active project
├─ code-server lifecycle manager
│   ├─ port allocator (free port per project)
│   ├─ lazy spawn (start on first tab click)
│   ├─ idle suspend (stop after T minutes of inactivity)
│   └─ health monitor + auto-restart
├─ Project registry (JSON, %APPDATA%\vstabs\projects.json)
└─ Hotkey + tray menu
```

### code-server backend

For each registered project:

| Project env | code-server location | Folder it opens |
|---|---|---|
| `local` | localhost (Windows or WSL) | a Windows or POSIX path |
| `wsl` | inside the WSL distro | a POSIX path in the distro |
| `ssh` | on the remote SSH host (or via Remote-SSH from a local code-server) | a POSIX path on the remote |

vstabs does not embed VS Code Desktop; it runs `code-server` instances and opens each in its own WebView2. The user's existing extensions (including Anthropic's Claude Code, verified) install per-instance from the marketplace. Local jsonl session storage (`~/.claude/projects/*.jsonl`) is shared across instances since they read the same path on disk.

### Why WebView2 (not Firefox / Tauri WRY default)

WebView2 = Chromium = Edge engine, bundled with Windows 10 1809+. It uses the OS's native IME (TSF), so Korean/Japanese/Chinese input works natively — verified directly on Chrome, which uses the same engine. Firefox also passed IME but had layout glitches in some chat UI elements during cross-browser verification, so it's not the wrapper choice.

Tauri's default backend on Windows is WebView2, so this is automatic.

## Project registry

`%APPDATA%\vstabs\projects.json`:

```json
{
  "version": 2,
  "projects": [
    {
      "id": "project-main",
      "name": "project-main",
      "env": "wsl",
      "wsl_distro": "Ubuntu",
      "path": "~/projects/main",
      "color": "#2ea043",
      "icon": "🏠",
      "code_server": {
        "spawn_mode": "lazy",
        "idle_suspend_minutes": 30
      },
      "order": 0
    },
    {
      "id": "gpu-dev",
      "name": "gpu-host",
      "env": "ssh",
      "ssh_host": "gpu-host",
      "path": "~/work",
      "color": "#a371f7",
      "icon": "🖥️",
      "code_server": {
        "spawn_mode": "lazy",
        "host_local_or_remote": "remote",
        "remote_port": 8080
      },
      "order": 1
    }
  ],
  "shortcuts": {
    "global_toggle": "Ctrl+Win+Space",
    "next_project": "Ctrl+Tab",
    "prev_project": "Ctrl+Shift+Tab",
    "select_n": "Ctrl+Alt+1..9"
  }
}
```

Two `code_server` placement strategies coexist:
- **local-host** (default for `local` and `wsl` envs): vstabs spawns a code-server on Windows or inside the WSL distro, allocates a free localhost port, opens the project folder
- **remote** (for `ssh`): code-server runs on the remote host, vstabs port-forwards via SSH tunnel and points its WebView at `http://127.0.0.1:{forwarded}`

### Ops topology assumption (single source of truth)

vstabs assumes the operations topology described in
`ops-config/docs/architecture.md` (private repo). Summary:

- All SSH transport rides Tailscale; external SSH ports are firewall-closed
- The user's primary work environment is a Windows laptop running vstabs as a
  Tauri desktop app; remote hosts are vstabs tabs reachable through the tailnet
- Public domains are reserved for end-user services (Cloudflare Workers);
  development surfaces (vscode-server / code-server / SSH) are never publicly
  exposed

vstabs depends on this model for the SSH backend default below to make sense.
If you are deploying vstabs in a different topology (e.g., team setup with
SAML SSO and public domains), revisit this section first.

### SSH backend security model (v0.2 default)

For personal-use deployments (e.g., Oracle Cloud Free Tier + Tailscale), vstabs enforces a strict tunnel-only model:

- **Remote code-server binds 127.0.0.1 only** — never `0.0.0.0`. Already true of the spawn command vstabs uses; the SSH backend will preserve this when spawning remotely.
- **No public domain exposure** — Cloudflare-tunneled or LB-fronted code-server URLs are an instant data-leak vector (anyone who guesses or scrapes the URL gets full editor + filesystem access). vstabs will not generate or recommend such configurations.
- **Connection path is SSH local port forward only** — vstabs runs an SSH client locally, opens `-L {local_port}:127.0.0.1:{remote_port}`, points its WebView at `http://127.0.0.1:{local_port}`. The remote port is never reachable from the public internet.
- **Tailscale assumed for SSH transport** — the SSH host alias resolves through the user's tailnet, and the remote host has its public SSH port closed at the OS firewall level. vstabs does not implement this; it documents the assumption.
- **An "MCP-only external expose" alternative model** was floated in user discussion but is out of vstabs scope, since vstabs unifies the entire VS Code editor surface, not just MCP endpoints.

## Core flow: tab click

```
on tab_click(project_id):
  registry_entry = registry.get(project_id)
  cs = code_server_pool.get_or_spawn(registry_entry)
  webview = webview_pool.get_or_create(project_id, url=cs.url())
  show(webview); hide(other_webviews)
```

```
code_server_pool.get_or_spawn(entry):
  if pool[entry.id].alive:
    pool[entry.id].mark_active()  # reset idle timer
    return pool[entry.id]
  port = port_allocator.next_free()
  cs = spawn_code_server(env=entry.env, path=entry.path, port=port)
  wait_until_responding(cs.url, timeout=20s)
  schedule_idle_check(cs, after=entry.code_server.idle_suspend_minutes)
  pool[entry.id] = cs
  return cs
```

WebViews are kept alive across tab switches so VS Code session state (open files, scroll, sidebar) is instantly restored. code-server backends are kept alive while in use and suspended after idle to recover memory.

## Phasing

### v0.0 — AHK prototype (shelved, code preserved)
Validated the *wrong* model (sibling tab bar). The spike's value was negative: it surfaced that the user wanted "wrapping" not "tabbing on top of separate windows."

### v0.1 spike — Tauri + 2 WebViews + 2 code-server (1–2 days)
**Last spike** before implementation. Goal: prove the technical core in isolation.
- Tauri shell with a hand-coded tab bar (2 buttons)
- Spawn 2 code-server instances on free ports against 2 different folders
- Show/hide their WebViews on tab click
- Verify: switch latency, memory, IME, Claude Code panel, lifecycle on close

### v0.1 — Implementation (1 week)
Once the spike passes:
- Project registry (JSON, edit via UI in v0.2)
- Tab bar with color/icon, hotkeys (`Ctrl+Alt+1..9`, `Ctrl+Tab`, `Ctrl+Win+Space`)
- code-server lifecycle manager (lazy spawn, idle suspend, health, restart)
- Tray menu, single-instance enforcement
- Local + WSL backends (SSH deferred to v0.2)

### v0.2 — Polish + remote
- SSH backend (port-forward + remote code-server)
- Per-project `--user-data-dir` opt-in for stronger isolation
- Drag-to-reorder tabs, add/remove via UI
- Optional lib-x memory tooltip on tab hover

### v0.3+ — Cross-platform
- macOS: ship as a Tauri app but recommend native `Window | Merge All Project Windows` in JetBrains / Zed if user prefers; vstabs's value on macOS is mainly the local/WSL/SSH unification
- Linux: defer

## Risks & mitigations

| Risk | Mitigation |
|---|---|
| WebView2 Chromium update breaks Korean IME (Chromium has had M124–M125 regressions) | Pin minimum WebView2 version; smoke test Korean input on each Tauri release |
| code-server memory cost (~150–300 MB per instance) | Lazy spawn + idle suspend; on a 16 GB laptop, 4–6 concurrent projects is comfortable |
| code-server lags upstream VS Code | Acceptable — monthly cadence is small lag for a 1-person tool |
| Microsoft proprietary extensions (Pylance, C#) won't install on code-server | Document known incompatibilities; suggest open-source replacements (basedpyright etc.) |
| Auth state per code-server instance | Anthropic Claude Code reads `~/.claude/projects/*.jsonl` natively (verified). Other extensions: case by case in v0.2. |
| WebView pool memory grows with N projects | Recycle WebViews for projects untouched for T hours; recreate on next click (loses session state for that one project) |

## Tech stack rationale

**Tauri 2.x (Rust + WebView2)**
- ~5 MB binary, sub-second startup
- Multi-WebView API (`WebviewWindow` / `Webview`) — direct support for our model
- WebView2 = Chromium = OS native IME on Windows
- Future cross-platform path is real (Tauri supports macOS / Linux)

**code-server (open-source)**
- VS Code Web, runs as a server process
- Marketplace compatible (Open VSX + many Microsoft extensions)
- Native filesystem access on whatever host it runs on (verified for Anthropic Claude Code)
- Active project, monthly releases tracking VS Code

Alternatives considered and rejected:
- **Electron wrapping VS Code Desktop** — embedding external native windows in Electron is not supported (electron/electron#10547), confirmed by spike
- **Native VS Code reparent via Win32 SetParent** — breaks IME / focus / lifetime (spike #1)
- **AHK sibling tab bar** — leaves N OS windows, fails JTBD #6 (spike v0.0)
- **VS Code fork** — Cursor/Windsurf depth, infeasible for one person

## Open questions (to resolve during v0.1 spike)

- **WebView pool**: hide via CSS, hide via Tauri's `set_visible(false)`, or detach from window tree? Pick the one with lowest re-show latency.
- **Process lifecycle on app exit**: stop all code-server instances cleanly? Leave them running (faster next launch) and offer a "stop all" tray action?
- **Hotkey conflicts**: `Ctrl+Tab` clashes with VS Code's editor tab switcher inside the WebView. Global vs focused, or use `Ctrl+Alt+Tab` like the AHK prototype.
- **Multi-monitor**: vstabs window remembers monitor / size; per-monitor instances?
