# vstabs — build & run (v0.1 step 1)

This is the first production-grade build of vstabs after the spike journey
(see [`docs/decisions/`](docs/decisions/) and [`docs/journal/`](docs/journal/)).

Step 1 covers: Tauri shell + iframe-based tab model + Rust-managed
`code-server` lifecycle. Native multi-WebView2, lazy spawn, and the JSON
registry come in step 2 / v0.2.

## Prerequisites (Windows host)

1. **Rust toolchain** — install via [rustup](https://rustup.rs/), `stable` channel
2. **Tauri CLI 2.x** — `cargo install tauri-cli --version "^2.0"`
3. **Microsoft C++ Build Tools** (already present if you have Visual Studio Code)
4. **WebView2 Runtime** — bundled with Windows 10 1809+ via Edge
5. **WSL distro with code-server installed** — `Ubuntu` is the default in `src-tauri/src/registry.rs`

If `code-server` is missing in WSL:

```bash
curl -fsSL https://code-server.dev/install.sh \
  | sh -s -- --method standalone --prefix $HOME/.local
```

## Run in dev mode

```powershell
cd path\to\vstabs
cargo tauri dev
```

The first build downloads ~250 MB of dependencies (windows-rs, Tauri framework,
WebView2 bindings) and takes 5–10 minutes. Subsequent builds are incremental
(seconds).

## Build a release binary

```powershell
cargo tauri build
```

Output: `src-tauri/target/release/bundle/` — `.msi` and `.exe` installers.

## What v0.1 step 1 does

- Opens a single OS window titled "vstabs"
- Loads two tabs from `registry::default_projects()`: project-main (WSL) and vstabs (WSL)
- Click a tab → backend spawns `code-server` for that project on its assigned port
- Iframe loads `http://127.0.0.1:{port}` and shows the editor
- Switching tabs hides the previous iframe and shows the next (warm switch)
- Closing the vstabs window kills all spawned `code-server` child processes

## Edit the project list (step 1)

Open `src-tauri/src/registry.rs`, modify `default_projects()`, rebuild.
Step 2 will read from `%APPDATA%\vstabs\projects.json` instead.

## Quick spike-style preview without Tauri

The `ui/` directory works standalone if served over HTTP — useful for fast UI
iteration without rebuilding the Rust shell:

```bash
# WSL
cd ~/project-main/vstabs/ui
python3 -m http.server 9000 --bind 127.0.0.1
# then open http://127.0.0.1:9000/ in Chrome/Edge,
# after starting code-server backends manually:
bash ~/project-main/vstabs/spike/v0.1-iframe-tab/start-code-servers.sh
```

The fallback path in `app.js` reads from `ui/projects.js` when `window.__TAURI__`
is absent.

## Linux build is for dev iteration only

A Linux build (`cargo tauri build` inside WSL) compiles fine and uses
`webkit2gtk` as its WebView. UI / lifecycle / `code-server` spawn / cleanup
all behave correctly there, but **deeply nested Chromium-only UIs (notably
the Claude Code chat panel inside a code-server iframe) render blank under
webkit2gtk**. This is a Linux-WebView limitation, not a vstabs bug — the
production target is Windows WebView2 (Edge / Chromium), where the panel
works (verified in spike Phase 1b on Chrome native).

Use Linux builds for fast iteration on the Rust backend / tab UI / lifecycle;
use Windows builds for end-to-end verification and shipping.

## Troubleshooting

- **"window blank, no tabs"** — open devtools (`Ctrl+Shift+I` in Tauri dev mode), check console for `list_projects` error
- **"code-server fails to spawn"** — check WSL distro name in `registry.rs` matches `wsl -l -v` output
- **"port already in use"** — another `code-server` is bound; `pkill -f code-server` in WSL or change ports in `registry.rs`
- **"VS Code IPC redirect"** — happens when Tauri inherits `VSCODE_IPC_HOOK_CLI` from the launching shell. The Rust spawn strips these, but if you spawn manually for testing, prefix with `env -u VSCODE_IPC_HOOK_CLI -u VSCODE_IPC_HOOK ...`
