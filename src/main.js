const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const opener = window.__TAURI__?.opener;

/** @typedef {{
 *  id: string,
 *  provider: "chocolatey" | "winget" | "scoop",
 *  name: string,
 *  version?: string,
 *  availableVersion?: string,
 *  summary?: string,
 *  category?: string,
 *  source?: string,
 *  pinned: boolean,
 *  outdated: boolean,
 * }} Package */

/** @typedef {{
 *  id: string,
 *  displayName: string,
 *  publisher?: string,
 *  displayVersion?: string,
 *  estimatedSizeKb?: number,
 *  protected: boolean,
 * }} Program */

/** @typedef {{ name: string, url: string, disabled: boolean, priority?: number }} ChocoSource */

/** @type {"installed" | "browse" | "updates" | "programs" | "sources" | "settings"} */
let view = "installed";
/** @type {Package[]} */
let packages = [];
/** @type {Program[]} */
let programs = [];
/** @type {ChocoSource[]} */
let sources = [];
/** @type {Set<string>} */
const selected = new Set();
/** @type {Set<string>} */
const selectedPrograms = new Set();
/** @type {Map<string, {status: string, message?: string}>} */
const statusById = new Map();
let busy = false;
/** @type {string} */
let sortKey = "name";
/** @type {"asc" | "desc"} */
let sortDir = "asc";
let browseQuery = "";
/** @type {any} */
let providerStatusCache = null;

const els = {
  packageRows: document.getElementById("package-rows"),
  programRows: document.getElementById("program-rows"),
  sourceRows: document.getElementById("source-rows"),
  search: document.getElementById("search"),
  filterChoco: document.getElementById("filter-choco"),
  filterWinget: document.getElementById("filter-winget"),
  filterScoop: document.getElementById("filter-scoop"),
  showSystem: document.getElementById("show-system"),
  refreshBtn: document.getElementById("refresh-btn"),
  selectVisibleBtn: document.getElementById("select-visible-btn"),
  clearBtn: document.getElementById("clear-btn"),
  selectAll: document.getElementById("select-all"),
  selectAllPrograms: document.getElementById("select-all-programs"),
  installBtn: document.getElementById("install-btn"),
  upgradeBtn: document.getElementById("upgrade-btn"),
  upgradeAllBtn: document.getElementById("upgrade-all-btn"),
  uninstallBtn: document.getElementById("uninstall-btn"),
  pinBtn: document.getElementById("pin-btn"),
  unpinBtn: document.getElementById("unpin-btn"),
  uninstallProgramsBtn: document.getElementById("uninstall-programs-btn"),
  selectedCount: document.getElementById("selected-count"),
  selectedDetail: document.getElementById("selected-detail"),
  countLabel: document.getElementById("count-label"),
  elevationBadge: document.getElementById("elevation-badge"),
  chocoBadge: document.getElementById("choco-badge"),
  wingetBadge: document.getElementById("winget-badge"),
  scoopBadge: document.getElementById("scoop-badge"),
  emptyState: document.getElementById("empty-state"),
  loadingState: document.getElementById("loading-state"),
  programsEmpty: document.getElementById("programs-empty"),
  programsLoading: document.getElementById("programs-loading"),
  sourcesEmpty: document.getElementById("sources-empty"),
  packagesPanel: document.getElementById("packages-panel"),
  programsPanel: document.getElementById("programs-panel"),
  sourcesPanel: document.getElementById("sources-panel"),
  settingsPanel: document.getElementById("settings-panel"),
  log: document.getElementById("log"),
  clearLogBtn: document.getElementById("clear-log-btn"),
  confirmDialog: document.getElementById("confirm-dialog"),
  confirmTitle: document.getElementById("confirm-title"),
  confirmMessage: document.getElementById("confirm-message"),
  confirmList: document.getElementById("confirm-list"),
  confirmOk: document.getElementById("confirm-ok"),
  sourceName: document.getElementById("source-name"),
  sourceUrl: document.getElementById("source-url"),
  addSourceBtn: document.getElementById("add-source-btn"),
  bootstrapChocoBtn: document.getElementById("bootstrap-choco-btn"),
  bootstrapAllBtn: document.getElementById("bootstrap-all-btn"),
  openWingetBtn: document.getElementById("open-winget-btn"),
  settingsChocoDesc: document.getElementById("settings-choco-desc"),
  settingsWingetDesc: document.getElementById("settings-winget-desc"),
  settingsScoopDesc: document.getElementById("settings-scoop-desc"),
  updateAuthority: document.getElementById("update-authority"),
  showUpdateDuplicates: document.getElementById("show-update-duplicates"),
};

