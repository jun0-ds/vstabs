# vstabs design

## Problem statement

Desktop VS Code has no L1 project tab bar. When working across multiple projects (some local, some WSL, some SSH), each becomes a separate VS Code window. The OS taskbar groups them under one icon but offers no project-level visual hierarchy, no environment differentiation, and no fast switching by project name.

Multi-root workspaces don't solve this — they merge all folders into one workspace, sharing search/LSP/terminal scope, which defeats the goal of independent project contexts.

## Jobs to be done

This design is anchored to six JTBDs:

1. **Context entry cost ≈ 0** — Returning to a project should restore its full state (handled by VS Code; vstabs only wakes the window).
2. **Mental model = tool model** — A project is a first-class object (name + path + environment), not a folder path.
3. **Concurrency + isolation** — Multiple projects alive in parallel; one visible at a time.
4. **Visual switch map** — All available projects always visible in the tab bar; switching is recognition, not recall.
5. **Environment as first-class** — local / WSL / SSH visually distinct (color, icon).
6. **Cognitive offloading** — Tool remembers which projects exist (out of scope: progress/notes — that's lib-x's job, possibly v0.2 integration).

## Non-goals

- Embedding or modifying VS Code internals (fragile, breaks on updates)
- Replacing VS Code's session state, file management, or extensions
- Syncing tab state across devices (the project registry is local; sync via dotfiles if needed)
- Cross-platform parity in v0.x (Windows-first; macOS/Linux is future work)

## Architecture

```
┌────────────────────────────────────────┐
│  vstabs (Tauri app, ~5MB native)       │
│  ┌──────────────────────────────────┐  │
│  │  Tab bar UI (Svelte/React)       │  │
│  └──────────────────────────────────┘  │
│  ┌──────────────────────────────────┐  │
│  │  Window controller (Win32 API)   │  │
│  │  - find VS Code windows by title │  │
│  │  - bring to front / hide         │  │
│  │  - spawn `code` CLI for new tab  │  │
│  └──────────────────────────────────┘  │
│  ┌──────────────────────────────────┐  │
│  │  Project registry (JSON)         │  │
│  └──────────────────────────────────┘  │
└────────────────────────────────────────┘
              ↕  Win32 (no parent/child reparenting)
┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│ VS Code #1   │ │ VS Code #2   │ │ VS Code #3   │
│ (control-    │ │ (sample-app,    │ │ (gpu-dev,    │
│  tower, WSL) │ │  WSL)        │ │  SSH)        │
└──────────────┘ └──────────────┘ └──────────────┘
```

vstabs and VS Code windows are **siblings under the OS**, not parent/child. This avoids Electron reparenting fragility (rendering glitches, focus/IME issues, GPU acceleration loss).

## Environment types

Three types cover all current cases. Cloud machines (Oracle Cloud, AWS, etc.) reach via Tailscale SSH and register as `ssh` — no separate `cloud` or `web` type needed.

| Type | Launch command |
|---|---|
| `local` | `code C:\path` |
| `wsl` | `code --remote wsl+{distro} /path` |
| `ssh` | `code --remote ssh-remote+{host} /path` |

A `web` type was considered (for code-server / vscode.dev) and rejected:
- Desktop VS Code over SSH already gives the "remote workspace, local UI" experience without running a separate server
- Web IDEs break Claude Code IDE integration (no PTY, no local fs)
- Tailscale SSH covers the multi-device access scenario that web IDEs are usually pitched for

## Project registry

`%APPDATA%\vstabs\projects.json` (or `~/.config/vstabs/projects.json` on POSIX):

```json
{
  "version": 1,
  "projects": [
    {
      "id": "project-main",
      "name": "project-main",
      "env": "wsl",
      "wsl_distro": "Ubuntu",
      "path": "~/projects/main",
      "color": "#2ea043",
      "icon": "🏠",
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
      "order": 1
    },
    {
      "id": "oracle-cloud",
      "name": "oracle-arm",
      "env": "ssh",
      "ssh_host": "oracle-arm.tail-scale.ts.net",
      "path": "~/work",
      "color": "#f59e0b",
      "icon": "☁️",
      "order": 2
    }
  ],
  "shortcuts": {
    "global_toggle": "Ctrl+Win+Space",
    "next_project": "Ctrl+Tab",
    "prev_project": "Ctrl+Shift+Tab"
  }
}
```

