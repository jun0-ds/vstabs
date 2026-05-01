// Project registry — step 1 returns a hardcoded sample list.
//
// The four entries below are *examples*: one local Windows folder, one WSL
// project, one Oracle-style cloud SSH host, one generic remote SSH host.
// Edit this file (or, after v0.2, edit the JSON registry at
// %APPDATA%\vstabs\projects.json) to match your own environment.
//
// SSH entries assume the user's ssh config (typically Tailscale-resolved
// aliases for personal-use deployments) already contains the host names
// listed here. See `docs/design.md` "SSH backend security model".

use crate::Project;

pub fn default_projects() -> Vec<Project> {
    vec![
        Project {
            id: "sample-local".into(),
            name: "sample-local".into(),
            icon: "🏠".into(),
            env: "local".into(),
            port: 8080,
            folder: "C:\\Projects\\sample-local".into(),
            wsl_distro: None,
            ssh_host: None,
        },
        Project {
            id: "sample-wsl".into(),
            name: "sample-wsl".into(),
            icon: "🐧".into(),
            env: "wsl".into(),
            port: 8081,
            folder: "/home/your-user/sample-wsl".into(),
            wsl_distro: Some("Ubuntu".into()),
            ssh_host: None,
        },
        // SSH entries — the host alias must be resolvable through your ssh
        // config. The remote `port` is what code-server will bind on the
        // remote host; vstabs allocates a fresh local port for the SSH
        // tunnel dynamically and points the WebView at that.
        Project {
            id: "sample-cloud".into(),
            name: "sample-cloud".into(),
            icon: "☁️".into(),
            env: "ssh".into(),
            port: 8090,
            folder: "/home/ubuntu".into(),
            wsl_distro: None,
            ssh_host: Some("my-cloud-host".into()),
        },
        Project {
            id: "sample-remote".into(),
            name: "sample-remote".into(),
            icon: "🖥".into(),
            env: "ssh".into(),
            port: 8091,
            folder: "/home/your-user".into(),
            wsl_distro: None,
            ssh_host: Some("my-remote-host".into()),
        },
    ]
}
