const { invoke } = window.__TAURI__.core;

const state = {
  folders: [],
  items: [],
  selected: null,
  appliedPath: null,
  env: null,
  // Web wallpaper property editor state for the current selection.
  webProps: [],
  webOverridesText: "{}",
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
    state.env = env;
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
  loadWebPanel(item);
}

// --- Web wallpaper property editor ------------------------------------------

const overridesKey = (path) => "webprops:" + path;

// Load and render the property editor for a web wallpaper (KDE only). Hides the
// panel for videos/images, off KDE, or when the wallpaper exposes no properties.
async function loadWebPanel(item) {
  state.webProps = [];
  if (!(state.env && state.env.is_kde) || item.kind !== "web") {
    hidePropsPanel();
    return;
  }
  state.webOverridesText = localStorage.getItem(overridesKey(item.path)) || "{}";
  let props = [];
  try {
    props = await invoke("web_properties", { path: item.path });
  } catch (_) {
    props = [];
  }
  // Guard against a slower request resolving after the user picked something else.
  if (!state.selected || state.selected.path !== item.path) return;
  state.webProps = props;
  renderProps(item);
}

function hidePropsPanel() {
  el("props-panel").classList.add("hidden");
  el("props-list").innerHTML = "";
}

function renderProps(item) {
  const list = el("props-list");
  list.innerHTML = "";
  if (!state.webProps.length) {
    hidePropsPanel();
    return;
  }
  el("props-title").textContent = item.name + " — settings";
  for (const p of state.webProps) {
    list.appendChild(propRow(item, p));
  }
  el("props-panel").classList.remove("hidden");
}

// Persist a changed property value and, if this wallpaper is live, re-apply.
function setProp(item, prop, value) {
  state.webOverridesText = Props.withOverride(
    state.webOverridesText,
    prop.name,
    value,
    prop.default,
  );
  localStorage.setItem(overridesKey(item.path), state.webOverridesText);
  if (state.appliedPath === item.path) scheduleLiveApply(item);
}

let liveTimer = null;
function scheduleLiveApply(item) {
  clearTimeout(liveTimer);
  liveTimer = setTimeout(() => {
    invoke("apply_wallpaper", { request: applyRequestFor(item) }).catch(() => {});
  }, 150);
}

// Build one editor row (label + typed control) for a property.
function propRow(item, prop) {
  const row = document.createElement("div");
  row.className = "prop-row";

  const label = document.createElement("span");
  label.className = "prop-label";
  label.textContent = prop.label;
  label.title = prop.label;
  row.appendChild(label);

  const ctl = document.createElement("div");
  ctl.className = "prop-control";
  const overrides = Props.parseOverrides(state.webOverridesText);
  const current = Props.valueFor(overrides, prop.name, prop.default);

  if (prop.type === "bool") {
    const cb = document.createElement("input");
    cb.type = "checkbox";
    cb.checked = current === true;
    cb.addEventListener("change", () => setProp(item, prop, cb.checked));
    ctl.appendChild(cb);
  } else if (prop.type === "slider") {
    const range = document.createElement("input");
    range.type = "range";
    range.min = prop.min;
    range.max = prop.max;
    range.step = prop.step > 0 ? prop.step : "any";
    range.value = current;
    const val = document.createElement("span");
    val.className = "prop-val";
    const fmt = (v) => (prop.step >= 1 ? String(Math.round(v)) : Number(v).toFixed(2));
    val.textContent = fmt(current);
    range.addEventListener("input", () => {
      val.textContent = fmt(range.value);
      setProp(item, prop, Number(range.value));
    });
    ctl.appendChild(range);
    ctl.appendChild(val);
  } else if (prop.type === "combo") {
    const sel = document.createElement("select");
    (prop.options || []).forEach((opt, i) => {
      const o = document.createElement("option");
      o.value = String(i);
      o.textContent = opt.label;
      if (opt.value === current) o.selected = true;
      sel.appendChild(o);
    });
    sel.addEventListener("change", () => {
      const opt = (prop.options || [])[parseInt(sel.value, 10)];
      if (opt) setProp(item, prop, opt.value);
    });
    ctl.appendChild(sel);
  } else if (prop.type === "color") {
    const color = document.createElement("input");
    color.type = "color";
    color.value = Props.colorToHex(current);
    color.addEventListener("change", () =>
      setProp(item, prop, Props.hexToColorString(color.value)),
    );
    ctl.appendChild(color);
  } else {
    const text = document.createElement("input");
    text.type = "text";
    text.value = current == null ? "" : String(current);
    text.addEventListener("change", () => setProp(item, prop, text.value));
    ctl.appendChild(text);
  }

  row.appendChild(ctl);
  return row;
}

function resetProps() {
  const item = state.selected;
  if (!item || item.kind !== "web") return;
  state.webOverridesText = "{}";
  localStorage.removeItem(overridesKey(item.path));
  renderProps(item);
  if (state.appliedPath === item.path) scheduleLiveApply(item);
}

// The apply request for an item, including web property overrides for web ones.
function applyRequestFor(item) {
  return {
    path: item.path,
    muted: el("muted").checked,
    fill_mode: parseInt(el("fill").value, 10),
    web_properties: item.kind === "web" ? state.webOverridesText : null,
  };
}

async function applySelected() {
  if (!state.selected) return;
  el("btn-apply").disabled = true;
  setStatus("Applying…");
  const request = applyRequestFor(state.selected);
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
  hidePropsPanel(); // a picked video has no web properties
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
el("props-reset").addEventListener("click", resetProps);

init();
