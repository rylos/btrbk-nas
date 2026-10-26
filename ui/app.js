"use strict";

/* ---------- texts ---------- */

const TEXTS = {
  it: {
    refresh: "Aggiorna", settings: "Impostazioni", targets: "Cartelle di backup",
    targets_hint: "Le cartelle in cui btrbk invia gli snapshot, una per PC. Il nome è solo un'etichetta.",
    add_target: "Aggiungi", discover: "Cerca nei volumi", thresholds: "Soglie di allarme",
    warning_after: "In ritardo dopo", critical_after: "Critico dopo", hours: "ore",
    cancel: "Annulla", save: "Salva", remove: "Rimuovi", label: "Nome", path: "Percorso",
    backups: "Backup", status_ok: "Aggiornato", status_warning: "In ritardo", status_critical: "Critico",
    last_backup: "Ultimo backup", snapshots: "Snapshot", oldest: "Più vecchio", incomplete: "Incompleti",
    exclusive: "Spazio esclusivo", never: "mai", no_targets_title: "Nessuna cartella di backup",
    no_targets: "Non ho trovato cartelle con snapshot btrbk. Aggiungine una dalle impostazioni.",
    open_settings: "Apri impostazioni", history: "Ultimi 30 giorni", subvolumes: "Subvolume",
    legend_ok: "snapshot ricevuto", legend_bad: "solo ricezioni incomplete", legend_none: "nessuno",
    col_snapshot: "Snapshot", col_taken: "Creato sul PC", col_received: "Ricevuto", col_duration: "Durata",
    col_state: "Stato", col_uuid: "Received UUID", browse: "Sfoglia", state_ok: "Completo",
    state_incomplete: "Ricezione incompleta", state_not_subvolume: "Non è un subvolume",
    state_unknown: "Sconosciuto", col_name: "Nome", col_size: "Dimensione", col_modified: "Modificato",
    col_mode: "Permessi", col_owner: "UID:GID", download: "Scarica", download_dir: "Scarica cartella (.tar)",
    filter: "Filtra…", empty_dir: "Cartella vuota", truncated: "Elenco troncato a 10.000 voci.",
    session_expired: "Sessione DSM scaduta", session_hint: "Accedi di nuovo a DSM e riapri l'app.",
    forbidden: "Accesso negato", forbidden_hint: "Questa app è riservata agli amministratori.",
    error: "Errore", saved: "Impostazioni salvate", discovered: "Trovate {n} cartelle nuove",
    discovered_none: "Nessuna cartella nuova trovata",
    incomplete_banner: "{n} snapshot con ricezione incompleta: btrbk non li usa come base e li ricrea al prossimo invio. Si possono cancellare.",
    late_banner: "L'ultimo backup completo di {s} risale a {t}.",
    no_backup_banner: "{s} non ha nessun backup completo.",
    overview: "Panoramica", ago: "fa",
    target_notify: "Notifiche per questo PC", global: "generale",
    notifications: "Notifiche", n_enabled: "Controlla i backup e invia notifiche",
    every: "ogni", minutes: "minuti", events: "Eventi",
    ev_late: "In ritardo", ev_critical: "Critico", ev_recovered: "Di nuovo aggiornato",
    ev_incomplete: "Ricezione incompleta", ev_errors: "Cartella non leggibile",
    reminder: "Ripeti finché il problema resta, ogni", reminder_hint: "0 = una volta sola",
    channels: "Canali", ch_desktop: "Desktop DSM (campanella), destinatario",
    ch_desktop_hint: "@administrators = tutti gli amministratori; oppure un utente (alice) o un gruppo (@gruppo). Sul desktop il testo è generico: i dettagli sono nell'app.",
    ch_webhook: "Webhook, con il messaggio completo", webhook_url: "URL",
    webhook_body: "Corpo JSON (facoltativo: vuoto = GET)", language: "Lingua",
    webhook_hint: "Segnaposto {title} e {message}. Telegram: https://api.telegram.org/bot<TOKEN>/sendMessage?chat_id=<ID>&text={message} · ntfy: corpo {\"topic\":\"<topic>\",\"title\":\"{title}\",\"message\":\"{message}\"} verso https://ntfy.sh · Discord: {\"content\":\"{message}\"}",
    send_test: "Invia notifica di prova", test_sent: "Notifica di prova inviata",
    monitor_on: "Controllo attivo ogni {m} min, ultimo {last}.",
    monitor_off: "Controllo spento dalle impostazioni.",
    monitor_stopped: "Controllo fermo: avvia btrbk Backups dal Centro pacchetti.",
    next_send: "Prossimo invio", incremental_from: "incrementale da {s}", full_send: "completo",
    pending: "Da inviare", on_pc: "Sul PC", next_parent: "prossimo parent", yes: "sì",
    source_line: "Snapshot del PC {h} aggiornati {t}.",
    source_missing: "Il PC non invia l'elenco dei suoi snapshot: il prossimo parent non è noto (vedi extras/pc nel README).",
    source_error: "Elenco degli snapshot del PC non leggibile: {e}",
    full_send_banner: "{s}: nessuno snapshot in comune con {h}, il prossimo invio di btrbk sarà completo.",
    ev_full_send: "Prossimo invio completo",
    delete: "Elimina", delete_old: "Elimina i vecchi ({n})", queue_title: "Coda eliminazioni",
    q_waiting: "parte tra {s} s", q_deleting: "eliminazione…", q_done: "eliminato", q_error: "errore",
    cancel_all: "Annulla tutto", clear: "Chiudi", undo: "Annulla", deletable: "Eliminabili",
    leave_warning: "Ci sono eliminazioni in coda.",
    del_parent: "È il prossimo parent: eliminarlo forzerebbe un invio completo.",
    del_newer: "Più recente del parent: btrbk può ancora averne bisogno.",
    del_unknown: "Il PC non invia l'elenco dei suoi snapshot: il parent non è noto.",
    del_no_common: "Nessuno snapshot in comune con il PC: la catena è già rotta, non si tocca.",
    del_receiving: "Ricezione incompleta di meno di un giorno: potrebbe essere ancora in corso.",
    del_not_subvolume: "Non è un subvolume.",
    caps_title: "Permessi da attivare.",
    caps_text: "Dopo ogni installazione o aggiornamento del pacchetto, esegui una volta via SSH:",
    not_subvolume_banner: "{n} cartelle hanno il nome di uno snapshot ma non sono subvolume: btrbk non le riconosce, vanno controllate a mano.",
  },
  en: {
    refresh: "Refresh", settings: "Settings", targets: "Backup folders",
    targets_hint: "The folders btrbk sends snapshots into, one per PC. The name is just a label.",
    add_target: "Add", discover: "Search volumes", thresholds: "Alert thresholds",
    warning_after: "Late after", critical_after: "Critical after", hours: "hours",
    cancel: "Cancel", save: "Save", remove: "Remove", label: "Name", path: "Path",
    backups: "Backups", status_ok: "Up to date", status_warning: "Late", status_critical: "Critical",
    last_backup: "Last backup", snapshots: "Snapshots", oldest: "Oldest", incomplete: "Incomplete",
    exclusive: "Exclusive space", never: "never", no_targets_title: "No backup folders",
    no_targets: "No folder with btrbk snapshots was found. Add one in the settings.",
    open_settings: "Open settings", history: "Last 30 days", subvolumes: "Subvolumes",
    legend_ok: "snapshot received", legend_bad: "incomplete receives only", legend_none: "none",
    col_snapshot: "Snapshot", col_taken: "Taken on the PC", col_received: "Received", col_duration: "Duration",
    col_state: "State", col_uuid: "Received UUID", browse: "Browse", state_ok: "Complete",
    state_incomplete: "Incomplete receive", state_not_subvolume: "Not a subvolume",
    state_unknown: "Unknown", col_name: "Name", col_size: "Size", col_modified: "Modified",
    col_mode: "Mode", col_owner: "UID:GID", download: "Download", download_dir: "Download folder (.tar)",
    filter: "Filter…", empty_dir: "Empty folder", truncated: "Listing cut at 10,000 entries.",
    session_expired: "DSM session expired", session_hint: "Sign in to DSM again and reopen the app.",
    forbidden: "Access denied", forbidden_hint: "This app is for administrators only.",
    error: "Error", saved: "Settings saved", discovered: "Found {n} new folders",
    discovered_none: "No new folder found",
    incomplete_banner: "{n} snapshots with an incomplete receive: btrbk does not use them as a base and receives them again next time. They can be deleted.",
    late_banner: "The last complete backup of {s} is from {t}.",
    no_backup_banner: "{s} has no complete backup.",
    overview: "Overview", ago: "ago",
    target_notify: "Notifications for this PC", global: "global",
    notifications: "Notifications", n_enabled: "Check the backups and send notifications",
    every: "every", minutes: "minutes", events: "Events",
    ev_late: "Late", ev_critical: "Critical", ev_recovered: "Up to date again",
    ev_incomplete: "Incomplete receive", ev_errors: "Folder unreadable",
    reminder: "Repeat while the problem lasts, every", reminder_hint: "0 = once only",
    channels: "Channels", ch_desktop: "DSM desktop (bell), recipient",
    ch_desktop_hint: "@administrators = every administrator; or a user (alice) or a group (@group). The desktop text is generic: the details are in the app.",
    ch_webhook: "Webhook, with the full message", webhook_url: "URL",
    webhook_body: "JSON body (optional: empty = GET)", language: "Language",
    webhook_hint: "Placeholders {title} and {message}. Telegram: https://api.telegram.org/bot<TOKEN>/sendMessage?chat_id=<ID>&text={message} · ntfy: body {\"topic\":\"<topic>\",\"title\":\"{title}\",\"message\":\"{message}\"} to https://ntfy.sh · Discord: {\"content\":\"{message}\"}",
    send_test: "Send a test notification", test_sent: "Test notification sent",
    monitor_on: "Check running every {m} min, last {last}.",
    monitor_off: "Check switched off in the settings.",
    monitor_stopped: "Check stopped: start btrbk Backups in Package Center.",
    next_send: "Next send", incremental_from: "incremental from {s}", full_send: "full",
    pending: "To send", on_pc: "On the PC", next_parent: "next parent", yes: "yes",
    source_line: "Snapshots of the PC {h} updated {t}.",
    source_missing: "The PC does not send the list of its snapshots: the next parent is unknown (see extras/pc in the README).",
    source_error: "The PC's snapshot list cannot be read: {e}",
    full_send_banner: "{s}: no snapshot in common with {h}, btrbk's next send will be a full one.",
    ev_full_send: "Next send is a full one",
    delete: "Delete", delete_old: "Delete the old ones ({n})", queue_title: "Deletion queue",
    q_waiting: "starts in {s} s", q_deleting: "deleting…", q_done: "deleted", q_error: "error",
    cancel_all: "Cancel all", clear: "Close", undo: "Undo", deletable: "Deletable",
    leave_warning: "Deletions are queued.",
    del_parent: "The next parent: deleting it would force a full send.",
    del_newer: "Newer than the parent: btrbk may still need it.",
    del_unknown: "The PC does not send the list of its snapshots: the parent is unknown.",
    del_no_common: "Nothing in common with the PC: the chain is already broken, left alone.",
    del_receiving: "Interrupted receive less than a day old: it may still be running.",
    del_not_subvolume: "Not a subvolume.",
    caps_title: "Permissions to enable.",
    caps_text: "After every install or upgrade of the package, run once over SSH:",
    not_subvolume_banner: "{n} folders are named like snapshots but are not subvolumes: btrbk does not recognise them, check them by hand.",
  },
};
const LANG = (navigator.language || "en").toLowerCase().startsWith("it") ? "it" : "en";
const T = TEXTS[LANG];
// placeholders without a value stay as they are: {title} in the webhook help
const t = (key, vars = {}) => (T[key] || key).replace(/\{(\w+)\}/g, (all, k) => vars[k] ?? all);

