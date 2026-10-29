//! The language of the messages: DSM's, which the interface sends along
//! (?lang= on every request); the periodic check uses the one set for the
//! webhook; anything else, a shell above all, gets English.
//!
//! Messages are written in the code in Italian and English:
//! `t!("italiano {}", "English {}", args...)`. Any other language comes
//! from src/lang/<code>.json, keyed by the Italian text, built into the
//! binary by build.rs. A message a table lacks falls back to English.

use std::cell::Cell;
use std::collections::HashMap;
use std::sync::OnceLock;

include!(concat!(env!("OUT_DIR"), "/lang_tables.rs"));

thread_local! {
    static LANG: Cell<&'static str> = const { Cell::new("en") };
}

/// DSM's language codes to ISO ones.
const DSM: &[(&str, &str)] = &[
    ("enu", "en"), ("ita", "it"), ("ger", "de"), ("fre", "fr"), ("spn", "es"), ("cht", "zh-TW"), ("chs", "zh-CN"),
    ("jpn", "ja"), ("krn", "ko"), ("nld", "nl"), ("ptb", "pt-BR"), ("ptg", "pt"), ("rus", "ru"), ("sve", "sv"),
    ("dan", "da"), ("nor", "nb"), ("plk", "pl"), ("trk", "tr"), ("csy", "cs"), ("hun", "hu"), ("tha", "th"),
];

fn tables() -> &'static HashMap<&'static str, HashMap<String, String>> {
    static PARSED: OnceLock<HashMap<&'static str, HashMap<String, String>>> = OnceLock::new();
    PARSED.get_or_init(|| {
        TABLES
            .iter()
            .filter_map(|(code, text)| Some((*code, serde_json::from_str::<HashMap<String, String>>(text).ok()?)))
            .collect()
    })
}

/// The languages the NAS can answer in.
pub fn available() -> Vec<&'static str> {
    let mut codes: Vec<_> = ["it", "en"].into_iter().chain(TABLES.iter().map(|(code, _)| *code)).collect();
    codes.sort_unstable();
    codes
}

/// The language to answer in: "it", "en", or one with a table; English
/// for anything else. Takes DSM's codes (ita, ger, ptb) and ISO ones.
pub fn resolve(raw: &str) -> &'static str {
    let low = raw.trim().replace('_', "-");
    let iso = DSM.iter().find(|(d, _)| d.eq_ignore_ascii_case(&low)).map(|(_, i)| i.to_string()).unwrap_or(low);
    let base = iso.split('-').next().unwrap_or_default().to_string();
    for candidate in [iso.as_str(), base.as_str()] {
        if let Some(code) = available().into_iter().find(|c| c.eq_ignore_ascii_case(candidate)) {
            return code;
        }
    }
    "en"
}

pub fn set(code: Option<&str>) {
    LANG.set(resolve(code.unwrap_or("en")));
}

pub fn code() -> &'static str {
    LANG.get()
}

/// The template in the current language.
pub fn pick(it: &'static str, en: &'static str) -> &'static str {
    match code() {
        "it" => it,
        "en" => en,
        other => tables().get(other).and_then(|t| t.get(it)).map(String::as_str).filter(|s| !s.is_empty()).unwrap_or(en),
    }
}

/// Fills the `{}` of a template in order.
pub fn fill(template: &str, args: &[String]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut rest = template;
    let mut args = args.iter();
    while let Some(at) = rest.find("{}") {
        out.push_str(&rest[..at]);
        out.push_str(args.next().map(String::as_str).unwrap_or(""));
        rest = &rest[at + 2..];
    }
    out.push_str(rest);
    out
}

