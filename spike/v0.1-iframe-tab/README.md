# vstabs v0.1 spike — iframe-based multi-project model (Phase 1)

> Validates the wrapping-browser model end-to-end with the lightest possible
> stack: 2 code-server backends + 1 HTML page with 2 iframes. No Tauri yet.
> If this passes, Phase 2 replaces iframes with Tauri WebView2 for production.

## What this spike answers

| Question | Why |
|---|---|
| Can two code-server instances coexist on different ports without conflict? | Foundation for N projects |
| Does iframe-based tab switching feel responsive (<100 ms)? | UX baseline |
| Does Korean IME work inside child iframes (not just top-level browser)? | The blocker we hit before |
| Does Claude Code panel work inside iframes for both tabs? | Largest extension dependency |
| Memory cost per concurrent code-server instance? | Sets the cap on practical N |

## What it does NOT yet prove

- Native multi-WebView2 in Tauri (Phase 2)
- code-server lifecycle management from Rust (lazy spawn, idle suspend, restart)
- Per-project lifecycle on container close
- Process supervision and port allocation

If Phase 1 passes, Phase 2 is mostly engineering — model risk is gone.

## Run

```bash
# WSL
bash start-code-servers.sh
```

This spawns two code-server instances (lazy-loads on first request):
- A: `http://127.0.0.1:8080` opening `~/sample-a`
- B: `http://127.0.0.1:8081` opening `~/sample-b`

(Edit the script if you want different folders.)

Then in **Chrome or Edge** on Windows (Firefox had layout glitches in earlier spike):

```
file:///\\wsl.localhost\<your-distro>\path\to\spike\v0.1-iframe-tab\index.html
```

Or copy `index.html` to Windows side and double-click.

## Test checklist

### Phase 1a — model passes (verified 2026-04-30)
- [x] Both tabs load (no port conflict, no auth prompt — `--auth none`)
- [x] Switch latency under 100 ms
- [x] Korean IME inside iframes
- [x] Claude Code panel inside iframes (both tabs)

### Phase 1b — identification + registry (this revision)
- [ ] **Browser tab title** — Chrome/Edge tab shows `🏠 project-main — vstabs` for tab A, `📑 vstabs — vstabs` for tab B (changes on switch)
- [ ] **Browser tab favicon** — emoji favicon changes per active project (visible in OS taskbar when browser is minimized)
- [ ] **Tab visual** — active tab has bottom border in env color, env tag pill colored per environment
- [ ] **Switch counter** — top-right shows `N projects • switch #X Yms`
- [ ] **Hotkey** — `Ctrl+Alt+1` activates tab A, `Ctrl+Alt+2` activates tab B (direct select)
- [ ] **`+` button** — clicking shows the "v0.1 add-project UI" alert
- [ ] **External registry** — `projects.js` controls everything; editing it + reload changes tabs
- [ ] **Empty state** — if `PROJECTS = []` in projects.js, page shows "No project selected"

### Phase 1c — operational (still in scope)
- [ ] File operations work in both tabs
- [ ] Sidebar / search / terminal isolated per tab
- [ ] Switch back restores state (open files, cursor, sidebar)
- [ ] Memory: ~150–300 MB per code-server

## Stop

```bash
bash stop-code-servers.sh
```

Logs at `/tmp/cs-spike/cs-{a,b}.log` for debugging.

## Editing the project list

Open `projects.js`, edit the `PROJECTS` array, reload the page. Each entry:

```js
{
  id: "project-main",     // unique slug, DOM-safe
  name: "project-main",    // display label
  icon: "🏠",                // emoji used in tab + favicon
  env: "wsl",               // "local" | "wsl" | "ssh" — colors the env pill
  port: 8080,               // localhost port where this project's code-server runs
  folder: "~/projects/main",
}
```

Environment colors (`ENV_COLORS` in `projects.js`):
- `local` — green
- `wsl` — cyan
- `ssh` — purple

To add a third project: edit `projects.js`, then add a third spawn line in `start-code-servers.sh` (port 8082, etc.), restart code-servers, reload the page.

## Known limits (spike-only)

- Project list edited by file (production has form-based add UI)
- No process supervision — if code-server crashes, manual restart via `start-code-servers.sh`
- No lazy spawn — all code-servers start at script run (production: spawn on first tab click)
- iframe ≠ Tauri WebView2 (different process model). Phase 2 confirms native.
- Browser tab is one OS window; production Tauri has full container window with vstabs as the OS-level identity.

## Result handling

If everything works:
- Move to **Phase 2** (Tauri shell + multi-WebView2 + Rust lifecycle manager)
- Or skip directly to **v0.1 implementation** if Phase 2 risk seems already covered

If Korean IME breaks inside iframes (unlikely, since it works in top-level Chrome):
- Diagnose iframe sandbox attributes / cross-origin policy
- Consider `<webview>` (Electron) or Tauri WebView2 directly

If Claude Code panel breaks in iframe but worked in top-level:
- Same iframe sandbox issue — may need `allow-scripts allow-same-origin allow-forms` etc.

If two code-server instances conflict (rare — they share `~/.local/share/code-server` user-data-dir by default):
- Add `--user-data-dir` per instance (`/tmp/cs-spike/data-a`, `data-b`)
