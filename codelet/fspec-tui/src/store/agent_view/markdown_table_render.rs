//! Box-drawing renderer for markdown pipe-tables (RPC-370).
//!
//! Feature: spec/features/markdown-table-box-drawing-rendering-in-rust-chat-view.feature
//!
//! Ports the TS `renderParsedTable` / `parseAlignment` / `padText`
//! (`src/tui/utils/markdown-table-formatter.ts`) into Rust. Column width
//! uses the display-width proxy, consistent with `text_wrap.rs`.
//! No ANSI/bold codes are emitted — the scrollback wrap path renders plain
//! spans. Entry point `format_markdown_tables` lives in `markdown_tables.rs`.

use unicode_width::UnicodeWidthStr;

/// Per-column horizontal alignment derived from separator-row colons.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Align {
    Left,
    Center,
    Right,
}

pub(super) fn is_table_row(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with('|') && trimmed.ends_with('|') && trimmed.len() >= 2
}

fn is_separator_cell(cell: &str) -> bool {
    let trimmed = cell.trim();
    !trimmed.is_empty() && trimmed.chars().all(|c| c == '-' || c == ':' || c == ' ')
}

fn is_separator_row(cells: &[String]) -> bool {
    !cells.is_empty() && cells.iter().all(|c| is_separator_cell(c))
}

fn parse_cells(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    // Strip leading + trailing '|', then split on '|'.
    let inner = &trimmed[1..trimmed.len() - 1];
    inner.split('|').map(|c| c.trim().to_string()).collect()
}

/// Mirror the TS `parseAlignment`: `:---:` center, `---:` right, `:---`
/// left, otherwise left (default).
fn parse_alignment(separator_cell: &str) -> Align {
    let trimmed = separator_cell.trim();
    let left = trimmed.starts_with(':');
    let right = trimmed.ends_with(':');
    match (left, right) {
        (true, true) => Align::Center,
        (false, true) => Align::Right,
        _ => Align::Left,
    }
}

/// Pad `text` to `width` chars within its column per `align`.
fn pad_text(text: &str, width: usize, align: Align) -> String {
    let pad = width.saturating_sub(text.width());
    if pad == 0 {
        return text.to_string();
    }
    match align {
        Align::Left => {
            let mut s = String::from(text);
            s.extend(std::iter::repeat_n(' ', pad));
            s
        }
        Align::Right => {
            let mut s: String = std::iter::repeat_n(' ', pad).collect();
            s.push_str(text);
            s
        }
        Align::Center => {
            let left = pad / 2;
            let right = pad - left;
            let mut s: String = std::iter::repeat_n(' ', left).collect();
            s.push_str(text);
            s.extend(std::iter::repeat_n(' ', right));
            s
        }
    }
}

/// Render one contiguous pipe block. If it has no separator row it is not a
/// real table and is emitted unchanged; otherwise it becomes a box grid.
pub(super) fn push_table_block(out: &mut String, block: &[&str]) {
    let rows: Vec<Vec<String>> = block.iter().map(|l| parse_cells(l)).collect();

    // Locate the first separator row. Without one, this is not a table.
    let Some(sep_idx) = rows.iter().position(|r| is_separator_row(r)) else {
        for line in block {
            out.push_str(line);
            out.push('\n');
        }
        return;
    };

    let header = if sep_idx == 0 {
        &rows[0]
    } else {
        &rows[sep_idx - 1]
    };
    let cols = header.len();

    // Per-column alignment from the separator row colons.
    let sep_cells = &rows[sep_idx];
    let mut align = vec![Align::Left; cols];
    for (j, a) in align.iter_mut().enumerate() {
        if let Some(cell) = sep_cells.get(j) {
            *a = parse_alignment(cell);
        }
    }

    // Data rows are those after the separator; pad/truncate to header length.
    let mut data: Vec<Vec<String>> = Vec::new();
    for row in rows.iter().skip(sep_idx + 1) {
        let mut cells: Vec<String> = row.iter().take(cols).cloned().collect();
        while cells.len() < cols {
            cells.push(String::new());
        }
        data.push(cells);
    }

    // Column widths = max display width across header + data per column.
    let mut widths = vec![0usize; cols];
    for (j, cell) in header.iter().enumerate() {
        widths[j] = widths[j].max(cell.width());
    }
    for row in &data {
        for (j, cell) in row.iter().enumerate() {
            widths[j] = widths[j].max(cell.width());
        }
    }

    push_border(out, &widths, '┌', '┬', '┐');
    push_cell_row(out, header, &widths, &align);
    push_border(out, &widths, '├', '┼', '┤');
    for row in &data {
        push_cell_row(out, row, &widths, &align);
    }
    push_border(out, &widths, '└', '┴', '┘');
}

/// Emit a border line, e.g. `┌─…─┬─…─┐`.
fn push_border(out: &mut String, widths: &[usize], left: char, mid: char, right: char) {
    out.push(left);
    out.push('─');
    for (j, w) in widths.iter().enumerate() {
        if j > 0 {
            out.push('─');
            out.push(mid);
            out.push('─');
        }
        out.extend(std::iter::repeat_n('─', *w));
    }
    out.push('─');
    out.push(right);
    out.push('\n');
}

/// Emit a `│ a │ b │` content row with each cell padded per alignment.
fn push_cell_row(out: &mut String, cells: &[String], widths: &[usize], align: &[Align]) {
    out.push('│');
    out.push(' ');
    for (j, w) in widths.iter().enumerate() {
        if j > 0 {
            out.push_str(" │ ");
        }
        let text = cells.get(j).map(String::as_str).unwrap_or("");
        let a = align.get(j).copied().unwrap_or(Align::Left);
        out.push_str(&pad_text(text, *w, a));
    }
    out.push(' ');
    out.push('│');
    out.push('\n');
}
