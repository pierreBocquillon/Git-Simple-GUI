//! Small text helpers: author initials and valid branch names.

pub fn initials(name: &str) -> String {
    let mut out = String::new();
    for word in name.split(|c: char| c.is_whitespace() || c == '.' || c == '-' || c == '_') {
        if let Some(c) = word.chars().next() {
            if c.is_alphanumeric() {
                out.extend(c.to_uppercase());
            }
        }
        if out.chars().count() >= 2 {
            break;
        }
    }
    if out.is_empty() {
        out.push('?');
    }
    out
}

/// A plausible branch name from what a person typed.
pub fn sanitize_ref_name(s: &str) -> String {
    let mut out: String = s
        .trim()
        .chars()
        .map(|c| {
            if c.is_whitespace() || "~^:?*[\\".contains(c) {
                '-'
            } else {
                c
            }
        })
        .collect();
    while out.contains("..") {
        out = out.replace("..", ".");
    }
    out.trim_matches(|c| c == '/' || c == '.').to_string()
}
