// code-server lifecycle — spawn one backend per project.
//
// On Windows host with a WSL project: invoke `wsl -d {distro} bash -c "..."` so the
// server runs inside the WSL distro and binds 127.0.0.1:{port}. WSL2 auto-forwards
// localhost ports to Windows, so the WebView can reach it as 127.0.0.1.
//
// On Windows host with a local project: spawn code-server.exe directly.
//
// VS Code's IPC env vars (VSCODE_IPC_HOOK_CLI etc.) hijack code-server into
// "open in existing instance" mode, so we strip them before spawn.

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
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct ServerHandle {
    pub port: u16,
    pub child: Child,
}

impl ServerHandle {
    pub fn kill(&mut self) -> std::io::Result<()> {
        self.child.start_kill()
    }
}

pub fn spawn_for(project: &Project) -> Result<ServerHandle, SpawnError> {
    let child = match project.env.as_str() {
        "wsl" => spawn_wsl_or_local(project)?,
        "local" => spawn_local(project.port, &project.folder)?,
        "ssh" => return Err(SpawnError::UnsupportedEnv("ssh (deferred to v0.2)".into())),
        other => return Err(SpawnError::UnsupportedEnv(other.into())),
    };
    Ok(ServerHandle {
        port: project.port,
        child,
    })
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
    #[cfg(windows)]
    {
        // CREATE_NO_WINDOW = 0x08000000 — prevent flashing console.
        std::os::windows::process::CommandExt::creation_flags(&mut cmd, 0x08000000);
    }
    cmd.spawn()
}

fn spawn_local(port: u16, folder: &str) -> std::io::Result<Child> {
    // Resolve code-server: PATH first, then ~/.local/bin/code-server (the
    // standalone install location used in the spike).
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
    #[cfg(windows)]
    {
        std::os::windows::process::CommandExt::creation_flags(&mut cmd, 0x08000000);
    }
    cmd.spawn()
}

fn shell_quote(s: &str) -> String {
    // Minimal POSIX single-quote escaping.
    let escaped = s.replace('\'', r"'\''");
    format!("'{}'", escaped)
}

fn which_code_server() -> String {
    // Tier 1: PATH lookup
    if let Ok(out) = std::process::Command::new("which").arg("code-server").output() {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !s.is_empty() {
                return s;
            }
        }
    }
    // Tier 2: standalone install location
    if let Some(home) = std::env::var_os("HOME") {
        let p = std::path::Path::new(&home).join(".local").join("bin").join("code-server");
        if p.exists() {
            return p.to_string_lossy().into_owned();
        }
    }
    // Tier 3: fall through to PATH lookup at exec time
    "code-server".to_string()
}
