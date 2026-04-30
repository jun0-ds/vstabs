# sibling-slave-spike

> Feasibility check #2. Implements the spawn-and-track ownership model
> (see [`../../docs/decisions/2026-04-26-spawn-and-track-ownership.md`](../../docs/decisions/2026-04-26-spawn-and-track-ownership.md)).

## What it answers

After [reparent was rejected](../../docs/decisions/2026-04-26-reparent-rejected.md), the next-best UX is "container window with VS Code visually slaved to it." The question:

1. Does **hwnd-diff capture** reliably identify the VS Code window that *we* spawned, ignoring others?
2. When the user moves/resizes the container, does the child VS Code follow smoothly enough to feel like a single window?
3. When the container closes, does the child VS Code keep running normally (since no reparent = no lifetime coupling)?
4. Does Korean IME / focus / keyboard input remain native (it should — we never touch the VS Code window's parent or style)?

## Edit before running

`src/main.rs`, top of file:

```rust
const TARGET_PATH: &str = r"C:\Temp";  // change to a path you want VS Code to open
```

Pick a path **VS Code is not already open at** (otherwise `--new-window` still creates a fresh hwnd, but the test is cleaner with an unused folder).

## Run

Pre-existing VS Code windows can stay open — the spike will snapshot and ignore them. That's the whole point.

```powershell
cd C:\Temp\sibling-slave-spike
cargo run --release
```

Expected console output:

```
[spike] container ready. spawning VS Code at C:\Temp ...
[spike] N pre-existing VS Code windows snapshotted (ignored).
[spike] launched `code --new-window C:\Temp` (cmd PID 12345). ...
[spike] captured new VS Code hwnd=HWND(0x...) after 3.2s
```

A new VS Code window appears positioned inside the container's client area (below a 30px gap that simulates the future tab bar).

## Test checklist

- [ ] **Ownership** — pre-existing VS Code windows are untouched. Only the new one is moved.
- [ ] **Type Korean** in the captured VS Code (`한/영` → 아무거나). Should work normally — no reparent done.
- [ ] **Move container** by dragging its title bar. Child VS Code should follow.
- [ ] **Resize container** from any edge. Child should resize.
- [ ] **Alt-tab to other apps and back** — does the container + child surface together, or separately?
- [ ] **Close container** (X button). Child VS Code should keep running as a normal top-level window.
- [ ] **Close child VS Code** (its X button) while container is still alive. Container should not crash; spike currently doesn't re-detect, just leaves child_hwnd dangling.

## Known limits (spike-only)

- Hardcoded single project path. Production registers multiple.
- Polling is 200ms. SetWinEventHook gives instant updates; v0.1 will use it.
- No focus coupling: alt-tabbing inside child VS Code does not raise the container, and vice versa. Production needs `EVENT_SYSTEM_FOREGROUND` hook.
- Child geometry follows container, not the other way around. If the child is moved by the user (e.g. Win+arrow snap), the container does not chase. v0.1 needs two-way or "snap back on container input."

## Result handling

Record outcome in `../../docs/decisions/2026-04-26-sibling-slave-result.md` (new ADR). If pass → v0.1 Tauri implementation. If "feels too floaty" → fall back to plain sibling tab bar (option C).
