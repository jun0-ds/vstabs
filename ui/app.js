// vstabs v0.2 A — frontend.
//
// Reads the project list from the backend's JSON-backed registry
// (list_projects), builds a tab per project, lazy-spawns the code-server
// backend on first activation. Add Project modal + tab right-click context
// menu (edit / move / remove). Backend is the source of truth — every CRUD
// op round-trips through Tauri commands and re-renders the tab bar.

const ENV_COLORS = {
  local: "#7cb342",
  wsl:   "#56b6c2",
  ssh:   "#a371f7",
};

const tabbar  = document.getElementById("tabbar");
const content = document.getElementById("content");
const emptyEl = document.getElementById("empty-state");

const tabEls       = {}; // id -> tab button element
const iframeEls    = {}; // id -> iframe element
const tabState     = {}; // id -> 'idle' | 'spawning' | 'running' | 'failed'
const effectivePort = {}; // id -> port the WebView should connect to

let projects = [];
let activeId = null;
let switchCount = 0;
let lastSwitchMs = 0;

// ---- Tauri bridge ---------------------------------------------------------

const inTauri = typeof window.__TAURI__ !== "undefined";
const invoke  = inTauri ? window.__TAURI__.core.invoke : null;

async function backendListProjects() {
  if (inTauri) return await invoke("list_projects");
  if (typeof PROJECTS !== "undefined") return PROJECTS;
  return [];
}
async function backendAddProject(p)    { return inTauri ? invoke("add_project", { project: p })    : null; }
async function backendUpdateProject(p) { return inTauri ? invoke("update_project", { project: p }) : null; }
async function backendRemoveProject(id){ return inTauri ? invoke("remove_project", { projectId: id }) : null; }
async function backendReorderProjects(ids) { return inTauri ? invoke("reorder_projects", { orderedIds: ids }) : null; }
async function backendListWslDistros() { return inTauri ? invoke("list_wsl_distros") : []; }
async function backendListSshAliases() { return inTauri ? invoke("list_ssh_aliases") : []; }

async function backendSpawn(project) {
  if (!inTauri) return { project_id: project.id, port: project.port, running: true };
  return await invoke("spawn_server", { project });
}
async function backendStop(projectId) {
  if (!inTauri) return;
  return await invoke("stop_server", { projectId });
}

// ---- Tiny helpers ---------------------------------------------------------

function envColor(env) { return ENV_COLORS[env] || "#888"; }

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

function toast(msg, ms = 2500) {
  const el = document.createElement("div");
  el.className = "toast";
  el.textContent = msg;
  document.body.appendChild(el);
  requestAnimationFrame(() => el.classList.add("show"));
  setTimeout(() => { el.classList.remove("show"); setTimeout(() => el.remove(), 250); }, ms);
}

function slugify(s) {
  return s.trim().toLowerCase()
    .replace(/[^a-z0-9-]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 40) || "project";
}


// ---- Tab status -----------------------------------------------------------

function setStatus(id, status) {
  tabState[id] = status;
  const dot = tabEls[id]?.querySelector(".status-dot");
  if (dot) {
    dot.classList.remove("running", "spawning", "failed");
    if (status === "running" || status === "spawning" || status === "failed") {
      dot.classList.add(status);
    }
  }
}

async function ensureBackend(project) {
  if (tabState[project.id] === "running") return;
  setStatus(project.id, "spawning");
  try {
    const status = await backendSpawn(project);
    if (status && typeof status.port === "number") {
      effectivePort[project.id] = status.port;
    }
    setStatus(project.id, "running");
  } catch (e) {
    setStatus(project.id, "failed");
    toast(`Failed to start ${project.name}: ${e}`);
    throw e;
  }
}

