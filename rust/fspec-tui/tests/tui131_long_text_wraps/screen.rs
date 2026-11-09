//! TUI-131 test support: one rendered frame of the App and the checks the
//! scenarios make on it ("on screen whole", "no cell holds a line break").
//!
//! Feature: spec/features/long-text-wraps-instead-of-being-cut-off.feature
//!
//! A text is "on screen whole" when it can be read from consecutive rows of
//! one column band of the frame: the rows' cells in that band are joined and
//! compared with every whitespace character removed, because a wrap drops
//! the space at the break. The bands are the whole width and every band
//! between two vertical box-drawing columns (pane dividers, dialog and view
//! borders), so text wrapped inside a pane is read without the pane beside
//! it. Dialog checks read only the inside of the dialog's box, so the same
//! text in the view behind it (the scrollback copy of an API error) never
//! counts.

use codelet_fspec_tui::App;
use codelet_test_helpers::TestResult;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

/// Characters that draw a vertical line or a box corner.
const VERTICAL: &[char] = &[
    '│', '┃', '║', '├', '┤', '┼', '┬', '┴', '╭', '╮', '╰', '╯', '┌', '┐', '└', '┘',
];
const TOP_LEFT: &[char] = &['╭', '┌'];
const TOP_RIGHT: &[char] = &['╮', '┐'];
const BOTTOM_LEFT: &[char] = &['╰', '└'];
const BOTTOM_RIGHT: &[char] = &['╯', '┘'];

/// `text` with every whitespace character removed.
pub fn squeeze(text: &str) -> String {
    text.chars().filter(|ch| !ch.is_whitespace()).collect()
}

/// One frame painted by `App::render`, cell by cell.
pub struct Screen {
    cells: Vec<Vec<String>>,
    width: usize,
}

