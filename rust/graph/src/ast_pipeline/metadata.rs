//! A definition's stored source text (BUG-616): its first lines, capped in
//! lines and bytes, always cut on a character boundary.

const MAX_SOURCE_LINES: usize = 100;
const MAX_SOURCE_BYTES: usize = 4096;

/// Extract source code with a cap on size.
///
/// Returns `(capped_source, was_truncated)`.
/// Caps at `MAX_SOURCE_LINES` lines and `MAX_SOURCE_BYTES` bytes. A capped
/// source is a prefix of `full_text` that ends at the last complete line
/// inside both caps, or, when the first line alone is longer than the byte
/// cap, at the last character boundary at or before `MAX_SOURCE_BYTES`
/// (BUG-616). Every cut is a character boundary.
pub fn extract_source(full_text: &str) -> (String, bool) {
    if full_text.lines().count() <= MAX_SOURCE_LINES && full_text.len() <= MAX_SOURCE_BYTES {
        return (full_text.to_string(), false);
    }

    // The first MAX_SOURCE_LINES lines, without the terminator of the last.
    let line_capped = match full_text.match_indices('\n').nth(MAX_SOURCE_LINES - 1) {
        Some((end, _)) => &full_text[..end],
        None => full_text,
    };
    let capped = if line_capped.len() <= MAX_SOURCE_BYTES {
        line_capped
    } else {
        // A line ends before a '\n' (ASCII, so a character boundary) at or
        // before byte MAX_SOURCE_BYTES; without one, cut at a character
        // boundary.
        match line_capped.as_bytes()[..=MAX_SOURCE_BYTES]
            .iter()
            .rposition(|&byte| byte == b'\n')
        {
            Some(end) => &line_capped[..end],
            None => &line_capped[..line_capped.floor_char_boundary(MAX_SOURCE_BYTES)],
        }
    };
    let capped = capped.strip_suffix('\r').unwrap_or(capped);

    (capped.to_string(), true)
}
