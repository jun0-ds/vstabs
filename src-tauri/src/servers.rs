// code-server lifecycle — spawn one backend per project.
//
// Three transport models:
//
//   "local" — spawn code-server.exe (or ~/.local/bin/code-server) directly on
//   the host where vstabs runs. Bound to 127.0.0.1:{port}.
//
//   "wsl"   — on Windows host, invoke `wsl -d {distro} bash -c "..."` so the
//   server runs inside the WSL distro and binds 127.0.0.1:{port}. WSL2
//   auto-forwards localhost ports to Windows. On Linux/macOS host, the "wsl"
//   env falls back to "local" (we are already on the same kernel as the path).
//
//   "ssh"   — open an SSH connection to a remote host through the user's ssh
//   client config (Tailscale-resolved hostname assumed; see
//   ops-config/docs/architecture.md). Allocate a free local port, set up an
//   `-L {local}:127.0.0.1:{remote}` port forward, and run code-server on the
//   remote bound to 127.0.0.1:{remote}. The ssh client's lifetime owns both
//   the tunnel and the remote code-server (no nohup/detach), so killing the
//   ssh child cleans up everything.
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
    /// For local/wsl this equals project.port. For ssh this is the local end
    /// of the SSH `-L` forward (allocated dynamically).
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
        "wsl" => {
            let child = spawn_wsl_or_local(project)?;
            Ok(ServerHandle {
                port: project.port,
                child,
            })
        }
        "local" => {
            let child = spawn_local(project.port, &project.folder)?;
            Ok(ServerHandle {
                port: project.port,
                child,
            })
        }
        "ssh" => spawn_ssh(project),
        other => Err(SpawnError::UnsupportedEnv(other.into())),
    }
}

// On Windows: a "wsl" project means cross the boundary via `wsl -d {distro}`.
// On Linux/macOS: we *are* the native host; treat "wsl" the same as "local"
// (the build is being run inside WSL or on Linux, so the path is reachable
// directly). This makes Linux dev builds usable for UI/lifecycle verification.
fn spawn_wsl_or_local(project: &Project) -> std::io::Result<Child> {
    #[cfg(windows)]
    {
        let distro = project.wsl_distro.as_deref().unwrap_or("Ubuntu");
        return spawn_wsl(distro, project.port, &project.folder);
    }
    #[cfg(not(windows))]
    {
        spawn_local(project.port, &project.folder)
    }
}

#[cfg(windows)]
fn spawn_wsl(distro: &str, port: u16, folder: &str) -> std::io::Result<Child> {
    let inner = format!(
        "env -u VSCODE_IPC_HOOK_CLI -u VSCODE_IPC_HOOK -u VSCODE_PID -u VSCODE_CWD \
         ~/.local/bin/code-server --bind-addr 127.0.0.1:{} --auth none {}",
        port,
        shell_quote(folder)
    );
    let mut cmd = Command::new("wsl");
    cmd.args(["-d", distro, "bash", "-c", &inner])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    hide_console_on_windows(&mut cmd);
    cmd.spawn()
}

fn spawn_local(port: u16, folder: &str) -> std::io::Result<Child> {
    let bin = which_code_server();
    let mut cmd = Command::new(&bin);
    cmd.args([
        "--bind-addr",
        &format!("127.0.0.1:{}", port),
        "--auth",
        "none",
        folder,
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
    let remote_port = project.port;
    let folder = &project.folder;

    // The remote command runs inside a bash subshell that:
    //   1. Strips VS Code IPC env vars (else code-server hijacks an existing instance)
    //   2. Installs a TERM/HUP trap that kills the whole process group on signal
    //   3. Backgrounds code-server, captures its PID, then `wait`s
    //
    // Combined with `ssh -tt` below, this makes ssh client death cleanly
    // SIGHUP the remote process tree — verified end-to-end against oracle
    // and gpu-host (2026-05-01 spike).
    let inner = format!(
        "exec env -u VSCODE_IPC_HOOK_CLI -u VSCODE_IPC_HOOK -u VSCODE_PID -u VSCODE_CWD \
         bash -c 'trap \"kill -TERM 0\" SIGTERM SIGHUP SIGINT; \
                  ~/.local/bin/code-server --bind-addr 127.0.0.1:{} --auth none {} & \
                  CSPID=$!; wait $CSPID'",
        remote_port,
        shell_quote(folder)
    );

    let mut cmd = Command::new("ssh");
    cmd.args([
        // Local port forward: bind 127.0.0.1:{local} on this machine, forward
        // to 127.0.0.1:{remote} on the SSH host (where code-server will bind).
        "-L",
        &format!("127.0.0.1:{}:127.0.0.1:{}", local_port, remote_port),
        // -tt: force TTY allocation even though our stdin is /dev/null. Required
        // for SIGHUP propagation to the remote process when ssh client dies.
        // Without this, the remote code-server keeps running as an orphan
        // (verified during the v0.2 D spike).
        "-tt",
        // Keep the connection alive; client-side detection of dead remotes.
        "-o",
        "ServerAliveInterval=30",
        "-o",
        "ServerAliveCountMax=3",
        // Don't ask about host keys for first connection — assume tailnet trust
        // (the user is already on a tailnet to reach this host alias at all).
        "-o",
        "StrictHostKeyChecking=accept-new",
        // The host alias resolves through the user's ssh config (Tailscale).
        host,
        &inner,
    ])
    .stdin(Stdio::null())
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
    // Bind to port 0 → OS picks a free port. Drop immediately so we can hand
    // the port to ssh -L. There's a tiny race window if something else binds
    // in between, but it's vanishingly small in practice.
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}

fn hide_console_on_windows(_cmd: &mut Command) {
    #[cfg(windows)]
    {
        // CREATE_NO_WINDOW = 0x08000000 — prevent flashing console on spawn.
        // tokio::process::Command provides creation_flags directly under cfg(windows).
        _cmd.creation_flags(0x08000000);
    }
}

fn shell_quote(s: &str) -> String {
    // Minimal POSIX single-quote escaping.
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