/* ---------- helpers ---------- */

function h(tag, attrs = {}, ...children) {
  const el = document.createElement(tag);
  for (const [key, value] of Object.entries(attrs || {})) {
    if (value === null || value === undefined || value === false) continue;
    if (key.startsWith("on")) el.addEventListener(key.slice(2), value);
    else if (key === "class") el.className = value;
    else if (key === "html") el.innerHTML = value; // only for static SVG markup
    else el.setAttribute(key, value === true ? "" : value);
  }
  for (const child of children.flat(Infinity)) {
    if (child === null || child === undefined || child === false) continue;
    el.append(child instanceof Node ? child : document.createTextNode(String(child)));
  }
  return el;
}

const ICONS = {
  folder: '<svg viewBox="0 0 24 24"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/></svg>',
  file: '<svg viewBox="0 0 24 24"><path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z"/><path d="M14 3v5h5"/></svg>',
  link: '<svg viewBox="0 0 24 24"><path d="M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1"/><path d="M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1"/></svg>',
  other: '<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="8"/></svg>',
  download: '<svg viewBox="0 0 24 24"><path d="M12 4v11m0 0-4-4m4 4 4-4M5 19h14"/></svg>',
  browse: '<svg viewBox="0 0 24 24"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/></svg>',
  back: '<svg viewBox="0 0 24 24"><path d="M15 18l-6-6 6-6"/></svg>',
  trash: '<svg viewBox="0 0 24 24"><path d="M4 7h16M10 11v6M14 11v6M5 7l1 13h12l1-13M9 7V4h6v3"/></svg>',
  lock: '<svg viewBox="0 0 24 24"><rect x="5" y="11" width="14" height="9" rx="2"/><path d="M8 11V8a4 4 0 0 1 8 0v3"/></svg>',
};
const icon = (name) => h("span", { html: ICONS[name], style: "display:contents" });