function showIframe(project) {
  const port = effectivePort[project.id];
  let iframe = iframeEls[project.id];
  if (!iframe) {
    iframe = document.createElement("iframe");
    iframe.dataset.id = project.id;
    // No ?folder= — code-server restores the last-opened workspace from its
    // per-tab --user-data-dir, or shows the empty Welcome screen for first run.
    iframe.src = `http://127.0.0.1:${port}/`;
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
  try {
    await ensureBackend(project);
    showIframe(project);
  } catch {
    return;
  }
  activeId = id;
  document.title = `${project.icon} ${project.name} — vstabs`;
  setFavicon(project.icon);
  switchCount += 1;
  lastSwitchMs = performance.now() - t0;
  renderMeta();
}

// ---- Tab bar build --------------------------------------------------------

function renderMeta() {
  const meta = tabbar.querySelector(".meta");
  if (!meta) return;
  const total = projects.length;
  meta.innerHTML =
    `<span>${total} project${total === 1 ? "" : "s"}</span>` +
    `<span>switch #${switchCount} ${lastSwitchMs.toFixed(0)}ms</span>`;
}

function buildTabBar() {
  // Wipe and rebuild — easier than diff for v0.2 A
  tabbar.innerHTML = "";
  Object.keys(tabEls).forEach((k) => delete tabEls[k]);

  projects.forEach((project) => {
    const tab = document.createElement("button");
    tab.className = "tab";
    tab.style.setProperty("--env-color", envColor(project.env));
    tab.dataset.id = project.id;
    tab.innerHTML =
      `<span class="status-dot"></span>` +
      `<span class="icon">${project.icon || "📁"}</span>` +
      `<span class="name">${escapeHtml(project.name)}</span>` +
      `<span class="env-tag">${project.env.toUpperCase()}</span>`;
    tab.addEventListener("click", () => activate(project.id));
    tab.addEventListener("contextmenu", (ev) => {
      ev.preventDefault();
      openContextMenu(ev.clientX, ev.clientY, project.id);
    });
    tabbar.appendChild(tab);
    tabEls[project.id] = tab;
    setStatus(project.id, tabState[project.id] || "idle");
  });

  // "+" button
  const addBtn = document.createElement("button");
  addBtn.className = "add";
  addBtn.title = "Add project";
  addBtn.textContent = "+";
  addBtn.addEventListener("click", () => openModal("add"));
  tabbar.appendChild(addBtn);

  // Spacer + meta
  const spacer = document.createElement("div");
  spacer.className = "spacer";
  tabbar.appendChild(spacer);
  const meta = document.createElement("div");
  meta.className = "meta";
  tabbar.appendChild(meta);
  renderMeta();
}

function escapeHtml(s) {
  return s.replace(/[&<>"']/g, (c) => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;"
  }[c]));
}

// ---- Modal: add / edit project --------------------------------------------

const modal = {
  overlay: document.getElementById("modal-overlay"),
  title:   document.getElementById("modal-title"),
  name:    document.getElementById("f-name"),
  icon:    document.getElementById("f-icon"),
  envBox:  document.getElementById("f-env"),
  wslWrap: document.getElementById("f-conditional-wsl"),
  wslSel:  document.getElementById("f-wsl-distro"),
  sshWrap: document.getElementById("f-conditional-ssh"),
  sshHost: document.getElementById("f-ssh-host"),
  sshList: document.getElementById("f-ssh-aliases"),
  error:   document.getElementById("modal-error"),
  cancel:  document.getElementById("modal-cancel"),
  submit:  document.getElementById("modal-submit"),
};

const ENV_DEFAULT_ICON = { local: "🏠", wsl: "🐧", ssh: "☁️" };

let editingId = null;

function setEnv(env) {
  modal.overlay.dataset.env = env;
  modal.envBox.querySelectorAll("label").forEach((l) =>
    l.classList.toggle("active", l.dataset.env === env));
  modal.wslWrap.classList.toggle("hidden", env !== "wsl");
  modal.sshWrap.classList.toggle("hidden", env !== "ssh");
}

modal.envBox.addEventListener("click", (ev) => {
  const lbl = ev.target.closest("label[data-env]");
  if (lbl) setEnv(lbl.dataset.env);
});

modal.cancel.addEventListener("click", () => closeModal());

