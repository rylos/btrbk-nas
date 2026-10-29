//! The language of the messages: Italian or English. The interface sends
//! its own (?lang=, from the DSM session), the periodic check uses the one
//! set for the webhook; anything else, a shell above all, gets English.

use std::cell::Cell;

thread_local! {
    static ITALIAN: Cell<bool> = const { Cell::new(false) };
}

/// Italian for any code starting with "it" (it, it-IT, ita), English
/// otherwise.
pub fn set(code: Option<&str>) {
    ITALIAN.set(code.is_some_and(|c| c.to_ascii_lowercase().starts_with("it")));
}

pub fn italian() -> bool {
    ITALIAN.get()
}

/// `t!("italiano {x}", "English {x}")`: format! in the current language.
macro_rules! t {
    ($it:literal, $en:literal $(, $arg:expr)* $(,)?) => {
        if $crate::lang::italian() { format!($it $(, $arg)*) } else { format!($en $(, $arg)*) }
    };
}
pub(crate) use t;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// The keys of one language in TEXTS of ui/app.js: the names before a
    /// colon, outside the quoted texts.
    fn ui_keys(app: &str, lang: &str) -> BTreeSet<String> {
        let start = app.find(&format!("\n  {lang}: {{\n")).expect("language block") + 1;
        let block = &app[start..start + app[start..].find("\n  },").expect("end of block")];
        let block = &block[block.find('{').unwrap() + 1..];
        let mut keys = BTreeSet::new();
        let (mut word, mut quoted, mut escaped) = (String::new(), false, false);
        for c in block.chars() {
            if quoted {
                quoted = escaped || c != '"';
                escaped = !escaped && c == '\\';
            } else if c == '"' {
                quoted = true;
                word.clear();
            } else if c.is_alphanumeric() || c == '_' {
                word.push(c);
            } else {
                if c == ':' && !word.is_empty() {
                    keys.insert(word.clone());
                }
                if !c.is_whitespace() {
                    word.clear();
                }
            }
        }
        keys
    }

    #[test]
    fn interface_texts_match() {
        let app = include_str!("../ui/app.js");
        let (it, en) = (ui_keys(app, "it"), ui_keys(app, "en"));
        assert!(it.len() > 100, "{} keys only: the parser lost its way", it.len());
        assert_eq!(it.difference(&en).collect::<Vec<_>>(), Vec::<&String>::new(), "missing in English");
        assert_eq!(en.difference(&it).collect::<Vec<_>>(), Vec::<&String>::new(), "missing in Italian");
        // every t("key") of the code has a text
        let used: BTreeSet<String> = app
            .match_indices("t(\"")
            // t(, not set( or split(
            .filter(|(at, _)| !app[..*at].ends_with(|c: char| c.is_alphanumeric() || c == '_' || c == '.'))
            .filter_map(|(at, _)| app[at + 3..].split_once('"').map(|(key, _)| key.to_string()))
            .filter(|key| key.chars().all(|c| c.is_alphanumeric() || c == '_'))
            .collect();
        assert_eq!(used.difference(&it).collect::<Vec<_>>(), Vec::<&String>::new(), "used but not defined");
    }

    #[test]
    fn desktop_texts_match() {
        let keys = |strings: &str| -> BTreeSet<String> {
            strings.lines().filter_map(|l| l.split_once('=').map(|(k, _)| k.to_string())).collect()
        };
        let it = keys(include_str!("../ui/texts/ita/strings"));
        assert!(!it.is_empty());
        assert_eq!(it, keys(include_str!("../ui/texts/enu/strings")));
    }

    #[test]
    fn picks_the_language() {
        let what = "x";
        set(Some("ita"));
        assert_eq!(t!("ciao {what}", "hello {what}"), "ciao x");
        set(Some("it-IT"));
        assert!(italian());
        set(Some("enu"));
        assert_eq!(t!("ciao {}", "hello {}", 1), "hello 1");
        set(None);
        assert!(!italian());
    }
}