impl Screen {
    /// Render `app` into a `width` x `height` buffer.
    pub fn of(app: &mut App, width: u16, height: u16) -> Self {
        let area = Rect::new(0, 0, width, height);
        let mut buffer = Buffer::empty(area);
        app.render(area, &mut buffer);
        let cells = (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| buffer[(x, y)].symbol().to_string())
                    .collect()
            })
            .collect();
        Self {
            cells,
            width: usize::from(width),
        }
    }

    fn row(&self, y: usize) -> String {
        self.cells[y].concat()
    }

    fn cell_is(&self, x: usize, y: usize, set: &[char]) -> bool {
        let mut chars = self.cells[y][x].chars();
        matches!((chars.next(), chars.next()), (Some(ch), None) if set.contains(&ch))
    }

    /// The frame as text, a line break shown as `⏎`, for failure messages.
    pub fn dump(&self) -> String {
        (0..self.cells.len())
            .map(|y| self.row(y).replace('\n', "⏎"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Some cell's symbol holds a line break.
    pub fn holds_line_break(&self) -> bool {
        self.cells.iter().flatten().any(|cell| cell.contains('\n'))
    }

    /// The (column, row) where `needle` starts on one row.
    pub fn find(&self, needle: &str) -> Option<(usize, usize)> {
        (0..self.cells.len()).find_map(|y| {
            let row = self.row(y);
            let byte = row.find(needle)?;
            let mut seen = 0;
            let x = self.cells[y]
                .iter()
                .take_while(|cell| {
                    seen += cell.len();
                    seen <= byte
                })
                .count();
            Some((x, y))
        })
    }

    /// The text of the cells of row `y` from column `from` up to (not
    /// including) column `to`.
    fn band(&self, y: usize, from: usize, to: usize) -> String {
        self.cells[y][from..to].concat()
    }

    /// `text` is on screen whole in one column band (see the module doc).
    pub fn shown_whole(&self, text: &str) -> bool {
        let needle = squeeze(text);
        let mut edges: Vec<usize> = (0..self.width)
            .filter(|&x| (0..self.cells.len()).any(|y| self.cell_is(x, y, VERTICAL)))
            .collect();
        edges.dedup();
        let starts = std::iter::once(0).chain(edges.iter().map(|x| x + 1));
        starts.filter(|&from| from < self.width).any(|from| {
            edges
                .iter()
                .copied()
                .filter(|&to| to > from)
                .chain(std::iter::once(self.width))
                .any(|to| {
                    let joined: String = (0..self.cells.len())
                        .map(|y| squeeze(&self.band(y, from, to)))
                        .collect();
                    joined.contains(&needle)
                })
        })
    }

    /// The row holding `needle`, from where `needle` starts up to the next
    /// vertical line (a pane divider) or the end of the row, trimmed.
    pub fn row_from(&self, needle: &str) -> Option<String> {
        let (x, y) = self.find(needle)?;
        let end = (x..self.width)
            .find(|&column| self.cell_is(column, y, VERTICAL))
            .unwrap_or(self.width);
        Some(self.band(y, x, end).trim().to_string())
    }

    /// The row of the bottom-left corner closing the box whose top-left
    /// corner is at (`left`, `top`), following its left border down.
    fn bottom_left_below(&self, left: usize, top: usize) -> Option<usize> {
        for y in top + 1..self.cells.len() {
            if self.cell_is(left, y, BOTTOM_LEFT) {
                return Some(y);
            }
            if !self.cell_is(left, y, VERTICAL) {
                return None;
            }
        }
        None
    }

    /// The inside of the smallest box (a dialog) that holds `anchor`.
    pub fn dialog(&self, anchor: &str) -> Option<Dialog> {
        let (ax, ay) = self.find(anchor)?;
        let mut best: Option<(usize, usize, usize, usize)> = None;
        for top in 0..ay {
            for left in 0..=ax {
                if !self.cell_is(left, top, TOP_LEFT) {
                    continue;
                }
                let Some(bottom) = self.bottom_left_below(left, top) else {
                    continue;
                };
                let Some(right) =
                    (left + 1..self.width).find(|&x| self.cell_is(x, bottom, BOTTOM_RIGHT))
                else {
                    continue;
                };
                let encloses = bottom > ay && right > ax;
                if !encloses || !self.cell_is(right, top, TOP_RIGHT) {
                    continue;
                }
                let area = (bottom - top) * (right - left);
                let smaller = match best {
                    Some((t, l, b, r)) => area < (b - t) * (r - l),
                    None => true,
                };
                if smaller {
                    best = Some((top, left, bottom, right));
                }
            }
        }
        let (top, left, bottom, right) = best?;
        Some(Dialog {
            rows: (top + 1..bottom)
                .map(|y| self.band(y, left + 1, right))
                .collect(),
            height: bottom - top + 1,
        })
    }
}

/// The inside of a dialog's box, row by row.
pub struct Dialog {
    rows: Vec<String>,
    /// Rows the box takes, borders included.
    pub height: usize,
}

impl Dialog {
    /// `text` is inside the dialog whole, possibly wrapped over rows.
    pub fn shown_whole(&self, text: &str) -> bool {
        let joined: String = self.rows.iter().map(|row| squeeze(row)).collect();
        joined.contains(&squeeze(text))
    }
}

/// `screen.dialog(anchor)` shows `text` whole.
pub fn dialog_shows(screen: &Screen, anchor: &str, text: &str) -> bool {
    screen
        .dialog(anchor)
        .is_some_and(|dialog| dialog.shown_whole(text))
}

/// Every failed check of a scenario, reported together so one run shows
/// which steps are cut off.
#[derive(Default)]
pub struct Checks(Vec<String>);

impl Checks {
    /// Record `what` (with the frame) unless `ok`.
    pub fn check(&mut self, ok: bool, what: &str, screen: &Screen) {
        if !ok {
            self.0.push(format!("{what}:\n{}", screen.dump()));
        }
    }

    pub fn done(self) -> TestResult {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(self.0.join("\n\n----\n").into())
        }
    }
}
