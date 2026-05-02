// vstabs — Tauri shell + per-project code-server lifecycle.
//
// Step 1 (✅ shipped): container window, list_projects, spawn/stop/status,
//   cleanup on close. Lazy spawn on first tab click.
// v0.2 D (✅ shipped): SSH backend with `-tt`+stdin-piped+SIGHUP-trap so the
//   ssh client's lifetime owns both the tunnel and the remote code-server.
// v0.2 A (this revision): JSON-backed registry at %APPDATA%\vstabs\projects.json
//   plus add/update/remove/reorder commands and host-introspection helpers
//   (WSL distro list, SSH alias list) so the UI can build a real Add Project
//   form. Hardcoded `sample_projects()` is kept for tests only.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{Manager, State};

mod registry;
mod servers;
mod sysinfo;

use servers::{spawn_for, wait_port_open, ServerHandle};

#[derive(Default)]
pub struct AppState {
    pub servers: Mutex<HashMap<String, ServerHandle>>,
}

// v0.2 A redesign:
// `folder` and `port` are no longer part of the user-facing schema.
//   - folder: the user opens it from inside VS Code (File → Open Folder).
//     code-server remembers last-opened workspace per --user-data-dir, so the
//     next spawn restores it. vstabs is the launcher, vscode is the editor.
//   - port: vstabs auto-allocates a free local port at spawn time. SSH remote
//     port is derived deterministically from the tab id so re-spawns reuse
//     the same remote port.
// Older v2 registry files that still contain `folder`/`port` fields are
// silently accepted (serde ignores unknown fields by default) and rewritten
// in v3 shape on the next save.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub env: String,
    #[serde(default)]
    pub wsl_distro: Option<String>,
    #[serde(default)]
    pub ssh_host: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct ServerStatus {
    pub project_id: String,
    pub port: u16,
    pub running: bool,
}

// ---- Project registry commands -------------------------------------------

#[tauri::command]
fn list_projects() -> Result<Vec<Project>, String> {
    registry::load_or_create()
        .map(|f| f.projects)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn add_project(project: Project) -> Result<(), String> {
    let mut file = registry::load_or_create().map_err(|e| e.to_string())?;
    registry::add(&mut file, project).map_err(|e| e.to_string())?;
    registry::save(&file).map_err(|e| e.to_string())
}

#[tauri::command]
fn update_project(project: Project) -> Result<(), String> {
    let mut file = registry::load_or_create().map_err(|e| e.to_string())?;
    registry::update(&mut file, project).map_err(|e| e.to_string())?;
    registry::save(&file).map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_project(state: State<'_, AppState>, project_id: String) -> Result<(), String> {
    // Kill backend if it's running so we don't leak.
    {
        let mut servers = state.servers.lock().unwrap();
        if let Some(mut h) = servers.remove(&project_id) {
            let _ = h.kill();
        }
    }
    let mut file = registry::load_or_create().map_err(|e| e.to_string())?;
    registry::remove(&mut file, &project_id).map_err(|e| e.to_string())?;
    registry::save(&file).map_err(|e| e.to_string())
}

#[tauri::command]
fn reorder_projects(ordered_ids: Vec<String>) -> Result<(), String> {
    let mut file = registry::load_or_create().map_err(|e| e.to_string())?;
    registry::reorder(&mut file, ordered_ids).map_err(|e| e.to_string())?;
    registry::save(&file).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_wsl_distros() -> Vec<String> {
    sysinfo::list_wsl_distros()
}

#[tauri::command]
fn list_ssh_aliases() -> Vec<String> {
    sysinfo::list_ssh_aliases()
}

// ---- code-server lifecycle commands --------------------------------------

#[tauri::command]
async fn spawn_server(
    state: State<'_, AppState>,
    project: Project,
) -> Result<ServerStatus, String> {
    {
        let servers = state.servers.lock().unwrap();
        if let Some(h) = servers.get(&project.id) {
            return Ok(ServerStatus {
                project_id: project.id,
                port: h.port,
                running: true,
            });
        }
    }
    let handle = spawn_for(&project).map_err(|e| e.to_string())?;
    let effective_port = handle.port;
    {
        let mut servers = state.servers.lock().unwrap();
        servers.insert(project.id.clone(), handle);
    }
    // Wait until the backend actually accepts connections — WSL/SSH cold start
    // takes 5–15s, and returning early causes the WebView to load before the
    // server is listening (ERR_CONNECTION_REFUSED).
    let ready = wait_port_open(effective_port, std::time::Duration::from_secs(20)).await;
    if !ready {
        return Err(format!(
            "backend port {} did not become reachable within 20s",
            effective_port
        ));
    }
    Ok(ServerStatus {
        project_id: project.id,
        port: effective_port,
        running: true,
    })
}

#[tauri::command]
async fn stop_server(state: State<'_, AppState>, project_id: String) -> Result<(), String> {
    let mut servers = state.servers.lock().unwrap();
    if let Some(mut handle) = servers.remove(&project_id) {
        handle.kill().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn server_status(state: State<'_, AppState>) -> Vec<ServerStatus> {
    let servers = state.servers.lock().unwrap();
    servers
        .iter()
        .map(|(id, h)| ServerStatus {
            project_id: id.clone(),
            port: h.port,
            running: true,
        })
        .collect()
}

pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            // registry CRUD
            list_projects,
            add_project,
            update_project,
            remove_project,
            reorder_projects,
            // host introspection
            list_wsl_distros,
            list_ssh_aliases,
            // code-server lifecycle
            spawn_server,
            stop_server,
            server_status,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                if let Some(state) = window.app_handle().try_state::<AppState>() {
                    if let Ok(mut servers) = state.servers.lock() {
                        for (_, mut h) in servers.drain() {
                            let _ = h.kill();
                        }
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
