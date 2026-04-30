// vstabs project registry (external data — eventual production format)
//
// Each entry drives one tab + one iframe + one code-server backend.
// Edit this file to add / remove projects (then reload the page).
//
// Field reference:
//   id        unique short slug (used as tab key, must be DOM-safe)
//   name      display label
//   icon      single emoji or unicode glyph (used in tab + favicon)
//   env       "local" | "wsl" | "ssh"  (for color coding + future routing)
//   envColor  CSS color for the env tag pill (override default per project if desired)
//   port      localhost port where this project's code-server is listening
//   folder    folder the code-server should open
//
// In production, this becomes JSON in %APPDATA%\vstabs\projects.json,
// edited via the vstabs UI. For the spike it's a JS file so file:// can load it.

const ENV_COLORS = {
  local: "#7cb342",
  wsl:   "#56b6c2",
  ssh:   "#a371f7",
};

const PROJECTS = [
  {
    id: "project-main",
    name: "project-main",
    icon: "🏠",
    env: "wsl",
    port: 8080,
    folder: "~/projects/main",
  },
  {
    id: "vstabs",
    name: "vstabs",
    icon: "📑",
    env: "wsl",
    port: 8081,
    folder: "~/projects/vstabs",
  },
];
