"use strict";

/* ---------- texts ---------- */

// The interface speaks DSM's language. ui/lang/<code>.js holds every text
// by key, en.js being the reference; a text a table lacks falls back to
// English. Adding a language: tools/i18n-check.py --new <code>.

/** DSM's language codes (SYNO.SDS.Session.lang) to ISO ones. */
const DSM_LANGS = {
  enu: "en", ita: "it", ger: "de", fre: "fr", spn: "es", cht: "zh-TW", chs: "zh-CN", jpn: "ja", krn: "ko",
  nld: "nl", ptb: "pt-BR", ptg: "pt", rus: "ru", sve: "sv", dan: "da", nor: "nb", plk: "pl", trk: "tr",
  csy: "cs", hun: "hu", tha: "th",
};

/** The languages there is a table for, by their own name: also the webhook's choices. */
const LANGUAGES = {
  de: "Deutsch", en: "English", es: "Español", fr: "Français", it: "Italiano", nl: "Nederlands",
  pl: "Polski", pt: "Português", "pt-BR": "Português (Brasil)",
};

// ?lang= (handy for tests), then the DSM session's language (the interface
// is a same-origin iframe of the desktop), then the browser's
function dsmLang() {
  const asked = new URLSearchParams(location.search).get("lang");
  if (asked) return asked;
  try {
    const lang = window.parent.SYNO.SDS.Session.lang;
    // "def" is DSM's "same as the browser"
    if (lang && lang !== "def") return lang;
  } catch { /* opened on its own, outside DSM */ }
  return navigator.language || "en";
}

/** ISO code of the language with a table, from DSM's or a browser's code: English if none. */
function resolveLang(raw) {
  const low = String(raw || "").trim().replace("_", "-");
  const iso = DSM_LANGS[low.toLowerCase()] || low;
  const [base, region] = iso.split("-");
  const full = region ? `${base.toLowerCase()}-${region.toUpperCase()}` : base.toLowerCase();
  return [full, base.toLowerCase()].find((code) => code in LANGUAGES) || "en";
}

const LANG = resolveLang(dsmLang());
/** For Intl: dates and numbers. */
const LOCALE = { en: "en-GB" }[LANG] || LANG;
let T = {};
let EN = {};
// DSM's web server sends no cache headers: the package version in every
// URL (build.sh writes it over "dev") keeps browsers off old files
const ASSET_VERSION = document.currentScript ? new URL(document.currentScript.src).search : "";

/** Loads the table of LANG, and English to fall back on. */
async function loadTexts() {
  const load = async (code) => {
    try {
      return (await import(`./lang/${code}.js${ASSET_VERSION}`)).default;
    } catch {
      return {};
    }
  };
  EN = await load("en");
  T = LANG === "en" ? EN : await load(LANG);
}

// With a count {n}, a table can hold one text per plural form of its
// language: key_one, key_few, key_many (Intl.PluralRules), key for the rest.
const pluralOf = (table, key, locale, n) =>
  typeof n === "number" ? table[`${key}_${new Intl.PluralRules(locale).select(n)}`] : undefined;

// placeholders without a value stay as they are: {title} in the webhook help
const t = (key, vars = {}) =>
  (pluralOf(T, key, LOCALE, vars.n) || T[key] || pluralOf(EN, key, "en", vars.n) || EN[key] || key)
    .replace(/\{(\w+)\}/g, (all, k) => vars[k] ?? all);

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
  share: '<svg viewBox="0 0 24 24"><path d="M14 4h6v6M20 4l-9 9M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5"/></svg>',
  lock: '<svg viewBox="0 0 24 24"><rect x="5" y="11" width="14" height="9" rx="2"/><path d="M8 11V8a4 4 0 0 1 8 0v3"/></svg>',
};
const icon = (name) => h("span", { html: ICONS[name], style: "display:contents" });

const dateFmt = new Intl.DateTimeFormat(LOCALE, { dateStyle: "medium", timeStyle: "short" });
const secondFmt = new Intl.DateTimeFormat(LOCALE, { dateStyle: "medium", timeStyle: "medium" });
const dayFmt = new Intl.DateTimeFormat(LOCALE, { day: "numeric" });
const rtf = new Intl.RelativeTimeFormat(LOCALE, { numeric: "auto" });

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
  // the NAS answers its messages in the interface's language
  query.set("lang", LANG);
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

