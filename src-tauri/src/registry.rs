// Project registry — JSON-backed (v0.2 A).
//
// File location:
//   Windows : %APPDATA%\vstabs\projects.json
//   Linux   : ~/.config/vstabs/projects.json
//   macOS   : ~/Library/Application Support/vstabs/projects.json
//
// Schema:
//   {
//     "version": 2,
//     "projects": [ { Project }, ... ]
//   }
//
// CRUD is exposed via Tauri commands (lib.rs). The whole file is rewritten
// on every mutation — small enough to not matter, simpler than partial writes.

use crate::Project;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use thiserror::Error;

const SCHEMA_VERSION: u32 = 3;

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("config dir not found")]
    NoConfigDir,
    #[error("project not found: {0}")]
    NotFound(String),
    #[error("duplicate project id: {0}")]
    DuplicateId(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RegistryFile {
    pub version: u32,
    pub projects: Vec<Project>,
}

impl Default for RegistryFile {
    fn default() -> Self {
        Self {
            version: SCHEMA_VERSION,
            projects: vec![],
        }
    }
}

pub fn registry_path() -> Result<PathBuf, RegistryError> {
    let base = dirs::config_dir().ok_or(RegistryError::NoConfigDir)?;
    Ok(base.join("vstabs").join("projects.json"))
}

pub fn load_or_create() -> Result<RegistryFile, RegistryError> {
    let path = registry_path()?;
    if !path.exists() {
        let parent = path.parent().ok_or(RegistryError::NoConfigDir)?;
        fs::create_dir_all(parent)?;
        let empty = RegistryFile::default();
        write_atomic(&path, &empty)?;
        return Ok(empty);
    }
    let raw = fs::read_to_string(&path)?;
    let parsed: RegistryFile = serde_json::from_str(&raw)?;
    Ok(parsed)
}

pub fn save(file: &RegistryFile) -> Result<(), RegistryError> {
    let path = registry_path()?;
    let parent = path.parent().ok_or(RegistryError::NoConfigDir)?;
    fs::create_dir_all(parent)?;
    write_atomic(&path, file)
}

fn write_atomic(path: &PathBuf, file: &RegistryFile) -> Result<(), RegistryError> {
    // Write to .tmp then rename — avoids half-written file if process dies mid-write.
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(file)?;
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(json.as_bytes())?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

pub fn add(file: &mut RegistryFile, project: Project) -> Result<(), RegistryError> {
    if file.projects.iter().any(|p| p.id == project.id) {
        return Err(RegistryError::DuplicateId(project.id));
    }
    file.projects.push(project);
    Ok(())
}

pub fn update(file: &mut RegistryFile, project: Project) -> Result<(), RegistryError> {
    let idx = file
        .projects
        .iter()
        .position(|p| p.id == project.id)
        .ok_or_else(|| RegistryError::NotFound(project.id.clone()))?;
    file.projects[idx] = project;
    Ok(())
}

pub fn remove(file: &mut RegistryFile, id: &str) -> Result<(), RegistryError> {
    let idx = file
        .projects
        .iter()
        .position(|p| p.id == id)
        .ok_or_else(|| RegistryError::NotFound(id.into()))?;
    file.projects.remove(idx);
    Ok(())
}

pub fn reorder(file: &mut RegistryFile, ordered_ids: Vec<String>) -> Result<(), RegistryError> {
    use std::collections::HashMap;
    let mut by_id: HashMap<String, Project> =
        file.projects.drain(..).map(|p| (p.id.clone(), p)).collect();
    let mut new_list = Vec::with_capacity(ordered_ids.len());
    for id in ordered_ids {
        let p = by_id
            .remove(&id)
            .ok_or_else(|| RegistryError::NotFound(id.clone()))?;
        new_list.push(p);
    }
    // Append any projects the caller forgot — defensive
    for (_, p) in by_id {
        new_list.push(p);
    }
    file.projects = new_list;
    Ok(())
}

// Kept for tests / dev seeding only. Production never surfaces this list —
// the JSON registry is the source of truth, empty by default.
#[allow(dead_code)]
pub fn sample_projects() -> Vec<Project> {
    vec![
        Project {
            id: "sample-local".into(),
            name: "sample-local".into(),
            icon: "🏠".into(),
            env: "local".into(),
            wsl_distro: None,
            ssh_host: None,
        },
        Project {
            id: "sample-wsl".into(),
            name: "sample-wsl".into(),
            icon: "🐧".into(),
            env: "wsl".into(),
            wsl_distro: Some("Ubuntu".into()),
            ssh_host: None,
        },
    ]
}
