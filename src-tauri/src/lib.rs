// vstabs v0.1 — Tauri shell + per-project code-server lifecycle.
//
// Step 1 scope (this commit):
//   - Container window with embedded HTML/JS UI (iframe-based tab model)
//   - list_projects   — return registered projects (hardcoded for step 1, JSON in v0.2)
//   - spawn_server    — launch code-server for a project (local Win or WSL distro)
//   - stop_server     — kill a code-server child process
//   - server_status   — list which projects currently have a live backend
//
// Out of scope for step 1 (deferred):
//   - native multi-WebView2 (iframe is enough for the model — see spike Phase 1b verification)
//   - lazy spawn / idle suspend
//   - JSON-backed registry + add-project form
//   - global hotkeys via tauri-plugin-global-shortcut
//   - SSH backend
//   - extension state migration / per-project --user-data-dir

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{Manager, State};

mod registry;
mod servers;

use servers::{spawn_for, ServerHandle};

#[derive(Default)]
pub struct AppState {
    pub servers: Mutex<HashMap<String, ServerHandle>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub env: String,
    pub port: u16,
    pub folder: String,
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

#[tauri::command]
fn list_projects() -> Vec<Project> {
    registry::default_projects()
}

#[tauri::command]
async fn spawn_server(
    state: State<'_, AppState>,
    project: Project,
) -> Result<ServerStatus, String> {
    {
        let servers = state.servers.lock().unwrap();
        if servers.contains_key(&project.id) {
            return Ok(ServerStatus {
                project_id: project.id,
                port: project.port,
                running: true,
            });
        }
    }
    let handle = spawn_for(&project).map_err(|e| e.to_string())?;
    let mut servers = state.servers.lock().unwrap();
    servers.insert(project.id.clone(), handle);
    Ok(ServerStatus {
        project_id: project.id,
        port: project.port,
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
            list_projects,
            spawn_server,
            stop_server,
            server_status,
        ])
        .on_window_event(|window, event| {
            // Cleanup all spawned servers on close so we don't leak processes.
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
