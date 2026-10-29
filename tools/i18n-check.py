#!/usr/bin/env python3
"""Checks the translations, or starts a new language.

    tools/i18n-check.py            every table against the reference
    tools/i18n-check.py --new de   start ui/lang/de.js, src/lang/de.json, ui/texts/ger

Four kinds of text, one file per language for each:

- ui/lang/<code>.js: the interface, by key; en.js is the reference. A text
  with a count {n} can have one key per plural form of the language
  (key_one, key_few, key_many: Intl.PluralRules), key being the rest.
- src/lang/<code>.json: the NAS's messages, t!("italiano", "English") in
  src/, keyed by the Italian; Italian and English need no table.
- ui/texts/<DSM code>/strings: the DSM desktop notifications.
- description_<DSM code> in spk/INFO.in: Package Center's description.

For each it reports what is missing, unused, still English, or has other
placeholders than the original. It exits 1 when something is missing or
broken; `cargo test` checks the same.
"""

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
UI_LANG = ROOT / "ui" / "lang"
SRC_LANG = ROOT / "src" / "lang"
TEXTS = ROOT / "ui" / "texts"
INFO = ROOT / "spk" / "INFO.in"

ISO_TO_DSM = {
    "en": "enu", "it": "ita", "de": "ger", "fr": "fre", "es": "spn", "zh-TW": "cht", "zh-CN": "chs", "ja": "jpn",
    "ko": "krn", "nl": "nld", "pt-BR": "ptb", "pt": "ptg", "ru": "rus", "sv": "sve", "da": "dan", "nb": "nor",
    "pl": "plk", "tr": "trk", "cs": "csy", "hu": "hun", "th": "tha",
}

STR = r'"((?:[^"\\]|\\.)*)"'


def unq(s):
    return json.loads('"' + s + '"')


def ui_table(code):
    """ui/lang/<code>.js: one `key: "text",` per line."""
    out = {}
    for line in (UI_LANG / f"{code}.js").read_text().splitlines():
        m = re.fullmatch(r"\s*(\w+): " + STR + r",?", line)
        if m:
            out[m[1]] = unq(m[2])
    return out


def write_ui_table(code, name, table):
    lines = [f"// {name}: every text of the interface, by key (see i18n in app.js).", "export default {"]
    lines += [f"  {k}: {json.dumps(v, ensure_ascii=False)}," for k, v in table.items()]
    (UI_LANG / f"{code}.js").write_text("\n".join(lines + ["};", ""]))


def messages():
    """Italian -> English of every t!() in src/."""
    out = {}
    for f in sorted((ROOT / "src").glob("*.rs")):
        if f.name == "lang.rs":
            continue
        for m in re.finditer(r"(?<![\w])t!\(\s*" + STR + r",\s*" + STR, f.read_text()):
            out[unq(m[1])] = unq(m[2])
    return out


def strings(dsm):
    path = TEXTS / dsm / "strings"
    return dict(re.findall(r'^(\w+)="(.*)"$', path.read_text(), re.M)) if path.exists() else {}


def descriptions():
    return dict(re.findall(r'^description(?:_(\w+))?="(.*)"$', INFO.read_text(), re.M))


def plural_of(key, reference):
    m = re.fullmatch(r"(\w+)_(zero|one|two|few|many)", key)
    return bool(m and m[1] in reference)


def placeholders(s, rust=False):
    return sorted(re.findall(r"\{\}" if rust else r"\{\w+\}", s))


def report(name, table, reference, rust=False, english=None):
    """Prints one table's state; returns the count of problems."""
    missing = [k for k in reference if not table.get(k)]
    # key_few, key_many...: plural forms only some languages need
    unused = [k for k in table if k not in reference and not plural_of(k, reference)]
    base = lambda k: k if k in reference else re.sub(r"_(zero|one|two|few|many)$", "", k)
    broken = [k for k, v in table.items() if base(k) in reference and v and placeholders(v, rust) != placeholders(reference[base(k)], rust)]
    same = [k for k, v in table.items() if english and v == english.get(k) and re.search(r"[a-z]{4}", v)]
    print(f"{name}: {len(table)} texts, {len(missing)} missing, {len(unused)} unused, "
          f"{len(same)} still English, {len(broken)} with other placeholders")
    for k in missing:
        print(f"  missing:      {k!r}")
    for k in broken:
        print(f"  placeholders: {k!r} -> {table[k]!r}")
    for k in unused:
        print(f"  unused:       {k!r}")
    for k in same:
        print(f"  english:      {k!r}")
    return len(missing) + len(broken)


def check():
    bad = 0
    en = ui_table("en")
    codes = sorted(p.stem for p in UI_LANG.glob("*.js"))
    for code in codes:
        bad += report(f"ui/lang/{code}.js", ui_table(code), en, english=None if code == "en" else en)
    msgs = messages()
    italian = {k: k for k in msgs}
    for path in sorted(SRC_LANG.glob("*.json")):
        bad += report(f"src/lang/{path.name}", json.loads(path.read_text()), italian, rust=True, english=msgs)
    enu = strings("enu")
    desc = descriptions()
    for code in codes:
        dsm = ISO_TO_DSM[code]
        bad += report(f"ui/texts/{dsm}/strings", strings(dsm), enu, english=None if code == "en" else enu)
        if code not in ("en",) and dsm not in desc:
            print(f"spk/INFO.in: description_{dsm} missing")
            bad += 1
        if code not in ("it", "en") and not (SRC_LANG / f"{code}.json").exists():
            print(f"src/lang/{code}.json missing")
            bad += 1
    return 1 if bad else 0


def new(code):
    if code not in ISO_TO_DSM:
        sys.exit(f"{code}: not one of DSM's languages ({', '.join(ISO_TO_DSM)})")
    dsm = ISO_TO_DSM[code]
    js, rs, texts = UI_LANG / f"{code}.js", SRC_LANG / f"{code}.json", TEXTS / dsm / "strings"
    for p in (js, rs, texts):
        if p.exists():
            sys.exit(f"{p.relative_to(ROOT)} exists already")
    # the English as a start: the check lists what is still English
    write_ui_table(code, code, ui_table("en"))
    SRC_LANG.mkdir(exist_ok=True)
    rs.write_text(json.dumps(dict(sorted(messages().items())), ensure_ascii=False, indent=1) + "\n")
    texts.parent.mkdir(exist_ok=True)
    texts.write_text((TEXTS / "enu" / "strings").read_text())
    print(f"written {js.relative_to(ROOT)}, {rs.relative_to(ROOT)} and {texts.relative_to(ROOT)}, in English")
    print(f"translate them, add {code} to LANGUAGES in ui/app.js and description_{dsm}=\"...\" to spk/INFO.in,")
    print("then run tools/i18n-check.py and cargo test")


if __name__ == "__main__":
    if len(sys.argv) == 3 and sys.argv[1] == "--new":
        new(sys.argv[2])
    elif len(sys.argv) == 1:
        sys.exit(check())
    else:
        sys.exit(__doc__)
