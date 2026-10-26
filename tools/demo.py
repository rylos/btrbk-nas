"""Made-up API answers for screenshots and UI work: three PCs backing up to a
NAS, with the cases the interface has to show (a late PC, an interrupted
receive, a PC that sends no report). Times are relative to now, so the
pages always look current.

Used by tools/devserver.py --demo; nothing here touches btrfs.
"""

import time
import uuid

HOUR = 3600
DAY = 24 * HOUR
GIB = 1 << 30
MIB = 1 << 20


def _uuid(*parts):
    return str(uuid.uuid5(uuid.NAMESPACE_URL, "btrbk-nas-demo/" + "/".join(parts)))


def _stamp(epoch):
    return time.strftime("%Y%m%dT%H%M", time.localtime(epoch))


# when each backup ran: (days ago, hour, minute)
DAILY = [(0, 8, 47), (1, 19, 12), (2, 9, 3), (3, 21, 30), (4, 8, 55), (5, 13, 41), (6, 9, 18)]
WEEKLY = [(13, 10, 2), (20, 18, 44), (27, 9, 11)]
MONTHLY = [(58, 20, 5)]


def _at(now, days_ago, hour, minute):
    t = time.localtime(now - days_ago * DAY)
    epoch = int(time.mktime((t.tm_year, t.tm_mon, t.tm_mday, hour, minute, 0, 0, 0, -1)))
    # today's backup must not be in the future
    return min(epoch, now - 2 * HOUR - 17 * 60) if days_ago == 0 else epoch


def _target(now, tid, label, path, subvolumes, *, shift=0, schedule, on_pc, warning=26, critical=72,
            report=True, incomplete=None):
    """subvolumes: [(name, receive seconds, exclusive bytes per snapshot)]."""
    groups = []
    for sub, receive, exclusive in subvolumes:
        snaps = []
        times = sorted((_at(now, d, h, m) - shift for d, h, m in schedule), reverse=True)
        for i, created in enumerate(times):
            name = f"{sub}.{_stamp(created)}"
            full = i == len(times) - 1
            otime = created + 4 + (i % 3)
            on = report and i < on_pc
            snaps.append({
                "name": name,
                "timestamp": _stamp(created),
                "problem": None,
                "info": {
                    "uuid": _uuid(label, name),
                    "parent_uuid": _uuid(label, "parent", name),
                    "received_uuid": _uuid(label, "pc", name),
                    "generation": 90210 - i * 37,
                    "otime": otime,
                    "rtime": otime + (receive * 40 if full else receive + (i * 7) % 11),
                    "readonly": True,
                    "root_id": 4100 + tid * 300 + len(snaps) * 3 + len(groups),
                },
                "usage": {"exclusive": exclusive * (6 if full else 1) + i * 37 * MIB % exclusive,
                          "referenced": exclusive * 40},
                "on_source": on if report else None,
                "created": created + 13 + (i * 17) % 40 if on else None,
                "deletion": ("parent" if i == 0 else "allowed") if report else "parent_unknown",
            })
        if incomplete and incomplete[0] == sub:
            created = _at(now, incomplete[1], 22, 6)
            name = f"{sub}.{_stamp(created)}"
            snaps.append({
                "name": name, "timestamp": _stamp(created), "problem": "incomplete",
                "info": {"uuid": _uuid(label, name), "parent_uuid": None, "received_uuid": None,
                         "generation": 1, "otime": created + 5, "rtime": None, "readonly": False,
                         "root_id": 4099 + tid * 300},
                "usage": {"exclusive": 2 * GIB + 311 * MIB, "referenced": 2 * GIB + 311 * MIB},
                "on_source": False if report else None, "created": None, "deletion": "allowed_incomplete",
            })
            snaps.sort(key=lambda s: s["name"], reverse=True)
        complete = [s for s in snaps if s["problem"] is None]
        last = max(s["info"]["rtime"] for s in complete)
        hours = (now - last) // HOUR
        status = "critical" if hours >= critical else "warning" if hours >= warning else "ok"
        summary = {
            "subvolume": sub,
            "count": len(snaps),
            "newest": snaps[0]["name"],
            "oldest": snaps[-1]["name"],
            "last_received": last,
            "incomplete": sum(s["problem"] == "incomplete" for s in snaps),
            "not_subvolume": 0,
            "exclusive_bytes": sum(s["usage"]["exclusive"] for s in snaps),
            "status": status,
            "chain": {"next_parent": complete[0]["name"], "pending": [], "full_send": False} if report else None,
        }
        groups.append({"summary": summary, "snapshots": snaps})
    order = {"ok": 0, "warning": 1, "critical": 2}
    return {
        "id": tid,
        "label": label,
        "path": path,
        "warning_hours": warning,
        "critical_hours": critical,
        "notify": True,
        "source": {"host": label, "generated": now - 41 * 60 - shift} if report else None,
        "groups": groups,
        "status": max((g["summary"]["status"] for g in groups), key=order.get),
        "last_received": max(g["summary"]["last_received"] for g in groups),
    }


