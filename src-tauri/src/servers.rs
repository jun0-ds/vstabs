// code-server lifecycle — spawn one backend per project tab.
//
// v0.2 A redesign: each spawn uses a per-tab `--user-data-dir` so vscode
// remembers its own last-opened workspace, sidebar layout, and per-tab
// extensions. No `folder` argument is passed — the user opens the folder
// from inside vscode (File → Open Folder), and code-server restores it on
// the next spawn.
//
// Three transport models:
//
//   "local" — spawn code-server.exe (or ~/.local/bin/code-server) directly
//   on the host where vstabs runs. Bound to 127.0.0.1:{auto-port}.
//
//   "wsl"   — on Windows host, invoke `wsl -d {distro} bash -c "..."` so the
//   server runs inside the WSL distro and binds 127.0.0.1:{auto-port}. WSL2
//   auto-forwards localhost ports to Windows. On Linux/macOS host the "wsl"
//   env falls back to "local".
//
//   "ssh"   — open an SSH connection to a remote host through the user's ssh
//   client config (Tailscale-resolved alias assumed). Allocate a free local
//   port, set up `-L {local}:127.0.0.1:{remote}` port forward, run code-server
//   on the remote bound to 127.0.0.1:{remote}. The ssh client's lifetime
//   owns both the tunnel and the remote process (`-tt` + bash trap →
//   SIGHUP propagates on client kill, no orphans).
//
// VS Code's IPC env vars (VSCODE_IPC_HOOK_CLI etc.) hijack code-server into
// "open in existing instance" mode, so we strip them before spawn on every
// transport.

use crate::Project;
use std::process::Stdio;
use thiserror::Error;
use tokio::process::{Child, Command};

#[derive(Debug, Error)]
pub enum SpawnError {
    #[error("unsupported env: {0}")]
    UnsupportedEnv(String),
    #[error("missing wsl_distro for wsl project")]
    MissingDistro,
    #[error("missing ssh_host for ssh project")]
    MissingSshHost,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct ServerHandle {
    /// Port the WebView should connect to on 127.0.0.1.
    /// For local/wsl this is the auto-allocated free port.
    /// For ssh this is the local end of the SSH `-L` forward.
    pub port: u16,
    pub child: Child,
}

impl ServerHandle {
    pub fn kill(&mut self) -> std::io::Result<()> {
        self.child.start_kill()
    }
}

pub fn spawn_for(project: &Project) -> Result<ServerHandle, SpawnError> {
    match project.env.as_str() {
        "wsl" => spawn_wsl_or_local(project),
        "local" => {
            let port = allocate_free_local_port()?;
            let child = spawn_local(port, &project.id)?;
            Ok(ServerHandle { port, child })
        }
        "ssh" => spawn_ssh(project),
        other => Err(SpawnError::UnsupportedEnv(other.into())),
    }
}

fn spawn_wsl_or_local(project: &Project) -> Result<ServerHandle, SpawnError> {
    #[cfg(windows)]
    {
        let distro = project
            .wsl_distro
            .as_deref()
            .ok_or(SpawnError::MissingDistro)?;
        let port = allocate_free_local_port()?;
        let child = spawn_wsl(distro, port, &project.id)?;
        Ok(ServerHandle { port, child })
    }
    #[cfg(not(windows))]
    {
        let _ = project; // distro unused on non-Windows
        let port = allocate_free_local_port()?;
        let child = spawn_local(port, &project.id)?;
        Ok(ServerHandle { port, child })
    }
}

#[cfg(windows)]
fn spawn_wsl(distro: &str, port: u16, tab_id: &str) -> std::io::Result<Child> {
    // Per-tab user-data-dir inside the WSL distro keeps each tab's vscode
    // state (last folder, sidebar, extensions) isolated.
    let user_data = format!("~/.local/share/vstabs/backends/{}", tab_id);
    let inner = format!(
        "mkdir -p {udd_q} && \
         env -u VSCODE_IPC_HOOK_CLI -u VSCODE_IPC_HOOK -u VSCODE_PID -u VSCODE_CWD \
         ~/.local/bin/code-server --bind-addr 127.0.0.1:{port} --auth none \
         --user-data-dir {udd_q}",
        udd_q = shell_quote(&user_data),
        port = port,
    );
    let mut cmd = Command::new("wsl");
    cmd.args(["-d", distro, "bash", "-c", &inner])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    hide_console_on_windows(&mut cmd);
    cmd.spawn()
}

fn spawn_local(port: u16, tab_id: &str) -> std::io::Result<Child> {
    let bin = which_code_server();
    let user_data = local_user_data_dir(tab_id);
    if let Some(parent) = std::path::Path::new(&user_data).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut cmd = Command::new(&bin);
    cmd.args([
        "--bind-addr",
        &format!("127.0.0.1:{}", port),
        "--auth",
        "none",
        "--user-data-dir",
        &user_data,
    ])
    .env_remove("VSCODE_IPC_HOOK_CLI")
    .env_remove("VSCODE_IPC_HOOK")
    .env_remove("VSCODE_PID")
    .env_remove("VSCODE_CWD")
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::null());
    hide_console_on_windows(&mut cmd);
    cmd.spawn()
}

