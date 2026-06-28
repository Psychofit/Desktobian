const { invoke } = window.__TAURI__.core;

const state = {
  folders: [],
  items: [],
  selected: null,
  appliedPath: null,
};

const el = (id) => document.getElementById(id);

function loadFolders() {
  try {
    const saved = JSON.parse(localStorage.getItem("folders") || "null");
    if (Array.isArray(saved) && saved.length) return saved;
  } catch (_) {
    /* ignore */
  }
  return null;
}

function saveFolders() {
  localStorage.setItem("folders", JSON.stringify(state.folders));
}

async function init() {
  try {
    const env = await invoke("get_environment");
    el("env-line").textContent =
      `${env.desktop || "Unknown DE"} · ${env.session_type || "?"} — applies via ${env.apply_method}`;
    state.folders = loadFolders() || (await invoke("default_library_folders"));
    saveFolders();
    await refresh();
  } catch (e) {
    setStatus("Init error: " + e, "err");
  }
}

async function refresh() {
  setStatus("Scanning library…");
  el("grid").innerHTML = "";
  try {
    state.items = await invoke("scan_library", {
      folders: state.folders,
      thumbnails: true,
    });
  } catch (e) {
    setStatus("Scan failed: " + e, "err");
    return;
  }
  clearStatus();
  renderGrid();
}

function renderGrid() {
  const grid = el("grid");
  grid.innerHTML = "";
  el("empty").classList.toggle("hidden", state.items.length > 0);
  for (const item of state.items) {
    grid.appendChild(card(item));
  }
}

function card(item) {
  const c = document.createElement("div");
  c.className = "card";
  if (!item.supported) c.classList.add("unsupported");
  if (item.path === state.appliedPath) c.classList.add("applied");
  if (state.selected && item.id === state.selected.id) c.classList.add("selected");

  const thumb = item.thumbnail
    ? `<img class="thumb" src="${item.thumbnail}" alt="" />`
    : `<div class="thumb placeholder">🎞</div>`;

  const soon = item.supported
    ? ""
    : `<div class="soon-tag">${escapeHtml(item.kind)} — soon</div>`;

  c.innerHTML = `
    ${thumb}
    ${soon}
    <div class="card-body">
      <span class="card-name" title="${escapeHtml(item.path)}">${escapeHtml(item.name)}</span>
      <span class="kind">${escapeHtml(item.kind)}</span>
    </div>`;

  c.addEventListener("click", () => {
    if (!item.supported) {
      setStatus(
        `${item.kind} wallpapers aren't supported yet — coming soon.`,
        "err",
      );
      return;
    }
    select(item);
  });
  c.addEventListener("dblclick", () => {
    if (item.supported) {
      select(item);
      applySelected();
    }
  });
  return c;
}

function select(item) {
  state.selected = item;
  el("selected-info").textContent = item.name;
  el("btn-apply").disabled = false;
  renderGrid();
}

async function applySelected() {
  if (!state.selected) return;
  el("btn-apply").disabled = true;
  setStatus("Applying…");
  const request = {
    path: state.selected.path,
    muted: el("muted").checked,
    fill_mode: parseInt(el("fill").value, 10),
  };
  try {
    const res = await invoke("apply_wallpaper", { request });
    setStatus(res.message, res.ok ? "ok" : "err");
    if (res.ok) {
      state.appliedPath = state.selected.path;
      renderGrid();
    }
  } catch (e) {
    setStatus("Apply failed: " + e, "err");
  } finally {
    el("btn-apply").disabled = false;
  }
}

async function addVideo() {
  const path = await invoke("pick_video");
  if (!path) return;
  state.selected = {
    id: "picked-" + path,
    name: path.split("/").pop(),
    path,
    kind: "video",
  };
  await applySelected();
  await refresh();
}

async function addFolder() {
  const folder = await invoke("pick_folder");
  if (!folder) return;
  if (!state.folders.includes(folder)) {
    state.folders.push(folder);
    saveFolders();
  }
  await refresh();
}

function setStatus(msg, kind) {
  const s = el("status");
  s.textContent = msg;
  s.className = "status" + (kind ? " " + kind : "");
  s.classList.remove("hidden");
}

function clearStatus() {
  el("status").classList.add("hidden");
}

function escapeHtml(s) {
  return String(s).replace(
    /[&<>"']/g,
    (c) =>
      ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c],
  );
}

el("btn-refresh").addEventListener("click", refresh);
el("btn-pick").addEventListener("click", addVideo);
el("btn-folder").addEventListener("click", addFolder);
el("btn-apply").addEventListener("click", applySelected);

init();