let bootstrappingProviders = false;

function logLine(message) {
  const stamp = new Date().toLocaleTimeString();
  els.log.textContent += `[${stamp}] ${message}\n`;
  els.log.scrollTop = els.log.scrollHeight;
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function escapeAttr(value) {
  return escapeHtml(value).replaceAll("`", "&#96;");
}

function providerFilters() {
  return {
    includeChocolatey: els.filterChoco.checked,
    includeWinget: els.filterWinget.checked,
    includeScoop: els.filterScoop.checked,
  };
}

function formatSize(kb) {
  if (kb == null || Number.isNaN(kb)) return "—";
  if (kb < 1024) return `${Math.round(kb)} KB`;
  const mb = kb / 1024;
  if (mb < 1024) return `${mb.toFixed(mb < 10 ? 1 : 0)} MB`;
  return `${(mb / 1024).toFixed(1)} GB`;
}

function compareText(a, b) {
  return a.localeCompare(b, undefined, { sensitivity: "base", numeric: true });
}

function setToolBadge(el, status, label) {
  if (!status) {
    el.textContent = `${label}…`;
    el.className = "auth-badge pending";
    return;
  }
  if (status.available) {
    el.textContent = status.version ? `${label} ${status.version}` : `${label} ready`;
    el.className = "auth-badge ok";
  } else {
    el.textContent = `${label} missing`;
    el.className = "auth-badge pending";
  }
}

async function refreshProviderStatus() {
  try {
    providerStatusCache = await invoke("provider_status");
    setToolBadge(els.chocoBadge, providerStatusCache.chocolatey, "Chocolatey");
    setToolBadge(els.wingetBadge, providerStatusCache.winget, "winget");
    setToolBadge(els.scoopBadge, providerStatusCache.scoop, "Scoop");
    updateSettingsPanel();
  } catch (err) {
    logLine(`Failed to check providers: ${err}`);
  }
}

function updateSettingsPanel() {
  const s = providerStatusCache;
  if (!s) return;
  els.settingsChocoDesc.textContent = s.chocolatey.available
    ? `Installed${s.chocolatey.path ? ` at ${s.chocolatey.path}` : ""}${s.chocolatey.version ? ` (${s.chocolatey.version})` : ""}`
    : s.chocolatey.message || "Not installed";
  els.bootstrapChocoBtn.disabled = busy || bootstrappingProviders || !!s.chocolatey.available;
  els.settingsWingetDesc.textContent = s.winget.available
    ? `Installed${s.winget.path ? ` at ${s.winget.path}` : ""}${s.winget.version ? ` (${s.winget.version})` : ""}`
    : s.winget.message || "Not installed";
  els.settingsScoopDesc.textContent = s.scoop.available
    ? `Installed${s.scoop.path ? ` at ${s.scoop.path}` : ""}${s.scoop.version ? ` (${s.scoop.version})` : ""}`
    : s.scoop.message || "Not installed — optional provider";
  const anyMissing =
    !s.chocolatey.available || !s.winget.available || !s.scoop.available;
  if (els.bootstrapAllBtn) {
    els.bootstrapAllBtn.disabled = busy || bootstrappingProviders || !anyMissing;
  }
}

async function loadUpdatePrefs() {
  try {
    const all = await invoke("settings_get");
    const authority = all?.updateAuthority;
    const showDupes = all?.showUpdateDuplicates;
    if (els.updateAuthority && typeof authority === "string") {
      els.updateAuthority.value = authority;
    }
    if (els.showUpdateDuplicates) {
      els.showUpdateDuplicates.checked = !!showDupes;
    }
  } catch {
    /* defaults from backend */
  }
}

async function saveUpdatePrefs() {
  try {
    const partial = {};
    if (els.updateAuthority) {
      partial.updateAuthority = els.updateAuthority.value;
    }
    if (els.showUpdateDuplicates) {
      partial.showUpdateDuplicates = els.showUpdateDuplicates.checked;
    }
    await invoke("settings_set", { partial });
  } catch (err) {
    logLine(`Failed to save update preferences: ${err}`);
  }
}

async function ensureProvidersInBackground() {
  if (bootstrappingProviders) return;
  bootstrappingProviders = true;
  updateSettingsPanel();
  try {
    await invoke("ensure_providers");
  } catch (err) {
    logLine(`Provider bootstrap error: ${err}`);
  } finally {
    bootstrappingProviders = false;
    await refreshProviderStatus();
    if (isPackageView()) {
      await loadPackages();
    } else {
      render();
    }
  }
}

function isPackageView() {
  return view === "installed" || view === "browse" || view === "updates";
}

function visiblePackages() {
  const q = els.search.value.trim().toLowerCase();
  let list = packages;
  if (q && view !== "browse") {
    list = packages.filter((p) => {
      const hay = `${p.name} ${p.id} ${p.category ?? ""} ${p.summary ?? ""} ${p.source ?? ""}`.toLowerCase();
      return hay.includes(q);
    });
  }
  const dir = sortDir === "asc" ? 1 : -1;
  return [...list].sort((a, b) => {
    const av = packageSortValue(a);
    const bv = packageSortValue(b);
    const cmp = compareText(String(av), String(bv));
    if (cmp === 0) return compareText(a.name, b.name) * dir;
    return cmp * dir;
  });
}

function packageSortValue(p) {
  switch (sortKey) {
    case "category":
      return p.category ?? "";
    case "provider":
      return p.provider;
    case "version":
      return p.version ?? "";
    case "available":
      return p.availableVersion ?? "";
    case "source":
      return p.source ?? "";
    case "status":
      return statusById.get(p.id)?.status ?? "";
    case "name":
    default:
      return p.name;
  }
}

function visiblePrograms() {
  const q = els.search.value.trim().toLowerCase();
  let list = programs;
  if (q) {
    list = programs.filter((p) => {
      const hay = `${p.displayName} ${p.publisher ?? ""} ${p.displayVersion ?? ""}`.toLowerCase();
      return hay.includes(q);
    });
  }
  const dir = sortDir === "asc" ? 1 : -1;
  return [...list].sort((a, b) => {
    let av;
    let bv;
    switch (sortKey) {
      case "publisher":
        av = a.publisher ?? "";
        bv = b.publisher ?? "";
        break;
      case "version":
        av = a.displayVersion ?? "";
        bv = b.displayVersion ?? "";
        break;
      case "size":
        return ((a.estimatedSizeKb ?? -1) - (b.estimatedSizeKb ?? -1)) * dir;
      case "status":
        av = statusById.get(a.id)?.status ?? "";
        bv = statusById.get(b.id)?.status ?? "";
        break;
      default:
        av = a.displayName;
        bv = b.displayName;
    }
    const cmp = compareText(String(av), String(bv));
    if (cmp === 0) return compareText(a.displayName, b.displayName) * dir;
    return cmp * dir;
  });
}

function updateSortHeaders() {
  document.querySelectorAll(".sort-btn").forEach((btn) => {
    const key = btn.getAttribute("data-sort");
    const active = key === sortKey;
    btn.setAttribute("aria-pressed", active ? "true" : "false");
    const ind = btn.querySelector(".sort-ind");
    if (ind) ind.textContent = active ? (sortDir === "asc" ? "▲" : "▼") : "";
  });
}

function updateViewChrome() {
  document.querySelectorAll(".view-btn").forEach((btn) => {
    btn.classList.toggle("active", btn.dataset.view === view);
  });

  const packageMode = isPackageView();
  const programsMode = view === "programs";
  const sourcesMode = view === "sources";
  const settingsMode = view === "settings";

  els.packagesPanel.classList.toggle("hidden", !packageMode);
  els.programsPanel.classList.toggle("hidden", !programsMode);
  els.sourcesPanel.classList.toggle("hidden", !sourcesMode);
  els.settingsPanel.classList.toggle("hidden", !settingsMode);

  document.querySelectorAll(".provider-filter").forEach((el) => {
    el.classList.toggle("hidden", !packageMode);
  });
  document.querySelector(".programs-only")?.classList.toggle("hidden", !programsMode);

  els.search.classList.toggle("hidden", sourcesMode || settingsMode);
  els.selectVisibleBtn.classList.toggle("hidden", sourcesMode || settingsMode);
  els.clearBtn.classList.toggle("hidden", sourcesMode || settingsMode);
  els.refreshBtn.classList.toggle("hidden", settingsMode);

    if (view === "browse") {
      els.search.placeholder = "Search packages… (popular apps shown when empty)";
    } else if (programsMode) {
    els.search.placeholder = "Search by name or publisher…";
  } else {
    els.search.placeholder = "Filter list…";
  }

  els.installBtn.classList.toggle("hidden", view !== "browse");
  els.upgradeBtn.classList.toggle("hidden", !(view === "installed" || view === "updates"));
  els.upgradeAllBtn.classList.toggle("hidden", view !== "updates");
  els.uninstallBtn.classList.toggle("hidden", view !== "installed");
  els.pinBtn.classList.toggle("hidden", view !== "installed");
  els.unpinBtn.classList.toggle("hidden", view !== "installed");
  els.uninstallProgramsBtn.classList.toggle("hidden", !programsMode);

  if (sourcesMode || settingsMode) {
    els.installBtn.disabled = true;
    els.upgradeBtn.disabled = true;
    els.upgradeAllBtn.disabled = true;
    els.uninstallBtn.disabled = true;
    els.pinBtn.disabled = true;
    els.unpinBtn.disabled = true;
    els.uninstallProgramsBtn.disabled = true;
  }
}

function updateSelectionUi() {
  if (view === "programs") {
    const list = programs.filter((p) => selectedPrograms.has(p.id) && !p.protected);
    els.selectedCount.textContent = String(list.length);
    els.selectedDetail.textContent = list.length ? "programs" : "—";
    els.uninstallProgramsBtn.disabled = busy || list.length === 0;
    const visible = visiblePrograms().filter((p) => !p.protected);
    const allSelected = visible.length > 0 && visible.every((p) => selectedPrograms.has(p.id));
    els.selectAllPrograms.checked = allSelected;
    els.selectAllPrograms.indeterminate =
      !allSelected && visible.some((p) => selectedPrograms.has(p.id));
    return;
  }

  if (!isPackageView()) {
    els.selectedCount.textContent = "0";
    els.selectedDetail.textContent = "—";
    return;
  }

  const list = packages.filter((p) => selected.has(p.id));
  els.selectedCount.textContent = String(list.length);
  els.selectedDetail.textContent = list.length ? "packages" : "—";

  els.installBtn.disabled = busy || view !== "browse" || list.length === 0;
  els.upgradeBtn.disabled =
    busy || !(view === "installed" || view === "updates") || list.length === 0;
  els.upgradeAllBtn.disabled = busy || view !== "updates" || packages.length === 0;
  els.uninstallBtn.disabled = busy || view !== "installed" || list.length === 0;
  els.pinBtn.disabled = busy || view !== "installed" || list.length === 0;
  els.unpinBtn.disabled = busy || view !== "installed" || list.length === 0;

  const visible = visiblePackages();
  const allSelected = visible.length > 0 && visible.every((p) => selected.has(p.id));
  els.selectAll.checked = allSelected;
  els.selectAll.indeterminate = !allSelected && visible.some((p) => selected.has(p.id));
}

function renderPackages() {
  const visible = visiblePackages();
  els.countLabel.textContent = `${packages.length} package${packages.length === 1 ? "" : "s"}`;
  els.loadingState.classList.add("hidden");
  els.emptyState.classList.toggle("hidden", visible.length > 0);

  els.packageRows.innerHTML = "";
  const frag = document.createDocumentFragment();
  for (const p of visible) {
    const tr = document.createElement("tr");
    if (selected.has(p.id)) tr.classList.add("selected");
    const status = statusById.get(p.id);
    const statusText = status?.status ?? "";
    const statusClass = statusText ? ` status-pill ${statusText}` : "status-pill";
    const versionCell = p.outdated
      ? `${escapeHtml(p.version ?? "—")} <span class="outdated-mark">outdated</span>`
      : escapeHtml(p.version ?? "—");
    tr.innerHTML = `
      <td class="col-check">
        <input type="checkbox" data-id="${escapeAttr(p.id)}" ${
          selected.has(p.id) ? "checked" : ""
        } ${busy ? "disabled" : ""} />
      </td>
      <td class="name-cell">${escapeHtml(p.name)}${
        p.pinned ? ' <span class="muted">(pinned)</span>' : ""
      }</td>
      <td class="muted">${escapeHtml(p.category ?? "—")}</td>
      <td class="muted">${escapeHtml(p.provider)}</td>
      <td class="muted">${versionCell}</td>
      <td class="muted">${escapeHtml(p.availableVersion ?? "—")}</td>
      <td class="muted">${escapeHtml(p.source ?? "—")}</td>
      <td><span class="${statusClass}">${
        statusText ? escapeHtml(statusText) : "—"
      }</span></td>
    `;
    frag.appendChild(tr);
  }
  els.packageRows.appendChild(frag);
  updateSortHeaders();
  updateSelectionUi();
}

function renderPrograms() {
  const visible = visiblePrograms();
  els.countLabel.textContent = `${programs.length} program${programs.length === 1 ? "" : "s"}`;
  els.programsLoading.classList.add("hidden");
  els.programsEmpty.classList.toggle("hidden", visible.length > 0);

  els.programRows.innerHTML = "";
  const frag = document.createDocumentFragment();
  for (const p of visible) {
    const tr = document.createElement("tr");
    if (selectedPrograms.has(p.id)) tr.classList.add("selected");
    if (p.protected) tr.classList.add("protected");
    const status = statusById.get(p.id);
    const statusText = status?.status ?? "";
    const statusClass = statusText ? ` status-pill ${statusText}` : "status-pill";
    tr.innerHTML = `
      <td class="col-check">
        <input type="checkbox" data-program-id="${escapeAttr(p.id)}" ${
          selectedPrograms.has(p.id) ? "checked" : ""
        } ${p.protected || busy ? "disabled" : ""} />
      </td>
      <td class="name-cell">${escapeHtml(p.displayName)}${
        p.protected ? ' <span class="muted">(protected)</span>' : ""
      }</td>
      <td class="muted">${escapeHtml(p.publisher ?? "—")}</td>
      <td class="muted">${escapeHtml(p.displayVersion ?? "—")}</td>
      <td class="muted">${formatSize(p.estimatedSizeKb)}</td>
      <td><span class="${statusClass}">${
        statusText ? escapeHtml(statusText) : "—"
      }</span></td>
    `;
    frag.appendChild(tr);
  }
  els.programRows.appendChild(frag);
  updateSortHeaders();
  updateSelectionUi();
}

function renderSources() {
  els.countLabel.textContent = `${sources.length} source${sources.length === 1 ? "" : "s"}`;
  els.sourcesEmpty.classList.toggle("hidden", sources.length > 0);
  els.sourceRows.innerHTML = "";
  const frag = document.createDocumentFragment();
  for (const s of sources) {
    const tr = document.createElement("tr");
    tr.innerHTML = `
      <td class="name-cell">${escapeHtml(s.name)}</td>
      <td class="muted">${escapeHtml(s.url)}</td>
      <td class="muted">${s.priority ?? "—"}</td>
      <td class="muted">${s.disabled ? "disabled" : "enabled"}</td>
      <td><button type="button" class="text-btn tiny" data-remove-source="${escapeAttr(
        s.name
      )}">Remove</button></td>
    `;
    frag.appendChild(tr);
  }
  els.sourceRows.appendChild(frag);
  updateSelectionUi();
}

function render() {
  updateViewChrome();
  if (isPackageView()) renderPackages();
  else if (view === "programs") renderPrograms();
  else if (view === "sources") renderSources();
  else if (view === "settings") {
    els.countLabel.textContent = "Settings";
    updateSettingsPanel();
    updateSelectionUi();
  }
}

async function loadCurrentView() {
  if (view === "settings") {
    await refreshProviderStatus();
    render();
    return;
  }
  if (view === "sources") {
    await loadSources();
    return;
  }
  if (view === "programs") {
    await loadPrograms();
    return;
  }
  await loadPackages();
}

async function loadPackages() {
  els.loadingState.classList.remove("hidden");
  els.emptyState.classList.add("hidden");
  const filters = providerFilters();
  try {
    if (view === "browse") {
      const q = els.search.value.trim();
      browseQuery = q;
      if (!q) {
        packages = await invoke("list_popular_packages", filters);
        render();
        logLine(
          `Showing ${packages.length} popular package(s). Type to search the catalogs.`
        );
        return;
      }
      packages = await invoke("search_packages", { query: q, ...filters });
      logLine(`Found ${packages.length} package(s) for “${q}”.`);
    } else if (view === "updates") {
      const authority = els.updateAuthority?.value || "winget";
      const showDuplicates = !!els.showUpdateDuplicates?.checked;
      packages = await invoke("list_outdated", {
        ...filters,
        preferProvider: authority,
        showDuplicates,
      });
      logLine(
        showDuplicates
          ? `Found ${packages.length} outdated package(s) (duplicates shown).`
          : `Found ${packages.length} outdated package(s) (authority: ${authority}).`
      );
    } else {
      packages = await invoke("list_installed", filters);
      logLine(`Loaded ${packages.length} installed package(s).`);
    }
    for (const id of [...selected]) {
      if (!packages.some((p) => p.id === id)) selected.delete(id);
    }
    render();
  } catch (err) {
    els.loadingState.classList.add("hidden");
    logLine(`Failed to load packages: ${err}`);
    packages = [];
    render();
  }
}

async function loadPrograms() {
  els.programsLoading.classList.remove("hidden");
  els.programsEmpty.classList.add("hidden");
  try {
    programs = await invoke("list_programs", {
      showSystem: els.showSystem.checked,
    });
    for (const id of [...selectedPrograms]) {
      if (!programs.some((p) => p.id === id)) selectedPrograms.delete(id);
    }
    render();
    logLine(`Loaded ${programs.length} programs.`);
  } catch (err) {
    els.programsLoading.classList.add("hidden");
    logLine(`Failed to list programs: ${err}`);
  }
}

async function loadSources() {
  try {
    sources = await invoke("list_choco_sources");
    render();
    logLine(`Loaded ${sources.length} Chocolatey source(s).`);
  } catch (err) {
    sources = [];
    render();
    logLine(`Failed to list sources: ${err}`);
  }
}

async function refreshElevation() {
  try {
    const elevated = await invoke("check_elevated");
    if (elevated) {
      els.elevationBadge.textContent = "Administrator";
      els.elevationBadge.className = "auth-badge ok";
    } else {
      els.elevationBadge.textContent = "Not elevated";
      els.elevationBadge.className = "auth-badge pending";
    }
  } catch {
    els.elevationBadge.textContent = "Elevation unknown";
    els.elevationBadge.className = "auth-badge error";
  }
}

async function confirmAction(title, message, names, okLabel, danger = true) {
  els.confirmTitle.textContent = title;
  els.confirmMessage.textContent = message;
  els.confirmList.innerHTML = names.map((n) => `<li>${escapeHtml(n)}</li>`).join("");
  els.confirmOk.textContent = okLabel;
  els.confirmOk.className = danger ? "text-btn danger" : "text-btn primary";

  return new Promise((resolve) => {
    const onClose = () => {
      els.confirmDialog.removeEventListener("close", onClose);
      resolve(els.confirmDialog.returnValue);
    };
    els.confirmDialog.addEventListener("close", onClose);
    els.confirmDialog.returnValue = "cancel";
    els.confirmDialog.showModal();
  });
}

async function runPackageAction(action, ids, names) {
  if (!ids.length || busy) return;
  const labels = {
    install: ["Install packages", "Install these packages?", "Install", false],
    uninstall: ["Uninstall packages", "Uninstall these packages?", "Uninstall", true],
    upgrade: ["Update packages", "Update these packages?", "Update", false],
    pin: ["Pin packages", "Pin these packages?", "Pin", false],
    unpin: ["Unpin packages", "Unpin these packages?", "Unpin", false],
  };
  const [title, message, ok, danger] = labels[action];
  const result = await confirmAction(title, message, names, ok, danger);
  if (result !== "ok") return;

  busy = true;
  for (const id of ids) statusById.set(id, { status: "queued" });
  render();
  logLine(`Starting ${action} for ${ids.length} package(s)…`);
  try {
    await invoke("run_package_action", { request: { action, ids } });
  } catch (err) {
    logLine(`Package action error: ${err}`);
  } finally {
    busy = false;
    await loadCurrentView();
  }
}

async function startProgramUninstall() {
  const targets = programs.filter((p) => selectedPrograms.has(p.id) && !p.protected);
  if (!targets.length || busy) return;
  const result = await confirmAction(
    "Confirm bulk uninstall",
    "This will remove these programs sequentially. This cannot be undone from this app.",
    targets.map((p) => p.displayName),
    "Uninstall",
    true
  );
  if (result !== "ok") return;

  busy = true;
  for (const p of targets) statusById.set(p.id, { status: "queued" });
  render();
  logLine(`Starting uninstall of ${targets.length} program(s)…`);
  try {
    await invoke("uninstall_programs", { ids: targets.map((p) => p.id) });
  } catch (err) {
    logLine(`Uninstall batch error: ${err}`);
  } finally {
    busy = false;
    await loadPrograms();
  }
}

async function setView(next) {
  if (view === next) return;
  view = next;
  selected.clear();
  sortKey = "name";
  sortDir = "asc";
  render();
  await loadCurrentView();
}

els.packageRows.addEventListener("change", (e) => {
  const input = e.target;
  if (!(input instanceof HTMLInputElement) || input.type !== "checkbox") return;
  const id = input.dataset.id;
  if (!id) return;
  if (input.checked) selected.add(id);
  else selected.delete(id);
  render();
});

els.programRows.addEventListener("change", (e) => {
  const input = e.target;
  if (!(input instanceof HTMLInputElement) || input.type !== "checkbox") return;
  const id = input.dataset.programId;
  if (!id) return;
  if (input.checked) selectedPrograms.add(id);
  else selectedPrograms.delete(id);
  render();
});

els.sourceRows.addEventListener("click", async (e) => {
  const btn = e.target.closest("[data-remove-source]");
  if (!(btn instanceof HTMLButtonElement)) return;
  const name = btn.dataset.removeSource;
  if (!name || busy) return;
  const result = await confirmAction(
    "Remove source",
    `Remove Chocolatey source “${name}”?`,
    [name],
    "Remove",
    true
  );
  if (result !== "ok") return;
  try {
    await invoke("remove_choco_source", { name });
    logLine(`Removed source ${name}.`);
    await loadSources();
  } catch (err) {
    logLine(`Failed to remove source: ${err}`);
  }
});

document.querySelectorAll(".sort-btn").forEach((btn) => {
  btn.addEventListener("click", () => {
    const key = btn.dataset.sort;
    if (!key) return;
    if (sortKey === key) sortDir = sortDir === "asc" ? "desc" : "asc";
    else {
      sortKey = key;
      sortDir = key === "size" ? "desc" : "asc";
    }
    render();
  });
});

document.querySelectorAll(".view-btn").forEach((btn) => {
  btn.addEventListener("click", () => setView(btn.dataset.view));
});

let searchTimer = null;
els.search.addEventListener("input", () => {
  if (view === "browse") {
    clearTimeout(searchTimer);
    searchTimer = setTimeout(() => loadPackages(), 350);
  } else {
    render();
  }
});

els.filterChoco.addEventListener("change", () => loadCurrentView());
els.filterWinget.addEventListener("change", () => loadCurrentView());
els.filterScoop.addEventListener("change", () => loadCurrentView());
els.showSystem.addEventListener("change", () => loadPrograms());
els.refreshBtn.addEventListener("click", async () => {
  await refreshProviderStatus();
  await loadCurrentView();
});
els.clearBtn.addEventListener("click", () => {
  if (view === "programs") selectedPrograms.clear();
  else selected.clear();
  render();
});
els.selectVisibleBtn.addEventListener("click", () => {
  if (view === "programs") {
    for (const p of visiblePrograms()) if (!p.protected) selectedPrograms.add(p.id);
  } else if (isPackageView()) {
    for (const p of visiblePackages()) selected.add(p.id);
  }
  render();
});
els.selectAll.addEventListener("change", () => {
  const visible = visiblePackages();
  if (els.selectAll.checked) for (const p of visible) selected.add(p.id);
  else for (const p of visible) selected.delete(p.id);
  render();
});
els.selectAllPrograms.addEventListener("change", () => {
  const visible = visiblePrograms().filter((p) => !p.protected);
  if (els.selectAllPrograms.checked) for (const p of visible) selectedPrograms.add(p.id);
  else for (const p of visible) selectedPrograms.delete(p.id);
  render();
});

els.installBtn.addEventListener("click", () => {
  const list = packages.filter((p) => selected.has(p.id));
  runPackageAction(
    "install",
    list.map((p) => p.id),
    list.map((p) => p.name)
  );
});
els.upgradeBtn.addEventListener("click", () => {
  const list = packages.filter((p) => selected.has(p.id));
  runPackageAction(
    "upgrade",
    list.map((p) => p.id),
    list.map((p) => p.name)
  );
});
els.upgradeAllBtn.addEventListener("click", () => {
  runPackageAction(
    "upgrade",
    packages.map((p) => p.id),
    packages.map((p) => p.name)
  );
});
els.uninstallBtn.addEventListener("click", () => {
  const list = packages.filter((p) => selected.has(p.id));
  runPackageAction(
    "uninstall",
    list.map((p) => p.id),
    list.map((p) => p.name)
  );
});
els.pinBtn.addEventListener("click", () => {
  const list = packages.filter((p) => selected.has(p.id));
  runPackageAction(
    "pin",
    list.map((p) => p.id),
    list.map((p) => p.name)
  );
});
els.unpinBtn.addEventListener("click", () => {
  const list = packages.filter((p) => selected.has(p.id));
  runPackageAction(
    "unpin",
    list.map((p) => p.id),
    list.map((p) => p.name)
  );
});
els.uninstallProgramsBtn.addEventListener("click", () => startProgramUninstall());

els.addSourceBtn.addEventListener("click", async () => {
  const name = els.sourceName.value.trim();
  const url = els.sourceUrl.value.trim();
  if (!name || !url) {
    logLine("Source name and URL are required.");
    return;
  }
  try {
    await invoke("add_choco_source", { name, url });
    els.sourceName.value = "";
    els.sourceUrl.value = "";
    logLine(`Added source ${name}.`);
    await loadSources();
  } catch (err) {
    logLine(`Failed to add source: ${err}`);
  }
});

els.bootstrapChocoBtn.addEventListener("click", async () => {
  if (busy || bootstrappingProviders) return;
  const result = await confirmAction(
    "Install Chocolatey",
    "This downloads and runs the official Chocolatey install script. Administrator rights are required.",
    ["Official bootstrap: community.chocolatey.org/install.ps1"],
    "Install",
    false
  );
  if (result !== "ok") return;
  await ensureProvidersInBackground();
});

els.bootstrapAllBtn?.addEventListener("click", async () => {
  if (busy || bootstrappingProviders) return;
  logLine("Installing any missing package managers…");
  await ensureProvidersInBackground();
});

els.openWingetBtn.addEventListener("click", async () => {
  const url = "ms-windows-store://pdp/?productid=9NBLGGH4NNS1";
  try {
    if (opener?.openUrl) await opener.openUrl(url);
    else window.open(url, "_blank");
  } catch (err) {
    logLine(`Could not open App Installer store page: ${err}`);
  }
});

els.updateAuthority?.addEventListener("change", async () => {
  await saveUpdatePrefs();
  if (view === "updates") await loadPackages();
});
els.showUpdateDuplicates?.addEventListener("change", async () => {
  await saveUpdatePrefs();
  if (view === "updates") await loadPackages();
});

els.clearLogBtn.addEventListener("click", () => {
  els.log.textContent = "";
});

await listen("bootstrap-progress", (event) => {
  const payload = event.payload;
  const label = payload.provider === "system" ? "bootstrap" : payload.provider;
  if (payload.line) {
    logLine(`[${label}] ${payload.line}`);
  } else if (payload.message) {
    logLine(`[${label}] ${payload.message}`);
  } else {
    logLine(`[${label}] ${payload.status}`);
  }
  if (payload.status === "done" || payload.status === "failed") {
    refreshProviderStatus();
  }
});

await listen("bootstrap-finished", async () => {
  logLine("Package manager bootstrap finished.");
  await refreshProviderStatus();
  if (isPackageView()) {
    await loadPackages();
  }
});

await listen("package-progress", (event) => {
  const payload = event.payload;
  statusById.set(payload.id, {
    status: payload.status,
    message: payload.message,
  });
  if (payload.line) {
    logLine(`${payload.displayName}: ${payload.line}`);
  } else {
    const detail = payload.message ? ` — ${payload.message}` : "";
    const code = payload.exitCode != null ? ` (exit ${payload.exitCode})` : "";
    logLine(`${payload.displayName}: ${payload.status}${detail}${code}`);
  }
  if (payload.status === "done" || payload.status === "uninstalled") {
    selected.delete(payload.id);
  }
  render();
});

await listen("package-finished", () => {
  logLine("Package batch finished.");
});

await listen("uninstall-progress", (event) => {
  const payload = event.payload;
  statusById.set(payload.id, {
    status: payload.status,
    message: payload.message,
  });
  const detail = payload.message ? ` — ${payload.message}` : "";
  const code = payload.exitCode != null ? ` (exit ${payload.exitCode})` : "";
  logLine(`${payload.displayName}: ${payload.status}${detail}${code}`);
  if (payload.status === "uninstalled") selectedPrograms.delete(payload.id);
  render();
});

await listen("uninstall-finished", () => {
  logLine("Program uninstall batch finished.");
});

await listen("tray:action", (event) => {
  if (event.payload === "refresh") {
    loadCurrentView();
  }
});

await refreshElevation();
await refreshProviderStatus();
await loadUpdatePrefs();
await loadCurrentView();
{
  const s = providerStatusCache;
  if (
    s &&
    (!s.chocolatey.available || !s.winget.available || !s.scoop.available)
  ) {
    void ensureProvidersInBackground();
  }
}