/// `t!("italiano {}", "English {}", args...)`: a String in the current
/// language. Placeholders are `{}` only, filled in order.
macro_rules! t {
    ($it:literal, $en:literal $(, $arg:expr)* $(,)?) => {
        $crate::lang::fill($crate::lang::pick($it, $en), &[$(::std::string::ToString::to_string(&$arg)),*])
    };
}
pub(crate) use t;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs;
    use std::path::Path;

    const ROOT: &str = env!("CARGO_MANIFEST_DIR");

    #[test]
    fn resolves_dsm_codes() {
        assert_eq!(resolve("ita"), "it");
        assert_eq!(resolve("it-IT"), "it");
        assert_eq!(resolve("enu"), "en");
        assert_eq!(resolve("ger"), "de");
        assert_eq!(resolve("ptb"), "pt-BR");
        assert_eq!(resolve("pt_br"), "pt-BR");
        assert_eq!(resolve("ptg"), "pt");
        assert_eq!(resolve("fr-CA"), "fr");
        // no table: English
        assert_eq!(resolve("tha"), "en");
        assert_eq!(resolve(""), "en");
    }

    #[test]
    fn fills_and_falls_back() {
        set(Some("ita"));
        assert_eq!(t!("{} saltati, {} errori", "{} skipped, {} errors", 3, 1), "3 saltati, 1 errori");
        assert_eq!(code(), "it");
        set(Some("enu"));
        assert_eq!(t!("{} saltati", "{} skipped", 2), "2 skipped");
        set(Some("ger"));
        assert_eq!(t!("messaggio che nessuna tabella ha", "a message no table has"), "a message no table has");
        set(None);
        assert_eq!(code(), "en");
        assert_eq!(fill("a {} b {}", &["1".into()]), "a 1 b ");
    }

    /// The (Italian, English) pairs of every t!() in src/.
    fn messages() -> BTreeMap<String, String> {
        fn literal(text: &str) -> Option<(String, &str)> {
            let text = text.trim_start().strip_prefix('"')?;
            let mut out = String::new();
            let mut chars = text.char_indices();
            while let Some((i, c)) = chars.next() {
                match c {
                    '"' => return Some((out, &text[i + 1..])),
                    '\\' => match chars.next()?.1 {
                        'n' => out.push('\n'),
                        other => out.push(other),
                    },
                    c => out.push(c),
                }
            }
            None
        }
        let mut out = BTreeMap::new();
        for entry in fs::read_dir(Path::new(ROOT).join("src")).unwrap().flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "rs") || path.ends_with("lang.rs") {
                continue;
            }
            let code = fs::read_to_string(&path).unwrap();
            for (at, _) in code.match_indices("t!(") {
                if code[..at].ends_with(|c: char| c.is_alphanumeric() || c == '_') {
                    continue;
                }
                let (it, rest) = literal(&code[at + 3..]).unwrap_or_else(|| panic!("{}: t! without a literal", path.display()));
                let (en, _) = literal(rest.trim_start().strip_prefix(',').unwrap()).unwrap();
                out.insert(it, en);
            }
        }
        out
    }

    #[test]
    fn message_tables_are_complete() {
        let messages = messages();
        assert!(messages.len() > 40, "{} messages only: the parser lost its way", messages.len());
        for (it, en) in &messages {
            assert_eq!(it.matches("{}").count(), en.matches("{}").count(), "{it:?} → {en:?}");
        }
        assert_eq!(tables().len(), TABLES.len(), "a src/lang/*.json is not valid JSON");
        for (code, table) in tables() {
            for (it, tr) in table {
                assert!(messages.contains_key(it), "src/lang/{code}.json: no longer used: {it:?}");
                assert_eq!(it.matches("{}").count(), tr.matches("{}").count(), "{code}: {it:?} → {tr:?}");
            }
            for it in messages.keys() {
                assert!(table.get(it).is_some_and(|t| !t.is_empty()), "src/lang/{code}.json: missing {it:?}");
            }
        }
    }

    /// A table of ui/lang/: one `key: "text",` per line.
    fn ui_table(code: &str) -> BTreeMap<String, String> {
        let text = fs::read_to_string(Path::new(ROOT).join(format!("ui/lang/{code}.js"))).unwrap();
        text.lines()
            .filter_map(|line| {
                let (key, value) = line.trim().split_once(": ")?;
                let value = value.strip_suffix(',').unwrap_or(value);
                (key.chars().all(|c| c.is_alphanumeric() || c == '_') && value.starts_with('"'))
                    .then(|| (key.to_string(), serde_json::from_str::<String>(value).unwrap()))
            })
            .collect()
    }

    fn placeholders(text: &str) -> BTreeSet<&str> {
        text.match_indices('{')
            .filter_map(|(at, _)| text[at..].find('}').map(|end| &text[at..=at + end]))
            .filter(|p| p[1..p.len() - 1].chars().all(|c| c.is_alphanumeric() || c == '_'))
            .collect()
    }

    #[test]
    fn interface_tables_are_complete() {
        let en = ui_table("en");
        assert!(en.len() > 100, "{} keys only: the parser lost its way", en.len());
        let mut codes = Vec::new();
        for entry in fs::read_dir(Path::new(ROOT).join("ui/lang")).unwrap().flatten() {
            let name = entry.file_name().into_string().unwrap();
            let code = name.trim_end_matches(".js").to_string();
            let table = ui_table(&code);
            let missing: Vec<_> = en.keys().filter(|k| !table.contains_key(*k)).collect();
            assert!(missing.is_empty(), "ui/lang/{name}: missing {missing:?}");
            for (key, text) in &table {
                // key_few, key_many...: plural forms only some languages need
                let reference = en.get(key).or_else(|| {
                    let (base, form) = key.rsplit_once('_')?;
                    ["zero", "one", "two", "few", "many"].contains(&form).then(|| en.get(base))?
                });
                let reference = reference.unwrap_or_else(|| panic!("ui/lang/{name}: {key} is not in en.js"));
                assert!(!text.is_empty(), "ui/lang/{name}: {key} is empty");
                assert_eq!(placeholders(text), placeholders(reference), "ui/lang/{name}: {key}");
            }
            codes.push(code);
        }
        codes.sort();
        // the interface and the NAS speak the same languages
        assert_eq!(codes, available());

        // every t("key") and data-i18n of the interface has a text
        let app = fs::read_to_string(Path::new(ROOT).join("ui/app.js")).unwrap();
        let html = fs::read_to_string(Path::new(ROOT).join("ui/index.html")).unwrap();
        let mut used = BTreeSet::new();
        for (text, call) in [(&app, "t(\""), (&html, "data-i18n=\""), (&html, "data-i18n-title=\"")] {
            for (at, _) in text.match_indices(call) {
                if call == "t(\"" && text[..at].ends_with(|c: char| c.is_alphanumeric() || c == '_' || c == '.') {
                    continue;
                }
                if let Some((key, _)) = text[at + call.len()..].split_once('"') {
                    used.insert(key.to_string());
                }
            }
        }
        let undefined: Vec<_> = used.iter().filter(|k| !en.contains_key(*k)).collect();
        assert!(undefined.is_empty(), "used but not defined: {undefined:?}");
    }

    #[test]
    fn desktop_texts_are_complete() {
        let keys = |dir: &Path| -> BTreeSet<String> {
            fs::read_to_string(dir.join("strings"))
                .unwrap()
                .lines()
                .filter_map(|l| l.split_once('=').map(|(k, _)| k.to_string()))
                .collect()
        };
        let texts = Path::new(ROOT).join("ui/texts");
        let enu = keys(&texts.join("enu"));
        assert!(!enu.is_empty());
        let mut dsm_codes = Vec::new();
        for entry in fs::read_dir(&texts).unwrap().flatten() {
            assert_eq!(keys(&entry.path()), enu, "{}", entry.path().display());
            dsm_codes.push(entry.file_name().into_string().unwrap());
        }
        // a DSM text folder for every language
        let mut expected: Vec<_> =
            available().into_iter().map(|code| DSM.iter().find(|(_, iso)| *iso == code).unwrap().0.to_string()).collect();
        expected.sort();
        dsm_codes.sort();
        assert_eq!(dsm_codes, expected);
    }
}
