//! HTML escaping helper — port of `src/server/utils/html-escape.ts`.

/// Escape HTML special characters (`&`, `<`, `>`, `"`, `'`) so that arbitrary
/// text can be safely embedded in HTML.
pub fn html_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#039;"),
            other => out.push(other),
        }
    }
    out
}
