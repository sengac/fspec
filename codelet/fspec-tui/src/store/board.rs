//! BoardStore — single source of truth for the Kanban board state.
//!
//! Feature: spec/features/rpc012-board-store.feature
//! Card: RPC-012 (parent RPC-002).
//!
//! Plain owned Rust struct held by [`crate::app::App`]. Mutated ONLY on
//! the App task per the RPC-009 single-task tenere pattern — no Mutex /
//! RwLock / atomics. Mirrors `src/tui/store/fspecStore.ts`'s
//! `sessionAttachments` map + the work-units-by-status grouping pattern
//! from `src/tui/components/UnifiedBoardLayout.tsx`.

use std::collections::HashMap;

use codelet_rpc_types::{CheckpointCounts, SessionId, WorkUnitInfo};

/// Canonical column order — must match `STATES` in
/// `src/tui/components/UnifiedBoardLayout.tsx`.
pub const COLUMN_ORDER: [&str; 7] = [
    "backlog",
    "specifying",
    "testing",
    "implementing",
    "validating",
    "done",
    "blocked",
];

/// Returns the index of `column` within [`COLUMN_ORDER`], or `None` if
/// the string does not name one of the seven canonical statuses.
pub fn column_index(column: &str) -> Option<usize> {
    COLUMN_ORDER.iter().position(|c| *c == column)
}

/// Work-units state + selection + session-attachments — the Rust analogue
/// of `useFspecStore` from `src/tui/store/fspecStore.ts`.
///
/// Field shape: locked by RPC-012 rule [1] / [2]. Public methods are the
/// only mutation surface; the App task calls them inside `App::dispatch`.
#[derive(Debug, Default)]
pub struct BoardStore {
    pub(super) work_units: Vec<WorkUnitInfo>,
    pub(super) by_column: HashMap<String, Vec<usize>>,
    focused_column: usize,
    pub(super) selected_index_per_column: HashMap<String, usize>,
    /// RPC-016: per-column scroll offset — the index of the first
    /// work unit in the column-content viewport. `0` for every
    /// column until move_selection / scroll_focused_column / End
    /// advances it.
    pub(super) scroll_offsets: HashMap<String, usize>,
    session_attachments: HashMap<String, SessionId>,
    last_changed_id: Option<String>,
    /// RPC-015: aggregate manual + auto checkpoint counts populated by
    /// `App::bootstrap` via `Action::CheckpointCountsLoaded`. The
    /// BoardView header reads this on every render to paint
    /// `Checkpoints: None` or `Checkpoints: N Manual, M Auto`. Defaults
    /// to `{0,0}` so the empty state paints `Checkpoints: None` until
    /// the bootstrap RPC returns.
    checkpoint_counts: CheckpointCounts,
}

impl BoardStore {
    /// Replace the full work-units list. Re-groups indices by status,
    /// then re-anchors each column's selection to the previously
    /// selected work-unit id (so the cursor follows an item that moved
    /// within its column — e.g. after `[` / `]` reorder). Falls back to
    /// clamping the prior numeric index when the previously selected id
    /// no longer lives in that column. Unknown statuses (not in
    /// [`COLUMN_ORDER`]) are silently dropped.
    pub fn replace_work_units(&mut self, units: Vec<WorkUnitInfo>) {
        // Capture id-of-selection per column BEFORE we rebuild so we
        // can re-anchor the cursor to follow the moved item. Per
        // RPC-017 rule [6]: "the focused-column selection follows the
        // moved unit (the unit stays selected, not the row position)".
        let mut prior_selected_id: HashMap<String, String> = HashMap::new();
        for column in COLUMN_ORDER {
            let Some(indices) = self.by_column.get(column) else {
                continue;
            };
            let sel = self.selected_index_for(column);
            if let Some(unit_idx) = indices.get(sel) {
                if let Some(unit) = self.work_units.get(*unit_idx) {
                    prior_selected_id.insert(column.to_string(), unit.id.clone());
                }
            }
        }

        self.work_units = units;
        self.by_column.clear();
        for column in COLUMN_ORDER {
            self.by_column.insert(column.to_string(), Vec::new());
        }
        for (idx, unit) in self.work_units.iter().enumerate() {
            if let Some(slot) = self.by_column.get_mut(&unit.status) {
                slot.push(idx);
            }
        }
        // Re-anchor selection per column: prefer the previously
        // selected id when it still lives in this column; otherwise
        // clamp the prior numeric index to the new length.
        for column in COLUMN_ORDER {
            let len = self
                .by_column
                .get(column)
                .map(|v| v.len())
                .unwrap_or(0);
            if len == 0 {
                self.selected_index_per_column
                    .insert(column.to_string(), 0);
                continue;
            }
            if let Some(prior_id) = prior_selected_id.get(column) {
                let indices = self
                    .by_column
                    .get(column)
                    .expect("column slot inserted above");
                let new_pos = indices
                    .iter()
                    .position(|i| &self.work_units[*i].id == prior_id);
                if let Some(pos) = new_pos {
                    self.selected_index_per_column
                        .insert(column.to_string(), pos);
                    continue;
                }
            }
            if let Some(sel) = self.selected_index_per_column.get_mut(column) {
                if *sel >= len {
                    *sel = len.saturating_sub(1);
                }
            }
        }
    }