modal.submit.addEventListener("click", async () => {
  const env = modal.overlay.dataset.env || "wsl";
  const name = modal.name.value.trim() || autoNextName();
  const icon = modal.icon.value.trim() || ENV_DEFAULT_ICON[env] || "📁";
  const wslDistro = env === "wsl" ? modal.wslSel.value.trim() : null;
  const sshHost   = env === "ssh" ? modal.sshHost.value.trim() : null;

  if (env === "wsl" && !wslDistro) return showModalError("WSL distro is required.");
  if (env === "ssh" && !sshHost)   return showModalError("SSH host is required.");

  const id = editingId || ensureUniqueId(slugify(name));
  const project = {
    id, name, icon, env,
    wsl_distro: wslDistro,
    ssh_host: sshHost,
  };

  try {
    if (editingId) {
      await backendUpdateProject(project);
      toast(`Updated: ${name}`);
    } else {
      await backendAddProject(project);
      toast(`Added: ${name}`);
    }
    closeModal();
    await reloadProjects();
    // For *new* projects, jump straight into the spawn so the user lands on
    // VS Code's empty Welcome screen and can File → Open Folder immediately.
    // (For edits we keep whatever was active before.)
    if (!editingId) {
      await activate(project.id);
    }
  } catch (e) {
    showModalError(String(e));
  }
});

function autoNextName() {
  const base = "Untitled";
  const taken = new Set(projects.map((p) => p.name));
  if (!taken.has(base)) return base;
  for (let i = 2; i < 1000; i++) {
    const candidate = `${base} ${i}`;
    if (!taken.has(candidate)) return candidate;
  }
  return `${base} ${Date.now()}`;
}

function showModalError(msg) {
  modal.error.textContent = msg;
  modal.error.classList.remove("hidden");
}

function ensureUniqueId(base) {
  const taken = new Set(projects.map((p) => p.id));
  if (!taken.has(base)) return base;
  for (let i = 2; i < 1000; i++) {
    const candidate = `${base}-${i}`;
    if (!taken.has(candidate)) return candidate;
  }
  return `${base}-${Date.now()}`;
}

async function openModal(mode, project = null) {
  editingId = mode === "edit" && project ? project.id : null;
  modal.title.textContent = mode === "edit" ? "Edit project" : "Add project";
  modal.submit.textContent = mode === "edit" ? "Save" : "Add";
  modal.error.classList.add("hidden");

  // Populate WSL distro dropdown + SSH alias datalist on every open (cheap).
  await populateHostHints();

  if (mode === "edit" && project) {
    modal.name.value = project.name;
    modal.icon.value = project.icon || "";
    setEnv(project.env);
    if (project.env === "wsl" && project.wsl_distro) modal.wslSel.value = project.wsl_distro;
    if (project.env === "ssh" && project.ssh_host)   modal.sshHost.value = project.ssh_host;
  } else {
    modal.name.value = "";
    modal.icon.value = "";
    modal.sshHost.value = "";
    setEnv("wsl");
  }
  modal.overlay.classList.remove("hidden");
  modal.name.focus();
}

function closeModal() {
  modal.overlay.classList.add("hidden");
  editingId = null;
}

async function populateHostHints() {
  // WSL distros
  modal.wslSel.innerHTML = "";
  let distros = [];
  try { distros = await backendListWslDistros(); } catch { distros = []; }
  if (!distros.length) distros = ["Ubuntu"];
  distros.forEach((d) => {
    const opt = document.createElement("option");
    opt.value = d; opt.textContent = d;
    modal.wslSel.appendChild(opt);
  });

  // SSH aliases
  modal.sshList.innerHTML = "";
  let aliases = [];
  try { aliases = await backendListSshAliases(); } catch { aliases = []; }
  aliases.forEach((a) => {
    const opt = document.createElement("option");
    opt.value = a;
    modal.sshList.appendChild(opt);
  });
}

// ---- Context menu ---------------------------------------------------------

const ctxMenu = document.getElementById("ctx-menu");
let ctxTargetId = null;

function openContextMenu(x, y, projectId) {
  ctxTargetId = projectId;
  ctxMenu.style.left = `${x}px`;
  ctxMenu.style.top  = `${y}px`;
  ctxMenu.classList.remove("hidden");
}

function closeContextMenu() {
  ctxMenu.classList.add("hidden");
  ctxTargetId = null;
}

