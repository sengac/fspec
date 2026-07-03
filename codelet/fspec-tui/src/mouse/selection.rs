//! Pure text-selection region geometry (COPY-002).
//!
//! Feature: spec/features/text-selection-region-model.feature
//!
//! Converts a raw anchor/cursor cell pair into a normalized, ordered set
//! of half-open [`RowSpan`]s regardless of drag direction. This is pure
//! geometry: NO crossterm, ratatui, or io imports — primitive types only.
//!
//! Half-open columns (matching the `rect_contains` convention): a span
//! `cols 2..6` covers columns 2, 3, 4, 5. Middle-row full spans are
//! `0..row_width`.
//!
//! Consumed by COPY-004 (text reconstruction), COPY-005 (highlight), and
//! held live by COPY-006.

/// A terminal cell coordinate (row, column).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub row: u16,
    pub col: u16,
}

/// A raw selection defined by the cell the drag started at (`anchor`)
/// and the cell it currently ends at (`cursor`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub anchor: Cell,
    pub cursor: Cell,
}

/// A half-open run of columns on a single row: `[start_col, end_col)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowSpan {
    pub row: u16,
    pub start_col: u16,
    pub end_col: u16,
}

impl Selection {
    /// Normalize the anchor/cursor pair into ordered, half-open row spans.
    ///
    /// `row_width` (content width) lets the first and middle rows of a
    /// multi-row selection extend to the row end (linewise semantics).
    /// A collapsed selection (`anchor == cursor`) yields an empty vec.
    pub fn spans(&self, row_width: u16) -> Vec<RowSpan> {
        let (start, end) = if (self.anchor.row, self.anchor.col)
            <= (self.cursor.row, self.cursor.col)
        {
            (self.anchor, self.cursor)
        } else {
            (self.cursor, self.anchor)
        };

        if start == end {
            return Vec::new();
        }

        if start.row == end.row {
            return vec![RowSpan {
                row: start.row,
                start_col: start.col,
                end_col: end.col,
            }];
        }

        let mut spans = Vec::new();
        spans.push(RowSpan {
            row: start.row,
            start_col: start.col,
            end_col: row_width,
        });
        for r in (start.row + 1)..end.row {
            spans.push(RowSpan {
                row: r,
                start_col: 0,
                end_col: row_width,
            });
        }
        spans.push(RowSpan {
            row: end.row,
            start_col: 0,
            end_col: end.col,
        });
        spans
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    //! Feature: spec/features/text-selection-region-model.feature
    use super::*;

    #[test]
    fn single_row_forward_selection_yields_one_span() {
        // @step Given a selection with anchor at row 1 column 2 and cursor at row 1 column 6
        let selection = Selection {
            anchor: Cell { row: 1, col: 2 },
            cursor: Cell { row: 1, col: 6 },
        };

        // @step When I request the spans for a row width of 10
        let spans = selection.spans(10);

        // @step Then I get one span: row 1 columns 2 to 6
        assert_eq!(
            spans,
            vec![RowSpan {
                row: 1,
                start_col: 2,
                end_col: 6
            }]
        );
    }

    #[test]
    fn backwards_multi_row_drag_normalizes_start_before_end() {
        // @step Given a selection with anchor at row 3 column 4 and cursor at row 1 column 2
        let selection = Selection {
            anchor: Cell { row: 3, col: 4 },
            cursor: Cell { row: 1, col: 2 },
        };

        // @step When I request the spans for a row width of 10
        let spans = selection.spans(10);

        // @step Then the normalized start is row 1 column 2 and the normalized end is row 3 column 4
        assert_eq!(
            spans,
            vec![
                RowSpan {
                    row: 1,
                    start_col: 2,
                    end_col: 10
                },
                RowSpan {
                    row: 2,
                    start_col: 0,
                    end_col: 10
                },
                RowSpan {
                    row: 3,
                    start_col: 0,
                    end_col: 4
                },
            ]
        );
    }

    #[test]
    fn multi_row_selection_extends_first_and_middle_rows_to_the_row_width() {
        // @step Given a selection with anchor at row 1 column 3 and cursor at row 3 column 5
        let selection = Selection {
            anchor: Cell { row: 1, col: 3 },
            cursor: Cell { row: 3, col: 5 },
        };

        // @step When I request the spans for a row width of 10
        let spans = selection.spans(10);

        // @step Then I get the spans: row 1 columns 3 to 10, row 2 columns 0 to 10, and row 3 columns 0 to 5
        assert_eq!(
            spans,
            vec![
                RowSpan {
                    row: 1,
                    start_col: 3,
                    end_col: 10
                },
                RowSpan {
                    row: 2,
                    start_col: 0,
                    end_col: 10
                },
                RowSpan {
                    row: 3,
                    start_col: 0,
                    end_col: 5
                },
            ]
        );
    }

    #[test]
    fn collapsed_selection_yields_no_spans() {
        // @step Given a selection with anchor at row 2 column 4 and cursor at row 2 column 4
        let selection = Selection {
            anchor: Cell { row: 2, col: 4 },
            cursor: Cell { row: 2, col: 4 },
        };

        // @step When I request the spans for a row width of 10
        let spans = selection.spans(10);

        // @step Then I get zero spans
        assert_eq!(spans, Vec::<RowSpan>::new());
    }

    #[test]
    fn same_row_backwards_drag_normalizes_to_one_ordered_span() {
        // @step Given a selection with anchor at row 1 column 6 and cursor at row 1 column 2
        let selection = Selection {
            anchor: Cell { row: 1, col: 6 },
            cursor: Cell { row: 1, col: 2 },
        };

        // @step When I request the spans for a row width of 10
        let spans = selection.spans(10);

        // @step Then I get one span: row 1 columns 2 to 6
        assert_eq!(
            spans,
            vec![RowSpan {
                row: 1,
                start_col: 2,
                end_col: 6
            }]
        );
    }
}
