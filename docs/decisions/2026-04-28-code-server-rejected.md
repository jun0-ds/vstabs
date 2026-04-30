# Decision: Reject code-server / web-VS-Code embedding

- **Date:** 2026-04-28
- **Status:** **Superseded** by [`2026-04-30-code-server-revived.md`](2026-04-30-code-server-revived.md)
- **Related:** [`2026-04-26-reparent-rejected.md`](2026-04-26-reparent-rejected.md), [`2026-04-26-spawn-and-track-ownership.md`](2026-04-26-spawn-and-track-ownership.md)
- **Spike artifacts:** [`../../spike/code-server-shots/`](../../spike/code-server-shots/)

> **Supersession note (2026-04-30)**: The IME blocker described below was a measurement artifact, not a structural limit. See revived ADR for the diagnosis correction. The "Claude Code integration works" finding remains valid; the "IME breaks" finding was wrong.

## Context

After reparent was rejected and sibling-slave (#2) showed two visually-separate frames, the user articulated the target UX as "VS Code를 wrapping하는 브라우저처럼 작동" — a single window where vstabs provides the chrome and VS Code is rendered inside.

The literal implementation is **code-server (web VS Code) embedded in a Tauri WebView**. design.md had pre-rejected this option assuming Claude Code IDE integration would break (PTY, local fs, MCP). That rejection was speculative — same epistemic flaw as the original reparent rejection. So we ran a verification spike to convert speculation into evidence.

## Spike

WSL host running `code-server 4.117.0` standalone, bound to `127.0.0.1:8080`, no auth. Headless Chrome (Playwright) drove the UI:

1. Installed `anthropic.claude-code` 2.1.121 (15.5M downloads, official, listed in marketplace)
2. Trust dialog accepted
3. Claude Code sidebar panel opened
4. Confirmed the panel **lists the user's actual local CLI sessions** (read from `~/.claude/projects/*.jsonl`) — direct evidence that file system access works inside code-server

Screenshots: [`spike/code-server-shots/01-initial.png` ... `06-claude-opened.png`](../../spike/code-server-shots/).

So the original "PTY/local fs/MCP will break" assumption was wrong. The path looked viable.

## Why it still gets rejected

User reported: **"한영키가 안먹어요"** (Korean/English IME toggle key does not work).

This is structural to web VS Code, not a code-server bug:

- Monaco editor inside a browser receives standard `KeyboardEvent`s only
- `한/영` (Hangul/English) is an **OS-level IME mode toggle**, not a browser-routable key event
- The browser sees the IME's *output* (composed characters) but cannot observe or trigger the mode switch itself
- Tauri WebView wrapping does not help — same Chromium, same constraint
- No application-side workaround exists. This is the same reason web IDEs across the board (vscode.dev, GitHub Codespaces in browser, Gitpod) are unusable for Korean developers without manual IME tray clicks per toggle

For a Korean-language user who switches IME modes constantly (chat, code comments, notes — all in this session), this is a per-keystroke friction multiplier. It violates JTBD #6 (cognitive load externalization) directly: vstabs would *add* cognitive load instead of removing it.

## Decision

Reject code-server / web-VS-Code embedding **permanently**, regardless of whether other dimensions (Claude Code integration, MCP, PTY) turn out to work.

## Why this matters as a separate ADR (not a footnote)

design.md's original rejection was on the wrong axis — it predicted "Claude Code integration breakage" and that turned out to be false. The *actual* blocker was IME, which design.md did not consider. Documenting this corrects the reasoning record so future revisits know:

- ✅ Claude Code integration in code-server **works** (proven by spike)
- ❌ Korean IME mode toggle **does not work** (structural to web)
- A future user without Korean IME requirements could legitimately revisit this option; this ADR is not a blanket "code-server is bad" claim

## Consequences

The "wrapping browser" UX target stands; the implementation path narrows further:

1. **Borderless sibling-slave** ([`spike/borderless/`](../../spike/borderless/)) — strip the OS frame from a sibling-slaved native VS Code window. Closest reachable approximation of "VS Code rendered inside container" without web embedding. Native IME preserved (no reparent, no web). Pending Windows-side spike result.
2. **Snap-to-region sibling** (model B) — give up the container frame entirely. Tab bar at top, active VS Code snapped into a fixed region below. Visually less integrated but closest to UX while preserving everything VS Code does natively.

The final choice between #1 and #2 depends on the borderless spike outcome.

## Action items

- [x] Record this decision
- [x] Stop and clean up the running code-server instance
- [ ] Run borderless spike (#3) on Windows host — pending user
- [ ] Based on borderless spike result, write final model decision ADR
- [ ] Fold all four ADRs (reparent, spawn-and-track, code-server, final) back into design.md and rewrite v0.1 architecture
