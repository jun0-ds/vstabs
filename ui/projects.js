// Spike fallback only — used when ui/ is opened in a plain browser (not Tauri).
// In Tauri, projects come from the Rust backend's list_projects command.
//
// Keep this in sync with src-tauri/src/registry.rs::default_projects() until
// v0.2 introduces the JSON-backed registry.

const PROJECTS = [
  { id: "project-main", name: "project-main", icon: "🏠", env: "wsl",
    port: 8080, folder: "~/projects/main" },
  { id: "vstabs", name: "vstabs", icon: "📑", env: "wsl",
    port: 8081, folder: "~/projects/vstabs" },
];
