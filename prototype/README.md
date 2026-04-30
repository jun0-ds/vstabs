# vstabs v0.0 — AHK prototype

Quick UX sanity check before committing to the Tauri MVP (see [`../docs/design.md`](../docs/design.md)).

## Run

1. Install [AutoHotkey v2](https://www.autohotkey.com/) on Windows.
2. Make sure VS Code's `code` CLI is on PATH (default if installed via the user installer with "Add to PATH" checked).
3. Double-click `vstabs.ahk`. From WSL the UNC path also works:
   `\\wsl.localhost\Ubuntu\path\to\vstabs\prototype\vstabs.ahk`
4. A horizontal tab bar appears centered at the top of the primary monitor.

## Edit projects

Open `vstabs.ahk`, modify the `PROJECTS` array near the top.

Each entry needs:

| field | required for | notes |
|---|---|---|
| `name` | all | display text |
| `env` | all | `local` / `wsl` / `ssh` |
| `path` | all | folder to open (POSIX for wsl/ssh, Windows for local) |
| `icon` | all | single emoji or character |
| `distro` | wsl | e.g. `"Ubuntu"` |
| `ssh_host` | ssh | VS Code Remote-SSH host alias |

## Hotkeys

| Combo | Action |
|---|---|
| Click tab | Activate window if it exists, else `code --remote ...` spawn |
| Ctrl+Alt+1..9 | Activate project N |
| Ctrl+Win+Space | Toggle tab bar visibility |
| Ctrl+Alt+R | Reload the script |

## Known limits (v0.0 by design)

- Hardcoded project list — edit the script to add/remove (registry comes in v0.1)
- Title match = folder basename + env marker. Same-name folders across projects will collide.
- One-way wrapper → VS Code. Alt-tabbing inside VS Code does not update the tab bar (two-way sync is v0.2).
- Tab button widths are estimated, not measured.
- Spawn waits up to 15s for the window; first SSH connect can be slower.
- No persistence: process restart re-discovers via title match.

## What we're checking

If after ~1 week of daily use the UX feels right (project = first-class object, switching is recognition not recall, environment is visually distinct), proceed to v0.1 Tauri. If not, iterate or kill before sinking a week into Rust.
