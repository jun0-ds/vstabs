# Decision: Reject Win32 SetParent reparenting of VS Code

- **Date:** 2026-04-26
- **Status:** Accepted
- **Supersedes:** Reaffirms the reparent rejection in [`../design.md`](../design.md) (previously written as a hypothesis)
- **Spike:** [`../../spike/reparent/`](../../spike/reparent/)

## Context

The original [`design.md`](../design.md) rejected embedding/reparenting VS Code on the assumption that it would break IME, GPU rendering, and focus routing. That rejection was speculative. When the v0.0 AHK prototype shipped a sibling tab bar, the user reported the intended UX was actually a **container window with VS Code rendered inside it** — the exact path design.md had pre-rejected.

Before sinking ~1 week into a Tauri MVP based on either model, we ran a 30-minute Rust spike to convert the speculation into an empirical answer.

## Spike

`spike/reparent/` — winit 0.30 container window, enumerate top-level VS Code windows, `SetParent` the first match into the container after stripping its style to `WS_CHILD | WS_VISIBLE`. Restore on clean exit.

Outcome:

- ✅ `SetParent` succeeds; VS Code visually nests inside the container.
- ❌ **Keyboard input does not reach the embedded VS Code** (failed before IME could even be tested).
- ❌ **Ctrl+C / abnormal exit kills the embedded VS Code along with the container** — child windows share the parent's lifetime, and the detach path only runs on the clean close-window event.
- ❌ **The user's actively-used VS Code window was hijacked**. Enumeration picking the first match is fixable, but the deeper problem is that *any* VS Code instance, once reparented, is no longer usable as a normal editor — so even with perfect enumeration we'd be conscripting a window the user has open work in.

Latent problems we did not even reach: GPU/Chromium compositor assumptions, DPI scaling, multi-monitor moves, Electron's own window-state tracking fighting the style change.

## Decision

Reject `SetParent`-based reparenting permanently. Do not revisit unless VS Code itself ships a public embed surface (e.g. an OS-level container handle), which is not on Microsoft's roadmap.

## Why this happens (root cause, for future readers)

VS Code is Electron, which is Chromium. Chromium's renderer assumes its native HWND is a top-level window: it owns the input message loop, the IME composition window, the GPU swap chain, and the focus chain as a peer of the OS shell. `SetParent` mutates the window tree node but does not refactor any of those assumptions. Keyboard messages start going to the new parent's wndproc, the IME composition window anchors to the wrong client area, and lifetime becomes parent-bound.

This is structural to Electron, not specific to VS Code's build. JetBrains, Zed, and Cursor are different processes with different architectures — JetBrains' multi-project tab bar works because it's *one* process drawing many project views, not many OS windows being shoved into one container.

## Consequences

The "L1 project tab bar" goal stands; the implementation path narrows to options that do not require reparenting:

1. **Strong sibling** (recommended next step) — Tauri container window draws the tab bar (and possibly a frame/title region). VS Code windows remain top-level siblings, but their position/size are slaved to the container via `SetWinEventHook` (`EVENT_SYSTEM_MOVESIZEEND`, `EVENT_SYSTEM_MINIMIZESTART`, etc.). Visually approximates "VS Code lives inside the container"; technically VS Code is untouched, so IME / GPU / focus / lifetime all remain native.
   - Trade-off: alt-tab can momentarily separate the container from its child VS Code. Mitigated by foreground-event hooks that re-stack on focus.

2. **code-server embed** — Run code-server, embed the browser UI in a Tauri WebView. True embedding because VS Code is now web. Cost: lose Claude Code IDE integration (no PTY, no local fs), lose extensions that need native APIs, run a server.

3. **Give up the "one window" feel** — Ship the original sibling tab bar (current AHK prototype model) as v0.1 final. Tab bar lives at top of screen; VS Code windows are independent. Cleanest, but does not match the user's stated UX intent.

Option 1 is the next design target. Option 2 is the fallback if `SetWinEventHook`-based slaving turns out to feel too floaty in practice.

## Action items

- [x] Record this decision
- [ ] Update `design.md` — mark reparent rejection as *spike-validated* (link this ADR), rewrite v0.1 architecture around strong-sibling model
- [ ] New ADR when option 1 vs option 2 is decided after another spike (sibling-slaving feasibility — ~1h)