def _targets(now):
    return [
        _target(now, 0, "workstation", "/volume1/backup/workstation",
                [("@", 23, 410 * MIB), ("@home", 48, 1200 * MIB), ("@games", 6, 3 * MIB)],
                schedule=DAILY + WEEKLY + MONTHLY, on_pc=3, incomplete=("@home", 2)),
        _target(now, 1, "laptop", "/volume1/backup/laptop",
                [("@", 31, 520 * MIB), ("@home", 64, 870 * MIB)],
                schedule=DAILY[:5] + WEEKLY, shift=int(1.4 * DAY), on_pc=2),
        _target(now, 2, "homeserver", "/volume2/backup/homeserver",
                [("@", 12, 220 * MIB), ("@srv", 95, 2600 * MIB)],
                schedule=[(1, 3, 0), (8, 3, 0), (15, 3, 0), (22, 3, 0)], on_pc=0,
                warning=8 * 24, critical=15 * 24, report=False),
    ]


def _entry(name, kind, size, mtime, mode, uid=1000, gid=1000, link=None):
    entry = {"name": name, "kind": kind, "size": size, "mtime": mtime, "mode": mode, "uid": uid, "gid": gid}
    if link:
        entry["link"] = link
    return entry


def _listing(now, snapshot, path):
    d, f = 0o755, 0o644
    sub = snapshot.split(".")[0]
    tree = {
        "@": {
            "": [("bin", "symlink", 7, 0o777, "usr/bin"), ("boot", "dir", 0, d), ("etc", "dir", 0, d),
                 ("home", "dir", 0, d), ("opt", "dir", 0, d), ("root", "dir", 0, 0o700), ("srv", "dir", 0, d),
                 ("usr", "dir", 0, d), ("var", "dir", 0, d)],
        },
        "@home": {
            "": [("alex", "dir", 0, 0o700)],
            "alex": [(".config", "dir", 0, d), (".local", "dir", 0, d), (".ssh", "dir", 0, 0o700),
                     ("Documents", "dir", 0, d), ("Music", "dir", 0, d), ("Pictures", "dir", 0, d),
                     ("Projects", "dir", 0, d), (".bashrc", "file", 3771, f), (".gitconfig", "file", 214, f),
                     (".profile", "file", 807, f)],
            "alex/Documents": [("Invoices", "dir", 0, d), ("Taxes 2026", "dir", 0, d),
                               ("budget.ods", "file", 48213, f), ("car insurance.pdf", "file", 1288451, f),
                               ("lease agreement.pdf", "file", 3401227, f), ("notes.md", "file", 6120, f),
                               ("passport scan.jpg", "file", 2210334, 0o600), ("recipes.odt", "file", 91822, f),
                               ("Shared", "symlink", 21, 0o777, "/srv/shared/documents"),
                               ("travel checklist.txt", "file", 1404, f)],
        },
        "@games": {"": [("SteamLibrary", "dir", 0, d)]},
    }
    rows = tree.get(sub, {}).get(path.strip("/"), [])
    entries = []
    for i, (name, kind, size, mode, *link) in enumerate(rows):
        uid = 0 if sub == "@" else 1000
        entries.append(_entry(name, kind, size, now - (i * 7 + 2) * DAY - i * 3917, mode, uid, uid,
                              link[0] if link else None))
    entries.sort(key=lambda e: (e["kind"] != "dir", e["name"].lower()))
    return {"entries": entries, "truncated": False}


def _settings(targets):
    return {
        "targets": [{"path": t["path"], "label": t["label"], "notify": True,
                     **({"warning_hours": t["warning_hours"], "critical_hours": t["critical_hours"]}
                        if t["warning_hours"] != 26 else {})} for t in targets],
        "warning_hours": 26,
        "critical_hours": 72,
        "notifications": {
            "enabled": True, "interval_minutes": 60, "late": True, "critical": True, "recovered": True,
            "incomplete": True, "errors": True, "full_send": True, "reminder_hours": 24, "desktop": True,
            "recipient": "@administrators",
            "webhook": {"enabled": False, "url": "", "body": ""},
            "language": "en",
        },
    }


def respond(method, route, params, body):
    """(HTTP status, JSON-able answer) for one API call."""
    now = int(time.time())
    targets = _targets(now)
    monitor = {"enabled": True, "interval_minutes": 60, "last_check": now - 12 * 60, "running": True}
    if (method, route) == ("GET", "overview"):
        overview = [{**{k: v for k, v in t.items() if k != "groups"}, "groups": [g["summary"] for g in t["groups"]]}
                    for t in targets]
        return 200, {"user": "admin", "version": "demo", "now": now, "capabilities": True,
                     "monitor": monitor, "targets": overview}
    if (method, route) == ("GET", "target"):
        t = targets[int(params.get("id", 0))]
        return 200, {"label": t["label"], "path": t["path"], "error": None, "source": t["source"],
                     "groups": t["groups"]}
    if (method, route) == ("GET", "browse"):
        return 200, _listing(now, params.get("snapshot", ""), params.get("path", ""))
    if (method, route) == ("GET", "settings"):
        return 200, _settings(targets)
    if (method, route) == ("GET", "monitor"):
        return 200, monitor
    if (method, route) == ("POST", "delete"):
        time.sleep(1.5)
        return 200, {"deleted": body.get("snapshot")}
    if method == "POST":
        return 200, {"ok": True, "targets": []}
    return 404, {"error": "not in the demo"}