fn spawn_ssh(project: &Project) -> Result<ServerHandle, SpawnError> {
    let host = project
        .ssh_host
        .as_deref()
        .ok_or(SpawnError::MissingSshHost)?;
    let local_port = allocate_free_local_port()?;
    let remote_port = derive_remote_port(&project.id);
    let user_data = format!("~/.local/share/vstabs/backends/{}", project.id);

    // Remote command:
    //   1. mkdir -p user-data-dir
    //   2. Strip VS Code IPC env (else code-server hijacks an existing instance)
    //   3. Trap TERM/HUP/INT to kill the whole process group on signal
    //   4. Background code-server, capture pid, wait
    let inner = format!(
        "exec env -u VSCODE_IPC_HOOK_CLI -u VSCODE_IPC_HOOK -u VSCODE_PID -u VSCODE_CWD \
         bash -c 'mkdir -p {udd_q}; \
                  trap \"kill -TERM 0\" SIGTERM SIGHUP SIGINT; \
                  ~/.local/bin/code-server --bind-addr 127.0.0.1:{rp} --auth none \
                    --user-data-dir {udd_q} & \
                  CSPID=$!; wait $CSPID'",
        udd_q = shell_quote(&user_data),
        rp = remote_port,
    );

    let mut cmd = Command::new("ssh");
    cmd.args([
        "-L",
        &format!("127.0.0.1:{}:127.0.0.1:{}", local_port, remote_port),
        "-tt",
        "-o",
        "ServerAliveInterval=30",
        "-o",
        "ServerAliveCountMax=3",
        "-o",
        "StrictHostKeyChecking=accept-new",
        host,
        &inner,
    ])
    // Windows OpenSSH treats Stdio::null() as immediate EOF and exits — keep
    // the pipe open for the child's lifetime.
    .stdin(Stdio::piped())
    .stdout(Stdio::null())
    .stderr(Stdio::null());
    hide_console_on_windows(&mut cmd);

    let child = cmd.spawn()?;
    Ok(ServerHandle {
        port: local_port,
        child,
    })
}

fn allocate_free_local_port() -> std::io::Result<u16> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}

/// Block (async) until 127.0.0.1:{port} accepts a TCP connection or `timeout`
/// elapses. Used right after spawn so the WebView doesn't load before the
/// code-server backend is actually listening (avoids ERR_CONNECTION_REFUSED).
pub async fn wait_port_open(port: u16, timeout: std::time::Duration) -> bool {
    let start = tokio::time::Instant::now();
    while start.elapsed() < timeout {
        let probe = tokio::time::timeout(
            std::time::Duration::from_millis(300),
            tokio::net::TcpStream::connect(("127.0.0.1", port)),
        )
        .await;
        if matches!(probe, Ok(Ok(_))) {
            return true;
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    false
}

/// Deterministic per-tab remote port for SSH backends. Re-spawns of the same
/// tab reuse the same remote port. Range 8090–9089. Collisions across distinct
/// tab ids on the same host are possible but rare.
fn derive_remote_port(tab_id: &str) -> u16 {
    let mut h: u32 = 0;
    for b in tab_id.bytes() {
        h = h.wrapping_mul(31).wrapping_add(b as u32);
    }
    8090u16.saturating_add((h % 1000) as u16)
}

fn local_user_data_dir(tab_id: &str) -> String {
    if let Some(base) = dirs::config_dir() {
        let p = base.join("vstabs").join("backends").join(tab_id);
        return p.to_string_lossy().into_owned();
    }
    format!("./vstabs-backends/{}", tab_id)
}

fn hide_console_on_windows(_cmd: &mut Command) {
    #[cfg(windows)]
    {
        // CREATE_NO_WINDOW = 0x08000000 — prevent flashing console on spawn.
        // tokio::process::Command exposes creation_flags directly under cfg(windows).
        _cmd.creation_flags(0x08000000);
    }
}

fn shell_quote(s: &str) -> String {
    let escaped = s.replace('\'', r"'\''");
    format!("'{}'", escaped)
}

fn which_code_server() -> String {
    if let Ok(out) = std::process::Command::new("which").arg("code-server").output() {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !s.is_empty() {
                return s;
            }
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        let p = std::path::Path::new(&home)
            .join(".local")
            .join("bin")
            .join("code-server");
        if p.exists() {
            return p.to_string_lossy().into_owned();
        }
    }
    "code-server".to_string()
}
