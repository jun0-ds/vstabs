// Spike fallback only — used when ui/ is opened in a plain browser (not Tauri).
// In Tauri, projects come from the Rust backend's list_projects command.
//
// Edit this file to point at your own folders/ports if you want to test the
// frontend in isolation. The numbers and names here are samples.

const PROJECTS = [
  { id: "sample-a", name: "sample-a", icon: "📁", env: "wsl",
    port: 8080, folder: "/home/your-user/sample-a" },
  { id: "sample-b", name: "sample-b", icon: "📂", env: "wsl",
    port: 8081, folder: "/home/your-user/sample-b" },
];
