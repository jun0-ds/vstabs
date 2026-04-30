// vstabs v0.1 — frontend entry point.
//
// Tabs are driven by the backend's list_projects command. When a tab is clicked
// for the first time, we ask the backend to spawn its code-server, then point
// the iframe at http://127.0.0.1:{port}. Subsequent activations just toggle
// visibility — the iframe stays alive (warm switch).

const ENV_COLORS = {
  local: "#7cb342",
  wsl:   "#56b6c2",
  ssh:   "#a371f7",
};

const tabbar = document.getElementById("tabbar");
const content = document.getElementById("content");
const emptyEl = document.getElementById("empty-state");

const tabEls = {};
const iframeEls = {};
const tabState = {}; // id -> 'idle' | 'spawning' | 'running'

let projects = [];
let activeId = null;
let switchCount = 0;
let lastSwitchMs = 0;

// ---- Tauri bridge --------------------------------------------------------
//
// withGlobalTauri = true in tauri.conf.json puts the runtime on window.__TAURI__.
// We fall back to fetch-based behavior if loaded outside Tauri, so the same UI
// can be opened in a plain browser for spike-style testing.

const inTauri = typeof window.__TAURI__ !== "undefined";
const invoke = inTauri ? window.__TAURI__.core.invoke : null;

async function backendListProjects() {
  if (inTauri) return await invoke("list_projects");
  // Spike fallback — load from projects.js if present.
  if (typeof PROJECTS !== "undefined") return PROJECTS;
  return [];
}

async function backendSpawn(project) {
  if (!inTauri) {
    return { project_id: project.id, port: project.port, running: true };
  }
  return await invoke("spawn_server", { project });
}

async function backendStop(projectId) {
  if (!inTauri) return;
  return await invoke("stop_server", { projectId });
}

// ---- Favicon (emoji svg data url) ----------------------------------------

function setFavicon(emoji) {
  let link = document.querySelector("link[rel~='icon']");
  if (!link) {
    link = document.createElement("link");
    link.rel = "icon";
    document.head.appendChild(link);
  }
  const svg =
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">' +
    `<text x="50" y="55" text-anchor="middle" dominant-baseline="middle" font-size="80">${emoji}</text>` +
    "</svg>";
  link.type = "image/svg+xml";
  link.href = "data:image/svg+xml;utf8," + encodeURIComponent(svg);
}

// ---- Toast (small status feedback) ---------------------------------------

function toast(msg, ms = 2500) {
  const el = document.createElement("div");
  el.className = "toast";
  el.textContent = msg;
  document.body.appendChild(el);
  requestAnimationFrame(() => el.classList.add("show"));
  setTimeout(() => {
    el.classList.remove("show");
    setTimeout(() => el.remove(), 250);
  }, ms);
}

// ---- Tab activation ------------------------------------------------------

function setStatus(id, status) {
  tabState[id] = status;
  const dot = tabEls[id]?.querySelector(".status-dot");
  if (dot) {
    dot.classList.toggle("running", status === "running");
    dot.classList.toggle("spawning", status === "spawning");
  }
}

async function ensureBackend(project) {
  if (tabState[project.id] === "running") return;
  setStatus(project.id, "spawning");
  try {
    await backendSpawn(project);
    setStatus(project.id, "running");
  } catch (e) {
    setStatus(project.id, "idle");
    toast(`Failed to start ${project.name}: ${e}`);
    throw e;
  }
}

function showIframe(project) {
  let iframe = iframeEls[project.id];
  if (!iframe) {
    iframe = document.createElement("iframe");
    iframe.dataset.id = project.id;
    iframe.src = `http://127.0.0.1:${project.port}/?folder=${encodeURIComponent(project.folder)}`;
    content.appendChild(iframe);
    iframeEls[project.id] = iframe;
  }
  Object.entries(iframeEls).forEach(([k, el]) => el.classList.toggle("active", k === project.id));
  emptyEl.classList.add("hidden");
}

async function activate(id) {
  const project = projects.find((p) => p.id === id);
  if (!project) return;
  const t0 = performance.now();
  Object.entries(tabEls).forEach(([k, el]) => el.classList.toggle("active", k === id));
  await ensureBackend(project);
  showIframe(project);
  activeId = id;
  document.title = `${project.icon} ${project.name} — vstabs`;
  setFavicon(project.icon);
  switchCount += 1;
  lastSwitchMs = performance.now() - t0;
  renderMeta();
}

// ---- Tab bar build -------------------------------------------------------

function envColor(env) {
  return ENV_COLORS[env] || "#888";
}

function renderMeta() {
  const meta = tabbar.querySelector(".meta");
  if (!meta) return;
  const total = projects.length;
  meta.innerHTML =
    `<span>${total} project${total === 1 ? "" : "s"}</span>` +
    `<span>switch #${switchCount} ${lastSwitchMs.toFixed(0)}ms</span>`;
}

function buildTab(project) {
  const tab = document.createElement("button");
  tab.className = "tab";
  tab.style.setProperty("--env-color", envColor(project.env));
  tab.dataset.id = project.id;
  tab.innerHTML =
    `<span class="status-dot"></span>` +
    `<span class="icon">${project.icon}</span>` +
    `<span class="name">${project.name}</span>` +
    `<span class="env-tag">${project.env.toUpperCase()}</span>`;
  tab.addEventListener("click", () => activate(project.id));
  tabbar.appendChild(tab);
  tabEls[project.id] = tab;
  setStatus(project.id, "idle");
}

function buildAddButton() {
  const btn = document.createElement("button");
  btn.className = "add";
  btn.title = "Add project";
  btn.textContent = "+";
  btn.addEventListener("click", () => {
    toast("Add-project UI is v0.2. Edit src-tauri/src/registry.rs for now.");
  });
  tabbar.appendChild(btn);
}

function buildSpacerAndMeta() {
  const spacer = document.createElement("div");
  spacer.className = "spacer";
  tabbar.appendChild(spacer);
  const meta = document.createElement("div");
  meta.className = "meta";
  tabbar.appendChild(meta);
}

// ---- Hotkeys -------------------------------------------------------------

window.addEventListener("keydown", (e) => {
  if (e.ctrlKey && e.altKey && /^Digit[1-9]$/.test(e.code)) {
    const n = parseInt(e.code.slice(-1), 10);
    const project = projects[n - 1];
    if (project) {
      activate(project.id);
      e.preventDefault();
    }
  }
});

// ---- Boot ----------------------------------------------------------------

async function boot() {
  try {
    projects = await backendListProjects();
  } catch (e) {
    toast(`Failed to load projects: ${e}`);
    projects = [];
  }
  if (!projects.length) {
    document.title = "vstabs (no projects)";
    return;
  }
  projects.forEach(buildTab);
  buildAddButton();
  buildSpacerAndMeta();
  renderMeta();
  await activate(projects[0].id);
}

boot();