const dateFmt = new Intl.DateTimeFormat(LANG, { dateStyle: "medium", timeStyle: "short" });
const dayFmt = new Intl.DateTimeFormat(LANG, { day: "numeric" });
const rtf = new Intl.RelativeTimeFormat(LANG, { numeric: "auto" });

function formatEpoch(seconds) {
  return seconds ? dateFmt.format(new Date(seconds * 1000)) : "—";
}

function relative(seconds, now) {
  if (!seconds) return t("never");
  const diff = seconds - now;
  const abs = Math.abs(diff);
  if (abs < 3600) return rtf.format(Math.round(diff / 60), "minute");
  if (abs < 48 * 3600) return rtf.format(Math.round(diff / 3600), "hour");
  return rtf.format(Math.round(diff / 86400), "day");
}

function duration(seconds) {
  if (seconds === null || seconds === undefined || seconds < 0) return "—";
  if (seconds < 60) return `${seconds} s`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)} min ${seconds % 60} s`;
  return `${Math.floor(seconds / 3600)} h ${Math.floor((seconds % 3600) / 60)} min`;
}

function bytes(n) {
  if (n === null || n === undefined) return "—";
  const units = ["B", "KiB", "MiB", "GiB", "TiB"];
  let i = 0;
  while (n >= 1024 && i < units.length - 1) { n /= 1024; i++; }
  return `${n.toFixed(i === 0 ? 0 : n < 10 ? 2 : 1)} ${units[i]}`;
}

/** "20261026T0847" -> Date in local time, or null. */
function parseStamp(stamp) {
  const m = /^(\d{4})(\d{2})(\d{2})(?:[T_](\d{2})(\d{2})(\d{2})?)?/.exec(stamp || "");
  if (!m) return null;
  return new Date(+m[1], +m[2] - 1, +m[3], +(m[4] || 0), +(m[5] || 0), +(m[6] || 0));
}

function modeString(mode, kind) {
  const type = kind === "dir" ? "d" : kind === "symlink" ? "l" : "-";
  const bits = "rwxrwxrwx";
  let out = "";
  for (let i = 0; i < 9; i++) out += mode & (1 << (8 - i)) ? bits[i] : "-";
  return type + out;
}

function statusPill(status) {
  return h("span", { class: `pill ${status}` }, h("span", { class: `dot ${status}` }), t(`status_${status}`));
}

function toast(message, isError = false) {
  const el = h("div", { class: `toast${isError ? " error" : ""}` }, message);
  document.getElementById("toasts").append(el);
  setTimeout(() => el.remove(), isError ? 8000 : 3500);
}

/* ---------- API ---------- */

let synoToken = "";

async function loadSynoToken() {
  // DSM hands the CSRF token of the current session to its own pages
  try {
    const res = await fetch("/webman/login.cgi", { credentials: "same-origin" });
    const data = await res.json();
    synoToken = data.SynoToken || "";
  } catch {
    synoToken = "";
  }
}

class ApiError extends Error {
  constructor(status, message) { super(message); this.status = status; }
}

function apiUrl(route, params = {}) {
  const query = new URLSearchParams(params);
  if (synoToken) query.set("SynoToken", synoToken);
  return `api.cgi/${route}?${query}`;
}

async function api(route, params = {}, body = undefined) {
  const options = { credentials: "same-origin", headers: {} };
  if (synoToken) options.headers["X-SYNO-TOKEN"] = synoToken;
  if (body !== undefined) {
    options.method = "POST";
    options.headers["Content-Type"] = "application/json";
    options.body = JSON.stringify(body);
  }
  const res = await fetch(apiUrl(route, params), options);
  let data = null;
  try { data = await res.json(); } catch { /* empty or not JSON */ }
  if (!res.ok) throw new ApiError(res.status, (data && data.error) || `HTTP ${res.status}`);
  return data;
}

/* ---------- state and routing ---------- */

const state = {
  overview: null,
  detail: null,   // target detail, cached for the current target
  detailId: null,
  group: null,    // selected subvolume
};

/** #/t/<id>[/g/<subvolume>][/b/<snapshot>[/<path>]] */
function parseHash() {
  const parts = location.hash.replace(/^#\/?/, "").split("/").map(decodeURIComponent);
  const route = { target: null, group: null, snapshot: null, path: "" };
  for (let i = 0; i < parts.length; i++) {
    if (parts[i] === "t") route.target = Number(parts[++i]);
    else if (parts[i] === "g") route.group = parts[++i];
    else if (parts[i] === "b") {
      route.snapshot = parts[++i];
      route.path = parts.slice(i + 1).join("/");
      break;
    }
  }
  return route;
}

function go(route) {
  let hash = "#/";
  if (route.target !== null && route.target !== undefined) hash += `t/${route.target}`;
  if (route.group) hash += `/g/${encodeURIComponent(route.group)}`;
  if (route.snapshot) {
    hash += `/b/${encodeURIComponent(route.snapshot)}`;
    if (route.path) hash += "/" + route.path.split("/").filter(Boolean).map(encodeURIComponent).join("/");
  }
  if (location.hash === hash) render();
  else location.hash = hash;
}

const content = () => document.getElementById("content");

/** Replaces the page; nested arrays and empty values are allowed. */
function setContent(...items) {
  content().replaceChildren(...items.flat(Infinity).filter((x) => x !== null && x !== undefined && x !== false));
}

function showPlaceholder(title, text, action) {
  content().replaceChildren(
    h("div", { class: "placeholder" }, h("div", {}, h("h2", {}, title), h("p", {}, text), action || null)),
  );
}

function showLoading() {
  content().replaceChildren(h("div", { class: "placeholder" }, h("div", { class: "spinner" })));
}

function showError(err) {
  if (err.status === 401) return showPlaceholder(t("session_expired"), t("session_hint"));
  if (err.status === 403) return showPlaceholder(t("forbidden"), t("forbidden_hint"));
  showPlaceholder(t("error"), err.message);
}

/* ---------- sidebar ---------- */

function renderSidebar(route) {
  const side = document.getElementById("sidebar");
  const ov = state.overview;
  if (!ov) return side.replaceChildren();
  side.replaceChildren(
    h("div", { class: "side-title" }, t("backups")),
    ...ov.targets.map((target) =>
      h("button", {
        class: `side-item${route.target === target.id ? " active" : ""}`,
        onclick: () => go({ target: target.id }),
      },
      h("span", { class: `dot ${target.status}` }),
      h("div", {},
        h("div", { class: "name" }, target.label),
        h("div", { class: "sub" }, target.error ? t("error") : relative(target.last_received, ov.now)),
      )),
    ),
  );
}

/* ---------- target page ---------- */

function groupCard(summary, now, selected, onclick) {
  return h("div", { class: `card clickable${selected ? " selected" : ""}`, onclick },
    h("div", { class: "card-head" },
      h("span", { class: "card-title", title: summary.subvolume }, summary.subvolume),
      statusPill(summary.status)),
    h("div", { class: "muted" }, t("last_backup")),
    h("div", { class: "big" }, relative(summary.last_received, now)),
    h("div", { class: "muted" }, formatEpoch(summary.last_received)),
    h("dl", { class: "facts" },
      h("dt", {}, t("snapshots")), h("dd", {}, summary.count),
      h("dt", {}, t("oldest")), h("dd", {}, summary.oldest ? formatEpochOrStamp(summary.oldest) : "—"),
      summary.incomplete ? [h("dt", {}, t("incomplete")), h("dd", { class: "warn" }, summary.incomplete)] : null,
      summary.chain ? [
        h("dt", {}, t("next_send")),
        summary.chain.full_send
          ? h("dd", { class: "crit" }, t("full_send"))
          : h("dd", { title: summary.chain.next_parent }, t("incremental_from", { s: formatEpochOrStamp(summary.chain.next_parent) })),
        summary.chain.pending.length
          ? [h("dt", {}, t("pending")), h("dd", { title: summary.chain.pending.join("\n") }, summary.chain.pending.length)]
          : null,
      ] : null,
      summary.exclusive_bytes !== null && summary.exclusive_bytes !== undefined
        ? [h("dt", {}, t("exclusive")), h("dd", {}, bytes(summary.exclusive_bytes))] : null,
    ));
}

function formatEpochOrStamp(name) {
  const stamp = name.slice(name.lastIndexOf(".") + 1);
  const date = parseStamp(stamp);
  return date ? dateFmt.format(date) : stamp;
}

function timeline(groups) {
  const days = [];
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  for (let i = 29; i >= 0; i--) days.push(new Date(today.getTime() - i * 86400000));
  const key = (d) => `${d.getFullYear()}${String(d.getMonth() + 1).padStart(2, "0")}${String(d.getDate()).padStart(2, "0")}`;

  const grid = h("div", { class: "timeline-grid", style: `grid-template-columns: max-content repeat(${days.length}, minmax(14px, 1fr))` });
  for (const group of groups) {
    const byDay = new Map();
    for (const snap of group.snapshots) {
      if (snap.problem === "not_subvolume") continue;
      const k = (snap.timestamp || "").slice(0, 8);
      const ok = !snap.problem;
      byDay.set(k, byDay.get(k) || ok);
    }
    grid.append(h("div", { class: "label" }, group.summary.subvolume));
    for (const day of days) {
      const k = key(day);
      const cls = byDay.has(k) ? (byDay.get(k) ? "has" : "bad") : "";
      grid.append(h("div", { class: `cell ${cls}`, title: dateFmt.format(day).split(",")[0] }));
    }
  }
  grid.append(h("div", {}));
  for (const day of days) grid.append(h("div", { class: "day" }, dayFmt.format(day)));
  return h("div", { class: "card timeline" }, grid,
    h("div", { class: "legend" },
      h("span", {}, h("i", { style: "background:var(--ok)" }), t("legend_ok")),
      h("span", {}, h("i", { style: "background:var(--warn)" }), t("legend_bad")),
      h("span", {}, h("i", { style: "background:var(--surface-2);border:1px solid var(--border)" }), t("legend_none")),
    ));
}

function snapshotState(snap) {
  if (!snap.problem && snap.info) return h("span", { class: "pill ok" }, t("state_ok"));
  if (snap.problem === "incomplete") return h("span", { class: "pill warning" }, t("state_incomplete"));
  if (snap.problem === "not_subvolume") return h("span", { class: "pill critical" }, t("state_not_subvolume"));
  return h("span", { class: "pill neutral" }, t("state_unknown"));
}

function snapshotTable(targetId, group) {
  const hasUsage = group.snapshots.some((s) => s.usage);
  const hasSource = group.snapshots.some((s) => s.on_source !== null && s.on_source !== undefined);
  const parent = group.summary.chain && group.summary.chain.next_parent;
  return h("div", { class: "table-wrap" }, h("table", {},
    h("thead", {}, h("tr", {},
      h("th", {}, t("col_snapshot")), h("th", {}, t("col_taken")), h("th", {}, t("col_received")),
      h("th", {}, t("col_state")),
      hasSource ? h("th", {}, t("on_pc")) : null,
      hasUsage ? h("th", {}, t("exclusive")) : null,
      h("th", {}, t("col_uuid")), h("th", { class: "actions" }))),
    h("tbody", {}, group.snapshots.map((snap) => {
      const info = snap.info || {};
      const taken = parseStamp(snap.timestamp);
      return h("tr", { "data-target": targetId, "data-name": snap.name, "data-deletion": snap.deletion },
        h("td", { class: "mono" }, snap.name,
          snap.name === parent ? h("span", { class: "pill accent tag" }, t("next_parent")) : null),
        h("td", {}, taken ? dateFmt.format(taken) : "—"),
        h("td", { title: info.rtime && info.otime ? `${t("col_duration")}: ${duration(info.rtime - info.otime)}` : "" },
          formatEpoch(info.rtime || info.otime)),
        h("td", {}, snapshotState(snap)),
        hasSource ? h("td", {}, snap.on_source ? h("span", { class: "pill ok" }, t("yes")) : "—") : null,
        hasUsage ? h("td", { class: "num" }, bytes(snap.usage && snap.usage.exclusive)) : null,
        h("td", { class: "mono", title: info.received_uuid || "" }, info.received_uuid ? info.received_uuid.slice(0, 8) + "…" : "—"),
        h("td", { class: "actions" },
          snap.problem === "not_subvolume" ? null : h("button", {
            class: "btn small",
            onclick: () => go({ target: targetId, group: group.summary.subvolume, snapshot: snap.name }),
          }, icon("browse"), t("browse")),
          h("span", { class: "del-cell" }, deleteControl(targetId, snap))));
    }))));
}

/* ---------- deletion queue ---------- */

// Clicks queue deletions without a dialog; each one waits GRACE_MS, during
// which it can be undone, then they run one at a time. The NAS checks every
// deletion again against the chain, whatever this page shows.
const GRACE_MS = 5000;
const queue = [];
let queueRunning = false;
let queueTimer = null;

const queueKey = (target, name) => `${target}|${name}`;
const queued = (target, name) => queue.find((q) => q.key === queueKey(target, name) && q.status !== "cancelled");
const allowedDeletion = (snap) => snap.deletion === "allowed" || snap.deletion === "allowed_incomplete";

const DELETION_REASON = {
  parent: "del_parent", newer: "del_newer", parent_unknown: "del_unknown",
  no_common_parent: "del_no_common", receiving: "del_receiving", not_subvolume: "del_not_subvolume",
};

function deleteControl(targetId, snap) {
  const item = queued(targetId, snap.name);
  if (item) return queueBadge(item);
  if (!allowedDeletion(snap)) {
    const reason = DELETION_REASON[snap.deletion];
    return h("span", { class: "lock", title: reason ? t(reason) : "" }, icon("lock"));
  }
  return h("button", {
    class: "btn small danger", title: t("delete"),
    onclick: () => enqueue(targetId, snap.name),
  }, icon("trash"));
}

function queueBadge(item) {
  const cls = { waiting: "neutral", deleting: "accent", done: "ok", error: "critical" }[item.status];
  const text = item.status === "waiting"
    ? t("q_waiting", { s: Math.max(1, Math.ceil((item.due - Date.now()) / 1000)) })
    : t(`q_${item.status}`);
  return h("span", { class: `pill ${cls}`, title: item.error || "" }, text);
}

function enqueue(target, name) {
  if (queued(target, name)) return;
  queue.push({ key: queueKey(target, name), target, name, status: "waiting", due: Date.now() + GRACE_MS });
  refreshQueueViews();
  pumpQueue();
}

function cancelItem(item) {
  if (item.status === "waiting") item.status = "cancelled";
  refreshQueueViews();
}

async function pumpQueue() {
  if (queueRunning) return;
  clearTimeout(queueTimer);
  const waiting = queue.filter((q) => q.status === "waiting");
  if (!waiting.length) return;
  const next = waiting.find((q) => q.due <= Date.now());
  if (!next) {
    // tick every second: the countdowns are on screen
    queueTimer = setTimeout(() => { refreshQueueViews(); pumpQueue(); }, 1000);
    return;
  }
  queueRunning = true;
  next.status = "deleting";
  refreshQueueViews();
  try {
    await api("delete", {}, { id: next.target, snapshot: next.name });
    next.status = "done";
  } catch (err) {
    next.status = "error";
    next.error = err.message;
    toast(`${next.name}: ${err.message}`, true);
  }
  queueRunning = false;
  refreshQueueViews();
  if (queue.some((q) => q.status === "waiting")) {
    pumpQueue();
  } else {
    // the queue is drained: show the target as it is now
    state.detail = null;
    await refresh();
  }
}

/** Updates the delete cells of the table on screen and the queue panel. */
function refreshQueueViews() {
  for (const row of document.querySelectorAll("tr[data-name]")) {
    const cell = row.querySelector(".del-cell");
    const snap = { name: row.dataset.name, deletion: row.dataset.deletion };
    if (cell) cell.replaceChildren(deleteControl(Number(row.dataset.target), snap));
  }
  const bulk = document.getElementById("bulk-delete");
  if (bulk && bulkContext) bulk.replaceWith(bulkDelete(bulkContext.targetId, bulkContext.group));
  renderQueuePanel();
}

function renderQueuePanel() {
  let panel = document.getElementById("queue");
  const visible = queue.filter((q) => q.status !== "cancelled");
  if (!visible.length) {
    if (panel) panel.remove();
    return;
  }
  if (!panel) {
    panel = h("div", { id: "queue", class: "queue" });
    document.body.append(panel);
  }
  const busy = visible.some((q) => q.status === "waiting" || q.status === "deleting");
  panel.replaceChildren(
    h("div", { class: "queue-head" },
      h("strong", {}, t("queue_title")),
      busy
        ? h("button", { class: "btn small", onclick: () => { visible.forEach(cancelItem); } }, t("cancel_all"))
        : h("button", { class: "btn small", onclick: () => { queue.length = 0; renderQueuePanel(); } }, t("clear"))),
    h("div", { class: "queue-list" }, visible.map((item) =>
      h("div", { class: "queue-item" },
        h("span", { class: "mono" }, item.name),
        queueBadge(item),
        item.status === "waiting" ? h("button", { class: "btn small", onclick: () => cancelItem(item) }, t("undo")) : null))));
}

window.addEventListener("beforeunload", (ev) => {
  if (queue.some((q) => q.status === "waiting" || q.status === "deleting")) {
    ev.preventDefault();
    ev.returnValue = t("leave_warning");
  }
});

let bulkContext = null;

function bulkDelete(targetId, group) {
  bulkContext = { targetId, group };
  const old = group.snapshots.filter((s) => allowedDeletion(s) && !queued(targetId, s.name));
  if (!old.length) return h("div", { id: "bulk-delete" });
  // oldest first: if the queue is stopped half way, what is left is the newest
  const names = old.map((s) => s.name).reverse();
  return h("div", { id: "bulk-delete", class: "toolbar" }, h("span"),
    h("button", {
      class: "btn danger",
      onclick: () => names.forEach((n) => enqueue(targetId, n)),
    },
      icon("trash"), t("delete_old", { n: old.length })));
}

const SETUP_COMMAND = "sudo /var/packages/btrbk-nas/target/bin/btrbk-nas setup";

function banners(detail, now) {
  const out = [];
  if (state.overview && state.overview.capabilities === false) {
    out.push(h("div", { class: "banner critical" },
      h("strong", {}, t("caps_title")), " ", t("caps_text"),
      h("pre", { class: "command" }, SETUP_COMMAND)));
  }
  if (detail.error) out.push(h("div", { class: "banner critical" }, detail.error));
  for (const group of detail.groups) {
    const s = group.summary;
    if (!s.last_received) out.push(h("div", { class: "banner critical" }, t("no_backup_banner", { s: s.subvolume })));
    else if (s.status !== "ok")
      out.push(h("div", { class: `banner ${s.status}` }, t("late_banner", { s: s.subvolume, t: relative(s.last_received, now) })));
    if (s.incomplete) out.push(h("div", { class: "banner warning" }, `${s.subvolume}: ${t("incomplete_banner", { n: s.incomplete })}`));
    if (s.chain && s.chain.full_send && detail.source) {
      out.push(h("div", { class: "banner warning" }, t("full_send_banner", { s: s.subvolume, h: detail.source.host })));
    }
    if (s.not_subvolume) out.push(h("div", { class: "banner warning" }, `${s.subvolume}: ${t("not_subvolume_banner", { n: s.not_subvolume })}`));
  }
  return out;
}

function sourceLine(detail, now) {
  const source = detail.source;
  if (!detail.groups.length) return null;
  if (!source) return h("div", { class: "source-line muted-line" }, t("source_missing"));
  if (source.error) return h("div", { class: "source-line warn-line" }, t("source_error", { e: source.error }));
  return h("div", { class: "source-line", title: formatEpoch(source.generated) },
    t("source_line", { h: source.host, t: relative(source.generated, now) }));
}

async function loadDetail(id, force = false) {
  if (!force && state.detailId === id && state.detail) return state.detail;
  state.detail = await api("target", { id });
  state.detailId = id;
  return state.detail;
}

async function renderTarget(route) {
  const ov = state.overview;
  const target = ov.targets.find((x) => x.id === route.target);
  if (!target) return go({ target: ov.targets[0].id });
  showLoading();
  const detail = await loadDetail(route.target);
  const groupName = route.group && detail.groups.some((g) => g.summary.subvolume === route.group)
    ? route.group : detail.groups[0] && detail.groups[0].summary.subvolume;
  const group = detail.groups.find((g) => g.summary.subvolume === groupName);

  setContent(
    h("div", { class: "page-head" },
      h("div", {}, h("h1", {}, detail.label), h("div", { class: "path" }, detail.path), sourceLine(detail, ov.now)),
      statusPill(target.status)),
    ...banners(detail, ov.now),
    h("div", { class: "cards" }, detail.groups.map((g) =>
      groupCard(g.summary, ov.now, g.summary.subvolume === groupName,
        () => go({ target: route.target, group: g.summary.subvolume })))),
    detail.groups.length ? [h("h2", { class: "section" }, t("history")), timeline(detail.groups)] : null,
    group ? [
      h("h2", { class: "section" }, t("snapshots")),
      h("div", { class: "tabs" }, detail.groups.map((g) =>
        h("button", {
          class: `tab${g.summary.subvolume === groupName ? " active" : ""}`,
          onclick: () => go({ target: route.target, group: g.summary.subvolume }),
        }, `${g.summary.subvolume} (${g.summary.count})`))),
      bulkDelete(route.target, group),
      snapshotTable(route.target, group),
    ] : null,
  );
}

/* ---------- file browser ---------- */

async function renderBrowse(route) {
  showLoading();
  const detail = await loadDetail(route.target);
  const listing = await api("browse", { id: route.target, snapshot: route.snapshot, path: route.path });
  const parts = route.path.split("/").filter(Boolean);
  const at = (n) => ({ ...route, path: parts.slice(0, n).join("/") });

  const crumbs = h("div", { class: "crumbs" },
    h("button", { onclick: () => go({ target: route.target, group: route.group }) }, detail.label),
    h("span", { class: "sep" }, "›"),
    parts.length ? h("button", { onclick: () => go(at(0)) }, route.snapshot) : h("span", { class: "current" }, route.snapshot),
    parts.map((part, i) => [
      h("span", { class: "sep" }, "/"),
      i === parts.length - 1 ? h("span", { class: "current" }, part) : h("button", { onclick: () => go(at(i + 1)) }, part),
    ]));

  const download = (path) => apiUrl("download", { id: route.target, snapshot: route.snapshot, path });
  const join = (name) => (route.path ? `${route.path}/${name}` : name);

  const tbody = h("tbody");
  const fill = (filter) => {
    const f = filter.toLowerCase();
    const rows = listing.entries.filter((e) => !f || e.name.toLowerCase().includes(f)).map((e) => {
      const open = e.kind === "dir" ? () => go({ ...route, path: join(e.name) }) : null;
      return h("tr", { class: open ? "entry" : "", onclick: open },
        h("td", { class: "name-cell" },
          h("span", { class: `entry-name ${e.kind}` },
            icon(e.kind === "dir" ? "folder" : e.kind === "symlink" ? "link" : e.kind === "file" ? "file" : "other"),
            e.name,
            e.link ? h("span", { class: "link-target" }, ` → ${e.link}`) : null)),
        h("td", { class: "num" }, e.kind === "file" ? bytes(e.size) : ""),
        h("td", {}, formatEpoch(e.mtime)),
        h("td", { class: "mono" }, modeString(e.mode, e.kind)),
        h("td", { class: "mono" }, `${e.uid}:${e.gid}`),
        h("td", { class: "actions" },
          e.kind === "file" || e.kind === "dir"
            ? h("a", {
              class: "btn small", href: download(join(e.name)), download: "",
              title: e.kind === "dir" ? t("download_dir") : t("download"),
              onclick: (ev) => ev.stopPropagation(),
            }, icon("download"), e.kind === "dir" ? ".tar" : "")
            : null));
    });
    tbody.replaceChildren(...(rows.length ? rows : [h("tr", {}, h("td", { colspan: 6, class: "muted" }, t("empty_dir")))]));
  };
  fill("");

  setContent(
    h("div", { class: "page-head" },
      h("div", {}, h("h1", { class: "mono" }, route.snapshot), h("div", { class: "path" }, `${detail.path}/${route.snapshot}`)),
      h("button", { class: "btn", onclick: () => go({ target: route.target, group: route.group }) }, icon("back"), t("snapshots"))),
    crumbs,
    h("div", { class: "toolbar" },
      h("input", { class: "filter", type: "search", placeholder: t("filter"), oninput: (ev) => fill(ev.target.value) }),
      h("a", { class: "btn", href: download(route.path), download: "" }, icon("download"), t("download_dir"))),
    listing.truncated ? h("div", { class: "banner warning" }, t("truncated")) : null,
    h("div", { class: "table-wrap" }, h("table", {},
      h("thead", {}, h("tr", {},
        h("th", {}, t("col_name")), h("th", {}, t("col_size")), h("th", {}, t("col_modified")),
        h("th", {}, t("col_mode")), h("th", {}, t("col_owner")), h("th", { class: "actions" }))),
      tbody)),
  );
}

/* ---------- settings ---------- */

const dialog = () => document.getElementById("settings");
const $ = (id) => document.getElementById(id);

function numberOrNull(input) {
  const value = input.value.trim();
  return value === "" ? null : Number(value);
}

function targetRow(target = { label: "", path: "", notify: true }) {
  const row = h("div", { class: "target-card" },
    h("div", { class: "target-row" },
      h("input", { class: "label", placeholder: t("label"), value: target.label || "" }),
      h("input", { class: "path", placeholder: "/volume1/backup/PC-btrfs", value: target.path || "", required: true }),
      h("button", { type: "button", class: "btn danger", onclick: () => row.remove() }, t("remove"))),
    h("div", { class: "target-options" },
      h("label", { class: "check" },
        h("input", { type: "checkbox", class: "notify", checked: target.notify !== false }), t("target_notify")),
      h("label", {}, t("warning_after"),
        h("input", { type: "number", min: 1, class: "warn", placeholder: t("global"), value: target.warning_hours ?? "" }), t("hours")),
      h("label", {}, t("critical_after"),
        h("input", { type: "number", min: 2, class: "crit", placeholder: t("global"), value: target.critical_hours ?? "" }), t("hours"))));
  return row;
}

/** Greys out what a switched-off checkbox controls. */
function syncDisabled() {
  const enabled = $("n-enabled").checked;
  for (const el of document.querySelectorAll("#notify-options input, #notify-options select, #notify-options textarea")) {
    el.disabled = !enabled;
  }
  $("n-recipient").disabled = !enabled || !$("n-desktop").checked;
  for (const id of ["n-webhook-url", "n-webhook-body", "n-language"]) $(id).disabled = !enabled || !$("n-webhook").checked;
  document.querySelector(".webhook").hidden = !$("n-webhook").checked;
}

function fillNotifications(n) {
  $("n-enabled").checked = n.enabled;
  $("n-interval").value = n.interval_minutes;
  for (const key of ["late", "critical", "recovered", "incomplete", "errors", "full_send"]) $(`n-${key}`).checked = n[key];
  $("n-reminder").value = n.reminder_hours;
  $("n-desktop").checked = n.desktop;
  $("n-recipient").value = n.recipient;
  $("n-webhook").checked = n.webhook.enabled;
  $("n-webhook-url").value = n.webhook.url;
  $("n-webhook-body").value = n.webhook.body;
  $("n-language").value = n.language;
  syncDisabled();
}

function readNotifications() {
  return {
    enabled: $("n-enabled").checked,
    interval_minutes: Number($("n-interval").value),
    late: $("n-late").checked,
    critical: $("n-critical").checked,
    recovered: $("n-recovered").checked,
    incomplete: $("n-incomplete").checked,
    errors: $("n-errors").checked,
    full_send: $("n-full_send").checked,
    reminder_hours: Number($("n-reminder").value),
    desktop: $("n-desktop").checked,
    recipient: $("n-recipient").value.trim(),
    webhook: {
      enabled: $("n-webhook").checked,
      url: $("n-webhook-url").value.trim(),
      body: $("n-webhook-body").value.trim(),
    },
    language: $("n-language").value,
  };
}

function monitorLine(monitor, now) {
  if (!monitor) return "";
  if (!monitor.enabled) return t("monitor_off");
  if (!monitor.running) return t("monitor_stopped");
  return t("monitor_on", {
    m: monitor.interval_minutes,
    last: monitor.last_check ? relative(monitor.last_check, now) : t("never"),
  });
}

async function openSettings() {
  try {
    const config = await api("settings");
    $("target-rows").replaceChildren(...config.targets.map(targetRow));
    $("warning-hours").value = config.warning_hours;
    $("critical-hours").value = config.critical_hours;
    fillNotifications(config.notifications);
    const ov = state.overview;
    $("monitor-status").textContent = ov ? monitorLine(ov.monitor, ov.now) : "";
    $("settings-error").textContent = "";
    dialog().showModal();
  } catch (err) {
    toast(err.message, true);
  }
}

function readSettings() {
  const targets = [...document.querySelectorAll("#target-rows .target-card")].map((row) => {
    const target = {
      label: row.querySelector(".label").value.trim(),
      path: row.querySelector(".path").value.trim().replace(/\/+$/, ""),
      notify: row.querySelector(".notify").checked,
    };
    const warn = numberOrNull(row.querySelector(".warn"));
    const crit = numberOrNull(row.querySelector(".crit"));
    if (warn !== null) target.warning_hours = warn;
    if (crit !== null) target.critical_hours = crit;
    return target;
  }).filter((x) => x.path);
  return {
    targets,
    warning_hours: Number($("warning-hours").value),
    critical_hours: Number($("critical-hours").value),
    notifications: readNotifications(),
  };
}

async function discover() {
  const button = $("discover");
  button.disabled = true;
  try {
    const found = await api("discover", {}, {});
    const have = new Set(readSettings().targets.map((x) => x.path));
    const fresh = found.targets.filter((x) => !have.has(x.path));
    $("target-rows").append(...fresh.map(targetRow));
    toast(fresh.length ? t("discovered", { n: fresh.length }) : t("discovered_none"));
  } catch (err) {
    toast(err.message, true);
  } finally {
    button.disabled = false;
  }
}

async function sendTest() {
  const button = $("n-test");
  button.disabled = true;
  try {
    await api("notify-test", {}, readNotifications());
    toast(t("test_sent"));
  } catch (err) {
    toast(err.message, true);
  } finally {
    button.disabled = false;
  }
}

async function saveSettings(ev) {
  ev.preventDefault();
  const error = $("settings-error");
  error.textContent = "";
  try {
    await api("settings", {}, readSettings());
    dialog().close();
    toast(t("saved"));
    state.detail = null;
    await refresh();
  } catch (err) {
    error.textContent = err.message;
  }
}

/* ---------- main ---------- */

async function render() {
  const route = parseHash();
  renderSidebar(route);
  const ov = state.overview;
  if (!ov) return;
  if (!ov.targets.length) {
    return showPlaceholder(t("no_targets_title"), t("no_targets"),
      h("button", { class: "btn primary", onclick: openSettings }, t("open_settings")));
  }
  if (route.target === null || Number.isNaN(route.target)) return go({ target: ov.targets[0].id });
  try {
    if (route.snapshot) await renderBrowse(route);
    else await renderTarget(route);
  } catch (err) {
    showError(err);
  }
}

async function refresh() {
  const button = document.getElementById("refresh");
  button.classList.add("spinning");
  try {
    state.overview = await api("overview");
    state.detail = null;
    document.getElementById("user").textContent = state.overview.user;
    await render();
  } catch (err) {
    renderSidebar(parseHash());
    showError(err);
  } finally {
    button.classList.remove("spinning");
  }
}

function init() {
  document.documentElement.lang = LANG;
  // inside the DSM window the window title already names the app
  if (window.self !== window.top) document.body.classList.add("embedded");
  for (const el of document.querySelectorAll("[data-i18n]")) el.textContent = t(el.dataset.i18n);
  for (const el of document.querySelectorAll("[data-i18n-title]")) el.title = t(el.dataset.i18nTitle);
  document.getElementById("refresh").addEventListener("click", refresh);
  document.getElementById("open-settings").addEventListener("click", openSettings);
  document.getElementById("add-target").addEventListener("click", () => document.getElementById("target-rows").append(targetRow()));
  document.getElementById("discover").addEventListener("click", discover);
  document.getElementById("n-test").addEventListener("click", sendTest);
  for (const id of ["n-enabled", "n-desktop", "n-webhook"]) document.getElementById(id).addEventListener("change", syncDisabled);
  document.getElementById("settings-cancel").addEventListener("click", () => dialog().close());
  document.getElementById("settings-form").addEventListener("submit", saveSettings);
  window.addEventListener("hashchange", render);
  loadSynoToken().then(refresh);
}

init();
