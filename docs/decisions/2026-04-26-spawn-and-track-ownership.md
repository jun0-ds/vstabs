# Decision: vstabs only manages VS Code instances it spawned

- **Date:** 2026-04-26
- **Status:** Accepted
- **Related:** [`2026-04-26-reparent-rejected.md`](2026-04-26-reparent-rejected.md), [`../design.md`](../design.md) JTBD #2

## Context

The reparent spike opportunistically grabbed the *first* visible VS Code window matching a title pattern. Side effects observed:

- The user's actively-edited project-main window was reparented and broken
- Any "external" VS Code (started by the user outside vstabs) became a candidate

Even after rejecting reparent itself, this enumeration model is wrong. vstabs treats projects as first-class objects (design.md JTBD #2), but a "currently-open VS Code grep" model treats *windows* as the unit. These are not the same. A user's ad-hoc `code .` window is not a "project" in the vstabs sense — it's an unmanaged session.

## Decision

vstabs **only manages VS Code instances that vstabs itself spawned**.

- Each registry entry has a launch command. vstabs runs that command, captures the resulting OS process(es), and tracks the resulting window(s) by hwnd.
- External VS Code windows (started by the user without vstabs' knowledge) are never enumerated, never grabbed, never displayed in the tab bar. They are outside vstabs' object model entirely.
- "Import an existing VS Code window into vstabs" is **not** a feature. It's permanently out of scope (or, if revisited in v0.3+, requires an explicit user gesture and a separate code path).

## Why hwnd-diff over PID-tree

Both `cmd /c code <path>` and the direct `Code.exe` launcher fork a child process tree before the actual editor window appears, so the PID returned by `Command::spawn` is rarely the PID owning the final hwnd. Two viable identification strategies:

1. **PID tree walk** — enumerate descendants of the spawned PID, match hwnd via `GetWindowThreadProcessId`. Robust but requires `Process32First/Next` plumbing.
2. **Hwnd diff** — snapshot the set of VS Code hwnds before spawn, snapshot again after, take the difference. Simple, no process-tree code. Race condition only if two vstabs spawns interleave (acceptable in v0.1; v0.2 can serialize).

v0.1 uses **hwnd diff**. v0.2+ may upgrade to PID tree if races appear in practice.

## Why not `--user-data-dir` per project (in v0.1)

Trade-off:

| Pros (per-project user-data-dir) | Cons |
|---|---|
| Bulletproof identification (vstabs writes a sentinel file under the dir) | N× memory (each instance loads its own extensions) |
| Settings/extensions isolated per project | First launch is 5–10s (cold extension cache) |
| | Auth state (Claude Code, GitHub Copilot, etc.) duplicated and re-prompted N times |

For 4–6 concurrent projects on the user's main laptop, hwnd-diff identification is sufficient and the auth-duplication cost of `--user-data-dir` is the dominant factor against it.

`--user-data-dir` becomes an opt-in **per-project flag** in v0.2+ for projects the user explicitly wants isolated.

## Consequences

- Title matching is demoted from "primary identifier" to "diagnostic / display only"
- Same-folder-name collisions (design.md "Risks" table) become non-issues
- The current AHK prototype's enumeration model is wrong by this decision and would need rewrite if v0.0 were extended (it won't be — v0.0 is shelved in favor of moving to Tauri)
- The next spike (`spike/sibling-slave/`) implements this model: container spawns the VS Code itself, no external grab
- Lifecycle on container exit: child VS Code is **left alive** (no reparent means no lifetime coupling). User closes VS Code separately if they want.

## Action items

- [x] Record this decision
- [x] Build sibling-slave spike using spawn-and-track model
- [ ] After sibling-slave spike result, fold both ADRs back into `design.md` (rewrite Architecture and Risks sections)