ctxMenu.addEventListener("click", async (ev) => {
  const item = ev.target.closest(".item");
  if (!item || !ctxTargetId) return;
  const action = item.dataset.action;
  const id = ctxTargetId;
  closeContextMenu();
  const project = projects.find((p) => p.id === id);
  if (!project) return;

  switch (action) {
    case "edit":
      openModal("edit", project);
      break;
    case "move-left":
    case "move-right": {
      const idx = projects.findIndex((p) => p.id === id);
      const newIdx = action === "move-left" ? idx - 1 : idx + 1;
      if (newIdx < 0 || newIdx >= projects.length) break;
      [projects[idx], projects[newIdx]] = [projects[newIdx], projects[idx]];
      try {
        await backendReorderProjects(projects.map((p) => p.id));
        buildTabBar();
      } catch (e) {
        toast(`Reorder failed: ${e}`);
        await reloadProjects();
      }
      break;
    }
    case "remove":
      openConfirmRemove(project);
      break;
  }
});

window.addEventListener("click", (ev) => {
  if (!ctxMenu.contains(ev.target)) closeContextMenu();
});
window.addEventListener("keydown", (ev) => {
  if (ev.key === "Escape") {
    closeContextMenu();
    closeModal();
    closeConfirm();
  }
});

// ---- Confirm dialog (remove) ---------------------------------------------

const confirmDlg = {
  overlay: document.getElementById("confirm-overlay"),
  message: document.getElementById("confirm-message"),
  cancel:  document.getElementById("confirm-cancel"),
  ok:      document.getElementById("confirm-ok"),
};
let confirmAction = null;

function openConfirmRemove(project) {
  confirmDlg.message.textContent =
    `Remove "${project.name}" from vstabs?\n\nStops the backend if running and ` +
    `deletes the per-tab vscode user-data-dir. The folders you opened inside ` +
    `vscode are not touched.`;
  confirmAction = async () => {
    try {
      await backendRemoveProject(project.id);
      delete tabState[project.id];
      delete effectivePort[project.id];
      const iframe = iframeEls[project.id];
      if (iframe) { iframe.remove(); delete iframeEls[project.id]; }
      if (activeId === project.id) {
        activeId = null;
        emptyEl.classList.toggle("hidden", projects.length > 1);
      }
      toast(`Removed: ${project.name}`);
      await reloadProjects();
    } catch (e) {
      toast(`Remove failed: ${e}`);
    }
  };
  confirmDlg.overlay.classList.remove("hidden");
}

function closeConfirm() {
  confirmDlg.overlay.classList.add("hidden");
  confirmAction = null;
}

confirmDlg.cancel.addEventListener("click", closeConfirm);
confirmDlg.ok.addEventListener("click", async () => {
  const fn = confirmAction;
  closeConfirm();
  if (fn) await fn();
});

// ---- Empty-state CTA + hotkeys -------------------------------------------

document.getElementById("empty-cta").addEventListener("click", () => openModal("add"));

window.addEventListener("keydown", (ev) => {
  if (ev.ctrlKey && ev.altKey && /^Digit[1-9]$/.test(ev.code)) {
    const n = parseInt(ev.code.slice(-1), 10);
    const project = projects[n - 1];
    if (project) {
      activate(project.id);
      ev.preventDefault();
    }
  }
});

// ---- Boot -----------------------------------------------------------------

async function reloadProjects() {
  try {
    projects = await backendListProjects();
  } catch (e) {
    toast(`Failed to load projects: ${e}`);
    projects = [];
  }
  if (!projects.length) {
    document.title = "vstabs";
    setFavicon("📁");
    emptyEl.classList.remove("hidden");
    tabbar.innerHTML = "";
    // Still need a "+" button so the user can add the first project from the bar too.
    const addBtn = document.createElement("button");
    addBtn.className = "add";
    addBtn.title = "Add project";
    addBtn.textContent = "+";
    addBtn.addEventListener("click", () => openModal("add"));
    tabbar.appendChild(addBtn);
    return;
  }
  buildTabBar();
  // Activate the first project unless the active one is still in the list.
  const target = projects.find((p) => p.id === activeId) || projects[0];
  await activate(target.id);
}

reloadProjects();
