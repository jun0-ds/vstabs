# borderless-spike

> Feasibility check #3. Sibling-slave + strip child's WS_CAPTION/WS_THICKFRAME.
> Closest reachable "wrapping browser" UX without reparent.

## What it answers

After sibling-slave (#2) showed two visually-separate frames, this spike asks:

1. If we strip the OS frame bits from the child VS Code window (no reparent), does it visually nest inside the container?
2. Does input / IME / GPU still work (it should — no reparent)?
3. On container close, does frame restoration return the child to a normal usable state?

## Pre-flight (recommended)

For the cleanest visual, set VS Code's title bar to native first:

```jsonc
// %APPDATA%\Code\User\settings.json
{
  "window.titleBarStyle": "native"
}
```

Restart VS Code so the setting takes effect. Without this, VS Code's own custom title bar still shows even after we strip the OS frame.

## Run

```powershell
cd C:\Temp\borderless-spike
cargo run --release
```

Pre-existing VS Code windows are snapshotted and ignored — only the spawned child is touched.

Expected console:
```
[spike] container ready. spawning VS Code at C:\Temp ...
[spike] N pre-existing VS Code windows snapshotted (ignored).
[spike] launched `code --new-window C:\Temp` (cmd PID ...)
[spike] captured hwnd=HWND(0x...) after X.Xs
        title="..."
        class="Chrome_WidgetWin_1"
[spike] original style=0x...
[spike] stripped style -> 0x...
[spike] layout #1: req=(...) ..., actual=(...) ..., swp=Ok(())
```

## Test checklist

- [ ] **Visual nesting** — VS Code appears inside the container area (no OS title bar / border on child)
- [ ] **Type Korean** in the child VS Code — should work normally (no reparent)
- [ ] **Move container** — child follows
- [ ] **Resize container** — child resizes
- [ ] **VS Code internals** — sidebar toggle, command palette, terminal, Claude Code panel all work
- [ ] **Container close** — child gets its frame back and is usable as a normal window

## Known limits

- Without `window.titleBarStyle: "native"`, VS Code's own custom title bar still shows. Strip works on OS frame only.
- Closing the child VS Code via its own (now hidden) close button is impossible. User must close container, which restores the frame.
- No hotkeys to bring back temporarily-hidden frame.
- Single project, hardcoded path.

## Outcome handling

- ✅ Visual nesting + all functions work → **strongest "wrapping browser" approximation reachable without reparent**. Use this model for v0.1 Tauri.
- ⚠️ Visual nesting works but VS Code's custom title bar still shows → nice but imperfect. Fold-back to B (snap-to-region without container frame) is also acceptable.
- ❌ Style strip rejected by VS Code (it re-applies its own style) or causes new breakage → fall back to B or revisit code-server option.