    /// Borrow the units currently in `column` in display order. Returns
    /// an empty `Vec` when `column` is not one of [`COLUMN_ORDER`].
    pub fn column_units(&self, column: &str) -> Vec<&WorkUnitInfo> {
        let Some(indices) = self.by_column.get(column) else {
            return Vec::new();
        };
        indices.iter().map(|i| &self.work_units[*i]).collect()
    }

    /// Currently focused column name. Always one of [`COLUMN_ORDER`].
    pub fn focused_column(&self) -> &'static str {
        COLUMN_ORDER[self.focused_column]
    }

    /// Index (0..7) of the currently focused column.
    pub fn focused_column_index(&self) -> usize {
        self.focused_column
    }

    /// Move focus to the next column (wraps at the end).
    pub fn focus_next_column(&mut self) {
        self.focused_column = (self.focused_column + 1) % COLUMN_ORDER.len();
    }

    /// Move focus to the previous column (wraps at index 0).
    pub fn focus_prev_column(&mut self) {
        self.focused_column = if self.focused_column == 0 {
            COLUMN_ORDER.len() - 1
        } else {
            self.focused_column - 1
        };
    }

    /// Set focus to the named column. No-op when `column` is not in
    /// [`COLUMN_ORDER`].
    pub fn set_focused_column(&mut self, column: &str) {
        if let Some(idx) = column_index(column) {
            self.focused_column = idx;
        }
    }

    /// The current selection inside `column`. Defaults to 0 when no
    /// selection has been recorded.
    pub fn selected_index_for(&self, column: &str) -> usize {
        self.selected_index_per_column
            .get(column)
            .copied()
            .unwrap_or(0)
    }

    /// Set the selection inside `column`. Clamps to the column's length.
    pub fn set_selected_index_for(&mut self, column: &str, index: usize) {
        let len = self
            .by_column
            .get(column)
            .map(|v| v.len())
            .unwrap_or(0);
        let clamped = if len == 0 {
            0
        } else {
            index.min(len.saturating_sub(1))
        };
        self.selected_index_per_column
            .insert(column.to_string(), clamped);
    }

    /// Borrow the currently selected work unit in the focused column,
    /// or `None` when the focused column is empty.
    pub fn selected_work_unit(&self) -> Option<&WorkUnitInfo> {
        let column = self.focused_column();
        let units = self.by_column.get(column)?;
        let idx = self.selected_index_for(column);
        let unit_idx = units.get(idx)?;
        self.work_units.get(*unit_idx)
    }

    /// Record an attachment between a work-unit id and a session id —
    /// mirrors `useFspecStore.attachSession` in the TS reference.
    pub fn attach_session(&mut self, work_unit_id: &str, session: SessionId) {
        // Mirror TS attachSession: remove any other work-unit currently
        // pointing at this session (session migration).
        let prior_for_session: Vec<String> = self
            .session_attachments
            .iter()
            .filter_map(|(k, v)| {
                if v == &session && k != work_unit_id {
                    Some(k.clone())
                } else {
                    None
                }
            })
            .collect();
        for k in prior_for_session {
            self.session_attachments.remove(&k);
        }
        self.session_attachments
            .insert(work_unit_id.to_string(), session);
    }

    /// Look up an attached session by work-unit id.
    pub fn session_for(&self, work_unit_id: &str) -> Option<&SessionId> {
        self.session_attachments.get(work_unit_id)
    }

    /// Detach any session currently attached to `work_unit_id`.
    pub fn detach_session(&mut self, work_unit_id: &str) {
        self.session_attachments.remove(work_unit_id);
    }

    /// Mark the most-recently-changed work unit id (drives the `⏩`
    /// animation in the eventual rich render — not used by this slice's
    /// placeholder render).
    pub fn mark_last_changed(&mut self, id: &str) {
        self.last_changed_id = Some(id.to_string());
    }

    /// Borrow the most-recently-changed work-unit id, if any.
    pub fn last_changed(&self) -> Option<&str> {
        self.last_changed_id.as_deref()
    }

    /// Total work-unit count across all columns.
    pub fn total_count(&self) -> usize {
        self.work_units.len()
    }

    /// RPC-015: read the aggregate manual + auto checkpoint counts.
    /// Returns the default `{0,0}` until the bootstrap RPC populates
    /// the field via `Action::CheckpointCountsLoaded`.
    pub fn checkpoint_counts(&self) -> CheckpointCounts {
        self.checkpoint_counts
    }

    /// RPC-015: replace the checkpoint counts. Called from
    /// `App::dispatch` on `Action::CheckpointCountsLoaded`.
    pub fn set_checkpoint_counts(&mut self, counts: CheckpointCounts) {
        self.checkpoint_counts = counts;
    }

    // RPC-016: per-column scroll viewport + viewport-aware selection
    // methods live in `super::board_viewport` so this file stays under
    // the 300 LoC ceiling. The `pub(super)` field visibility above
    // grants the sibling module access to the underlying maps.
}
