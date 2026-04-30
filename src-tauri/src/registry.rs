// Project registry — step 1 returns a hardcoded list.
//
// v0.2 will read %APPDATA%\vstabs\projects.json (Windows) or ~/.config/vstabs/projects.json
// and write through it via add/remove project commands invoked from the UI.

use crate::Project;

pub fn default_projects() -> Vec<Project> {
    vec![
        Project {
            id: "project-main".into(),
            name: "project-main".into(),
            icon: "🏠".into(),
            env: "wsl".into(),
            port: 8080,
            folder: "~/projects/main".into(),
            wsl_distro: Some("Ubuntu".into()),
            ssh_host: None,
        },
        Project {
            id: "vstabs".into(),
            name: "vstabs".into(),
            icon: "📑".into(),
            env: "wsl".into(),
            port: 8081,
            folder: "~/projects/vstabs".into(),
            wsl_distro: Some("Ubuntu".into()),
            ssh_host: None,
        },
    ]
}