function snapshotTable(targetId, targetPath, group) {
  const hasUsage = group.snapshots.some((s) => s.usage);
  const hasSource = group.snapshots.some((s) => s.on_source !== null && s.on_source !== undefined);
  const parent = group.summary.chain && group.summary.chain.next_parent;
  return h("div", { class: "table-wrap" }, h("table", {},
    h("thead", {}, h("tr", {},
      h("th", {}, t("col_snapshot")), h("th", {}, t("col_taken")),
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
        // to the second when the PC reports it, else to the minute of the name;
        // the receive on the NAS goes in the tooltip
        h("td", {
          title: [
            info.rtime || info.otime ? `${t("col_received")}: ${secondFmt.format(new Date((info.rtime || info.otime) * 1000))}` : "",
            info.rtime && info.otime ? `${t("col_duration")}: ${duration(info.rtime - info.otime)}` : "",
          ].filter(Boolean).join("\n"),
        }, snap.created ? secondFmt.format(new Date(snap.created * 1000)) : taken ? dateFmt.format(taken) : "—"),
        h("td", {}, snapshotState(snap)),
        hasSource ? h("td", {}, snap.on_source ? h("span", { class: "pill ok" }, t("yes")) : "—") : null,
        hasUsage ? h("td", { class: "num" }, bytes(snap.usage && snap.usage.exclusive)) : null,
        h("td", { class: "mono", title: info.received_uuid || "" }, info.received_uuid ? info.received_uuid.slice(0, 8) + "…" : "—"),
        h("td", { class: "actions" },
          // only a read-only snapshot holds still: the NAS refuses to browse the others
          !info.readonly ? null : h("button", {
            class: "btn small",
            onclick: () => go({ target: targetId, group: group.summary.subvolume, snapshot: snap.name }),
          }, icon("browse"), t("browse")),
          snap.problem === "not_subvolume" ? null : fileStationButton(`${targetPath}/${snap.name}`, true),
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
      snapshotTable(route.target, detail.path, group),
    ] : null,
  );
}

/* ---------- file browser ---------- */

// File Station names paths by shared folder: /volume2/backup/x is /backup/x.
// Outside a shared folder it has nothing to show, and no button appears.
function fileStationPath(path) {
  const m = /^\/volume[^/]+(\/[^/].*)$/.exec(path);
  return m ? m[1].replace(/\/+$/, "") : null;
}

// Inside the DSM window the page asks the desktop to launch File Station; in a
// plain tab it opens DSM with the same launch parameters.
function openInFileStation(path, isDir) {
  const openfile = isDir ? `${path}/` : path;
  const app = "SYNO.SDS.App.FileStation3.Instance";
  try {
    const sds = window.parent !== window && window.parent.SYNO && window.parent.SYNO.SDS;
    if (sds && sds.AppLaunch) return sds.AppLaunch(app, { openfile });
  } catch (e) { /* not inside DSM */ }
  window.open(`/?launchApp=${app}&launchParam=${encodeURIComponent(`openfile=${encodeURIComponent(openfile)}`)}`, "_blank");
}

function fileStationButton(path, isDir, label) {
  const fsPath = fileStationPath(path);
  return fsPath === null ? null : h("button", {
    class: label ? "btn" : "btn small", title: t("file_station"),
    onclick: (ev) => { ev.stopPropagation(); openInFileStation(fsPath, isDir); },
  }, icon("share"), label || null);
}

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
  const base = `${detail.path}/${route.snapshot}`;

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
            : null,
          e.kind === "file" || e.kind === "dir"
            ? fileStationButton(`${base}/${join(e.name)}`, e.kind === "dir")
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
      fileStationButton(route.path ? `${base}/${route.path}` : base, true, t("file_station")),
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

// The DSM desktop keeps the loader of the version it started with, so after
// an upgrade the window can open the old index.html, still in the browser's
// cache. When the NAS runs another version than the URLs name, reload with
// its version, once.
function staleVersion(version) {
  const named = new URLSearchParams(ASSET_VERSION).get("v");
  if (!named || named === "dev" || !version || version === "demo" || named === version) return false;
  const url = new URL(location.href);
  if (url.searchParams.get("v") === version) return false;
  url.searchParams.set("v", version);
  location.replace(url);
  return true;
}

async function refresh() {
  const button = document.getElementById("refresh");
  button.classList.add("spinning");
  try {
    state.overview = await api("overview");
    if (staleVersion(state.overview.version)) return;
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

/* ---------- theme ---------- */

// Light or dark: the system's, until the button in the bar picks one, which
// this browser then remembers.
const THEME_KEY = "btrbk-nas.theme";

function storedTheme() {
  try {
    return localStorage.getItem(THEME_KEY);
  } catch {
    return null;
  }
}

function currentTheme() {
  return document.documentElement.dataset.theme
    || (matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light");
}

// built here, not in index.html: a page that is still the cached one of
// an older version must not stop the interface from starting
function themeButton() {
  const button = h("button", {
    class: "icon-btn",
    id: "theme",
    html: '<svg class="to-dark" viewBox="0 0 24 24"><path d="M20 14.5A8 8 0 0 1 9.5 4a8 8 0 1 0 10.5 10.5z"/></svg>'
      + '<svg class="to-light" viewBox="0 0 24 24"><circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/></svg>',
    onclick: toggleTheme,
  });
  document.getElementById("open-settings").before(button);
}

function showTheme() {
  const dark = currentTheme() === "dark";
  const button = document.getElementById("theme");
  button.classList.toggle("is-dark", dark);
  button.title = t(dark ? "theme_light" : "theme_dark");
}

function toggleTheme() {
  const next = currentTheme() === "dark" ? "light" : "dark";
  document.documentElement.dataset.theme = next;
  try {
    localStorage.setItem(THEME_KEY, next);
  } catch { /* private window: for this page only */ }
  showTheme();
}

// before anything is drawn, so the page never flashes the other theme
if (storedTheme() === "light" || storedTheme() === "dark") document.documentElement.dataset.theme = storedTheme();

async function init() {
  await loadTexts();
  document.documentElement.lang = LANG;
  const languages = document.getElementById("n-language");
  for (const [code, name] of Object.entries(LANGUAGES)) languages.append(h("option", { value: code }, name));
  // inside the DSM window the window title already names the app
  if (window.self !== window.top) document.body.classList.add("embedded");
  for (const el of document.querySelectorAll("[data-i18n]")) el.textContent = t(el.dataset.i18n);
  for (const el of document.querySelectorAll("[data-i18n-title]")) el.title = t(el.dataset.i18nTitle);
  document.getElementById("refresh").addEventListener("click", refresh);
  document.getElementById("open-settings").addEventListener("click", openSettings);
  themeButton();
  matchMedia("(prefers-color-scheme: dark)").addEventListener("change", showTheme);
  showTheme();
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
