# Decision: Revive code-server embedding — vstabs = Tauri container + multi-WebView

- **Date:** 2026-04-30
- **Status:** Accepted
- **Supersedes:** [`2026-04-28-code-server-rejected.md`](2026-04-28-code-server-rejected.md)
- **Related:** [`2026-04-26-reparent-rejected.md`](2026-04-26-reparent-rejected.md), [`2026-04-26-spawn-and-track-ownership.md`](2026-04-26-spawn-and-track-ownership.md)
- **Journal:** [`../journal/2026-04-30-prior-art-deep-dive.md`](../journal/2026-04-30-prior-art-deep-dive.md)

## Context

`code-server-rejected` (2026-04-28) was based on a single observation: "한영키가 안 먹어요" — Korean IME toggle did not work in the spike's web VS Code. The reasoning leap was *"browser IME = OS-level structural limit, no workaround possible."*

After the prior-art deep-dive (2026-04-30) showed that **every viable v0.1 path was blocked** (reparent → IME/lifetime; sibling-slave → not "wrapped"; borderless → custom title bar remained; OS-level Sets → cancelled by Microsoft itself; Cursor/Zed → same Windows OS limit), the user re-questioned the IME conclusion and asked for cross-browser verification.

## What changed

The user tested the same code-server instance directly in **Firefox and Chrome** on the Windows host (not via Playwright headless). Both browsers:

- Korean IME toggle (한/영 key) **works**
- Hangul composition **works** ("테스트 테스트" typed cleanly)
- Claude Code chat panel: Korean message in, Korean response out, both clean
- Firefox had **layout glitches** in some chat UI elements; Chrome was visually clean
- File operations, sidebar, command palette, extensions — all native VS Code Desktop equivalents

Earlier "IME broken" observation was a **Playwright headless `chromium-headless-shell` artifact**: automated keyboard events from the test driver don't trigger the OS IME mode toggle the way real key presses do. I conflated "IME doesn't work in this test environment" with "IME doesn't work for users."

## Decision

Adopt **vstabs = Tauri container + multi-WebView2 (Chromium-based) + code-server backend** as the v0.1 model.

WebView2 is Chromium under the hood — same engine the user verified Korean IME works on. Firefox is excluded as a wrapper option (layout glitches), but its IME success was the proof that the limit was Chromium-headless-shell-specific, not browser-IME-structural.

## Architecture

```
┌─────────────────────────────────────────────────────────┐  vstabs (Tauri app, ~5 MB)
│ 🏠 project-main  📊 lib-x  🖥 gpu-dev  +             │  L1 — tab bar (Tauri UI)
├─────────────────────────────────────────────────────────┤
│                                                         │
│         WebView2 (Chromium, system Edge)                │  L2 — active project
│         loads http://127.0.0.1:{port-N}                 │     (Monaco / sidebar /
│         backed by per-project code-server instance      │      Claude Code panel)
│                                                         │
└─────────────────────────────────────────────────────────┘
```

- vstabs is one OS window → taskbar shows one icon, alt-tab shows one entry → JTBD #6 (cognitive load) satisfied at OS level
- Each tab corresponds to one code-server backend instance (per-project isolation, JTBD #3) and one WebView in the container (per-project sidebar/search/LSP scope, JTBD #2)
- Active tab's WebView is shown; others are hidden but kept alive (warm switch, JTBD #1 entry cost ≈ 0)
- Native browser IME on the host OS reaches into WebView2 normally → Korean / Japanese / Chinese input native
- VS Code extensions (including Anthropic's Claude Code) install into each code-server instance via the marketplace, with `~/.claude/projects/*.jsonl` accessed natively (verified)

## Why this beats every prior candidate

| Candidate | Visual "wrapping" | OS-level grouping | Korean IME | Cost |
|---|---|---|---|---|
| AHK v0.0 sibling | ❌ | ❌ | ✅ | low |
| Reparent | partial | ✅ | ❌ (Chromium) | high, broken |
| Sibling-slave | ❌ two frames | ✅ | ✅ | medium, broken UX |
| Borderless | partial | ✅ | ✅ | medium, custom title bar still visible |
| Snap-to-region | ❌ no frame | ❌ | ✅ | low, gives up frame |
| **Tauri + code-server multi-WebView (this)** | **✅** | **✅** | **✅** | **medium, all JTBDs hit** |

## Trade-offs (v0.1 detail to resolve)

1. **Memory** — N code-server processes vs 1 process / multi-folder. Per-project isolation (JTBD #3) requires N processes; multi-folder breaks it (search/LSP shared). Mitigation: lazy spawn (code-server starts only on first tab click), idle suspend after T minutes.
2. **Auth** — Each code-server instance has its own auth state. Anthropic Claude Code in particular needs Anthropic OAuth per instance (or shared via local credential store).
3. **Remote tabs (WSL/SSH)** — code-server can target remote hosts via the Remote-SSH extension or run *on* the remote host. Two sub-models possible; v0.1 picks one and ships.
4. **Process supervision** — vstabs needs to manage code-server lifecycle (spawn / health / restart on crash) and pick free ports per project.
5. **WebView2 deployment** — WebView2 runtime is bundled with Windows 10 1809+ via Edge. Tauri's bundler handles fallback installer. No extra user step on modern Windows.

## Risks & mitigations

| Risk | Mitigation |
|---|---|
| Chromium IME bug regression in a future Edge update | Pin WebView2 version; smoke test Korean input on each release |
| code-server lags upstream VS Code by weeks | Acceptable — VS Code's release cadence is monthly, lag is small for 1-person tool |
| code-server doesn't support Microsoft proprietary extensions (e.g., Pylance) | Use open-source alternatives (basedpyright instead of Pylance); document per-extension status |
| Per-project code-server memory cost (~150–300 MB each) | Lazy spawn + idle suspend; on the user's main laptop (16 GB), 4–6 concurrent projects is comfortable |
| User's existing Claude Code CLI sessions need to interop | Verified: extension reads `~/.claude/projects/*.jsonl` natively. Same data path. |

## Action items

- [x] Mark `code-server-rejected` as Superseded
- [x] Update journal with the cross-browser verification result and the model decision
- [x] Rewrite `design.md` end-to-end with the new architecture (this ADR's diagram becomes the canonical design)
- [ ] v0.1 last spike — Tauri shell with two WebView2 instances pointing at two local code-server ports, prove tab switching and process lifecycle. ~1–2 days.
- [ ] After spike passes, v0.1 implementation: tab bar UI, registry, lazy spawn, hotkeys.
