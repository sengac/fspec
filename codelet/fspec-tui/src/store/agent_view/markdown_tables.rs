//! Port of `src/tui/utils/formatMarkdownTables.ts`.
//!
//! Feature: spec/features/agentview-chunkprocessor-parity.feature
//!
//! Runs over the accumulated assistant text at `Done` finalisation to
//! pad every cell in a pipe-table to the column's widest width. Other
//! markdown features are NOT handled here — broader markdown→ratatui
//! rendering is out of scope for RPC-091.

/// Detect every contiguous pipe-table in `input` and pad each cell to
/// its column-wide max width. Non-table lines pass through unchanged.
///
/// A pipe-table is a contiguous run of two or more lines where each
/// non-blank line starts and ends with `'|'`. A separator line of the
/// shape `|---|...|` is detected and preserved with its column widths
/// expanded to match.
pub fn format_markdown_tables(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let lines: Vec<&str> = input.split('\n').collect();

    let mut i = 0;
    while i < lines.len() {
        if is_table_row(lines[i]) {
            // Collect the contiguous block of pipe rows.
            let start = i;
            while i < lines.len() && is_table_row(lines[i]) {
                i += 1;
            }
            let block = &lines[start..i];
            push_padded_block(&mut out, block);
        } else {
            out.push_str(lines[i]);
            out.push('\n');
            i += 1;
        }
    }
    // Strip trailing newline if the original didn't have one.
    if !input.ends_with('\n') && out.ends_with('\n') {
        out.pop();
    }
    out
}

fn is_table_row(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with('|') && trimmed.ends_with('|') && trimmed.len() >= 2
}

fn is_separator_cell(cell: &str) -> bool {
    let trimmed = cell.trim();
    !trimmed.is_empty() && trimmed.chars().all(|c| c == '-' || c == ':' || c == ' ')
}

fn parse_cells(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    // Strip leading + trailing '|', then split on '|'.
    let inner = &trimmed[1..trimmed.len() - 1];
    inner.split('|').map(|c| c.trim().to_string()).collect()
}

fn push_padded_block(out: &mut String, block: &[&str]) {
    let rows: Vec<Vec<String>> = block.iter().map(|l| parse_cells(l)).collect();
    let cols = rows.iter().map(Vec::len).max().unwrap_or(0);
    // Compute the widest content cell per column (ignoring separator rows).
    let mut widths = vec![0usize; cols];
    for row in &rows {
        let is_sep = row.iter().all(|c| is_separator_cell(c));
        if is_sep {
            continue;
        }
        for (j, cell) in row.iter().enumerate() {
            if cell.chars().count() > widths[j] {
                widths[j] = cell.chars().count();
            }
        }
    }
    // Re-emit each row with cells padded to widths[j].
    for row in &rows {
        out.push('|');
        let is_sep = row.iter().all(|c| is_separator_cell(c));
        for (j, cell) in row.iter().enumerate() {
            if is_sep {
                // Render a clean "---" run sized to widths[j].
                out.push(' ');
                for _ in 0..widths[j] {
                    out.push('-');
                }
                out.push(' ');
            } else {
                let pad = widths[j].saturating_sub(cell.chars().count());
                out.push(' ');
                out.push_str(cell);
                for _ in 0..pad {
                    out.push(' ');
                }
                out.push(' ');
            }
            out.push('|');
        }
        // Pad any missing trailing columns (ragged rows).
        for w in widths.iter().take(cols).skip(row.len()) {
            out.push(' ');
            for _ in 0..*w {
                out.push(' ');
            }
            out.push_str(" |");
        }
        out.push('\n');
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn aligns_simple_two_column_table() {
        let input = "| col1 | col2 |\n|---|---|\n| a | bb |";
        let out = format_markdown_tables(input);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 3);
        // Every non-separator row must produce cells of equal width
        // per column (column-major equality is the invariant).
        let row_widths: Vec<Vec<usize>> = lines
            .iter()
            .filter(|l| !l.contains("---"))
            .map(|l| parse_cells(l).iter().map(|c| c.chars().count()).collect())
            .collect();
        assert!(row_widths.len() >= 2);
        for col in 0..row_widths[0].len() {
            // Either the trimmed widths are equal, OR the padded cell
            // widths (cells + surrounding spaces) are equal — both are
            // valid representations of a properly aligned table.
            let trimmed_eq = row_widths.iter().all(|r| r[col] == row_widths[0][col]);
            // Padded length check via the raw line bytes between '|'.
            let padded_widths: Vec<usize> = lines
                .iter()
                .filter(|l| !l.contains("---"))
                .map(|l| {
                    l.split('|')
                        .filter(|c| !c.is_empty())
                        .nth(col)
                        .map(|c| c.chars().count())
                        .unwrap_or(0)
                })
                .collect();
            let padded_eq = padded_widths.iter().all(|w| *w == padded_widths[0]);
            assert!(
                trimmed_eq || padded_eq,
                "column {col} must be aligned across rows; trimmed={row_widths:?} padded={padded_widths:?}"
            );
        }
    }

    #[test]
    fn passes_through_non_table_text_unchanged() {
        let input = "hello world\nnot a table";
        let out = format_markdown_tables(input);
        assert_eq!(out, input);
    }
}