## Core flow: tab click

```
on tab_click(project_id):
  proj = registry.get(project_id)
  hwnd = find_vscode_window(proj)

  if hwnd exists:
    bring_to_front(hwnd)
  else:
    cmd = build_code_command(proj)
    spawn(cmd)
    wait_for_window(proj, timeout=15s)  # SSH/WSL can be slow

find_vscode_window(proj):
  # VS Code window title patterns:
  #   "{folder} - Visual Studio Code"
  #   "[WSL: Ubuntu] {folder} - Visual Studio Code"
  #   "[SSH: host] {folder} - Visual Studio Code"
  pattern = build_title_pattern(proj)
  return EnumWindows(filter=pattern)
```

## Risks & mitigations

| Risk | Mitigation |
|---|---|
| VS Code window title format changes between versions | Title patterns isolated to a config-driven matcher; version-test on each VS Code release |
| Two projects with the same folder name collide on title match | Disambiguate via `--user-data-dir` per registered project, track PID alongside title |
| Remote workspace startup is slow (5–15s) | Show loading spinner on tab; optional "warm pool" (pre-launch idle windows) in v0.2 |
| Wrapper crash leaves VS Code windows orphaned | Lossless — windows survive independently; relaunching wrapper re-discovers them |
| User alt-tabs in VS Code, wrapper tab bar doesn't reflect active window | v0.1 is one-way (wrapper → VS Code). Two-way sync via `SetWinEventHook` is v0.2 |

## Phasing

### v0.0 — AHK prototype (1 day)

Goal: Validate the UX before committing to a Tauri build. AutoHotkey v2 can do Win32 window enumeration, hotkeys, and basic GUI in ~200 lines.

- Hardcoded project list
- Top-of-screen tab bar (tooltip text + click)
- `code --remote ...` spawn
- `FindWindow` + `BringWindowToTop` for activation

If after 1 week of using this prototype the UX feels right → proceed to v0.1. If it doesn't → iterate or kill.

### v0.1 — Tauri MVP (1 week)

- Project registry (JSON, edit via UI)
- Persistent tab bar (always-on-top window)
- Environment color/icon
- Hotkeys (`Ctrl+1..9`, `Ctrl+Tab`)
- Tab reorder (drag), add/remove
- Single-instance enforcement

### v0.2 — Polish

- Two-way sync (`SetWinEventHook` to track VS Code window focus changes)
- Tab grouping (e.g., "WSL projects" / "SSH projects")
- Optional lib-x memory tooltip on tab hover (read `memory/domain/{project}.md`)

### v0.3+ — Cross-platform

- macOS via `NSWorkspace` + Accessibility API
- Linux via wmctrl / X11 / Wayland (last priority)

## Tech stack rationale

**Tauri 2.x (Rust + WebView)**
- ~5MB binary, sub-second startup
- Native Win32 access via `windows` crate
- WebView for UI lets us iterate fast on visual design
- Future cross-platform path is real (Tauri supports all three OSes)

Alternatives considered:
- **Electron** — 150MB binary, heavy startup. Rejected.
- **AHK v2** — Perfect for v0.0 prototype, weak for production UI. Used only for prototype.
- **C# + WinUI** — Windows-only forever. Rejected for cross-platform reasons.

## Open questions (to resolve before v0.1)

- Tab bar position: top-of-screen always-on-top vs. dock-able sidebar?
- Multi-monitor: tab bar on primary monitor only, or per-monitor?
- Should VS Code windows snap to a specific monitor when activated?
- Hotkey conflicts with VS Code's own `Ctrl+Tab` — global vs. focused?
