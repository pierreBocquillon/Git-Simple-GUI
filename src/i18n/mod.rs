//! Translations of every text shown on screen: English (the default), French, German and
//! Spanish.
//!
//! In the code, a text is written as a key:
//!
//! ```ignore
//! tr!("toolbar.pull")                          // -> &'static str
//! tr!("toast.branch_created", name = clean)    // -> String, "{name}" replaced
//! ```
//!
//! Each language is one file (`en.rs`, `fr.rs`, `de.rs`, `es.rs`) holding a list of
//! `(key, text)` pairs. To add a text, add its key to all four files (a test checks that every
//! language has the same keys and the same `{placeholders}`). A key missing from a language
//! falls back to English.
//!
//! The language is chosen in the app (⚙ → Language), proposed by the installer, and English
//! when neither said anything.

mod de;
mod en;
mod es;
mod fr;

use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    En,
    Fr,
    De,
    Es,
}

impl Lang {
    pub const ALL: [Lang; 4] = [Lang::En, Lang::Fr, Lang::De, Lang::Es];

    /// Short code saved in the settings file ("en", "fr", "de", "es").
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Fr => "fr",
            Lang::De => "de",
            Lang::Es => "es",
        }
    }

    pub fn from_code(code: &str) -> Option<Lang> {
        Lang::ALL.into_iter().find(|l| l.code() == code.trim())
    }

    /// The language's name in that language, for the language menu.
    pub fn native_name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Fr => "Français",
            Lang::De => "Deutsch",
            Lang::Es => "Español",
        }
    }

    fn strings(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Lang::En => en::STRINGS,
            Lang::Fr => fr::STRINGS,
            Lang::De => de::STRINGS,
            Lang::Es => es::STRINGS,
        }
    }

    /// The language's texts by key (built on first use).
    fn table(self) -> &'static HashMap<&'static str, &'static str> {
        static TABLES: [OnceLock<HashMap<&str, &str>>; 4] = [
            OnceLock::new(),
            OnceLock::new(),
            OnceLock::new(),
            OnceLock::new(),
        ];
        TABLES[self as usize].get_or_init(|| self.strings().iter().copied().collect())
    }
}

/// The language in use, as `Lang as u8`.
static CURRENT: AtomicU8 = AtomicU8::new(Lang::En as u8);

pub fn current() -> Lang {
    Lang::ALL[CURRENT.load(Ordering::Relaxed) as usize]
}

pub fn set(lang: Lang) {
    CURRENT.store(lang as u8, Ordering::Relaxed);
}

/// The text for `key` in the current language (English if missing there, the key itself if
/// missing everywhere, so a forgotten text is easy to spot).
pub fn text(key: &'static str) -> &'static str {
    current()
        .table()
        .get(key)
        .or_else(|| Lang::En.table().get(key))
        .copied()
        .unwrap_or(key)
}

/// Replace each `{name}` in `template` with its value.
pub fn fill(template: &str, args: &[(&str, String)]) -> String {
    let mut out = template.to_string();
    for (name, value) in args {
        out = out.replace(&format!("{{{name}}}"), value);
    }
    out
}

/// `tr!("key")` gives the text for `key`; `tr!("key", name = value, …)` also replaces each
/// `{name}` in it (any value that implements `Display`).
#[macro_export]
macro_rules! tr {
    ($key:literal) => {
        $crate::i18n::text($key)
    };
    ($key:literal, $($name:ident = $value:expr),+ $(,)?) => {
        $crate::i18n::fill(
            $crate::i18n::text($key),
            &[$((stringify!($name), ($value).to_string())),+],
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn placeholders(s: &str) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        let mut rest = s;
        while let Some(start) = rest.find('{') {
            let Some(len) = rest[start..].find('}') else {
                break;
            };
            out.insert(rest[start..start + len + 1].to_string());
            rest = &rest[start + len + 1..];
        }
        out
    }

    #[test]
    fn every_language_has_every_key_with_the_same_placeholders() {
        let english = Lang::En.table();
        assert_eq!(english.len(), en::STRINGS.len(), "duplicate key in en.rs");
        for lang in [Lang::Fr, Lang::De, Lang::Es] {
            let table = lang.table();
            assert_eq!(
                table.len(),
                lang.strings().len(),
                "duplicate key in {:?}",
                lang
            );
            for (key, text) in english {
                let Some(other) = table.get(key) else {
                    panic!("{:?} is missing \"{key}\"", lang);
                };
                assert_eq!(
                    placeholders(text),
                    placeholders(other),
                    "{:?} \"{key}\": placeholders differ",
                    lang
                );
            }
            for key in table.keys() {
                assert!(
                    english.contains_key(key),
                    "{:?} has unknown key \"{key}\"",
                    lang
                );
            }
        }
    }

    #[test]
    fn fill_replaces_placeholders() {
        assert_eq!(
            fill("{a} and {b}", &[("a", "1".into()), ("b", "2".into())]),
            "1 and 2"
        );
    }
}
