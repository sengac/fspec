macro_rules! compose_action_project_files {
    ($($collected:tt)*) => {
        compose_action_transcript_selection! {
            $($collected)*
            // RPC-373: BoardView `D` opens the project FOUNDATION.md in the user's
            // default browser via the RPC-372 viewer server (no-op when unavailable).
            OpenFoundation,

            // RPC-374: BoardView `A` opens an attachment picker for the selected work
            // unit; `OpenAttachmentPicker` pushes the dialog and `OpenAttachment(path)`
            // launches the browser at the viewer URL (no-op when unavailable).
            OpenAttachmentPicker,
            OpenAttachment(String),

            // BOARD-022: BoardView `/` opens the work-unit search dialog (client-side
            // filter over the BoardStore snapshot — no RPC surface); selecting a
            // match emits `SelectWorkUnit(id)` which focuses the board's column + row.
            OpenWorkUnitSearch,
            SelectWorkUnit(String),

            // RPC-356: Dual-pane ChangedFilesView (BoardView `F` opens; selection →
            // LoadFileDiff → FileDiffLoaded; Esc → close).
            OpenChangedFilesView,
            CloseChangedFilesView,
            ChangedFilesLoaded(Vec<codelet_rpc_types::ChangedFile>),
            LoadFileDiff(String),
            FileDiffLoaded {
                path: String,
                diff: Option<String>,
            },

            // RPC-364: Three-pane CheckpointsView (BoardView `C` opens; selecting a
            // checkpoint → LoadCheckpointFiles → CheckpointFilesLoaded; selecting a
            // file → LoadCheckpointFileDiff → CheckpointFileDiffLoaded; Esc → close).
            OpenCheckpointsView,
            CloseCheckpointsView,
            CheckpointsLoaded(Vec<codelet_rpc_types::CheckpointInfo>),
            /// TUI-109: per-item checkpoint-enumeration progress frame from the
            /// `checkpoints_progress_rx()` subscriber. Folded into the
            /// CheckpointsView LoadingDialog's counter row ONLY while the list
            /// stage is in flight; stale-dropped once the list has flushed
            /// (a late frame must never re-open the dialog).
            CheckpointsProgress(codelet_rpc_types::CheckpointsProgress),
            LoadCheckpointFiles {
                work_unit_id: String,
                name: String,
            },
            CheckpointFilesLoaded {
                work_unit_id: String,
                name: String,
                files: Vec<codelet_rpc_types::ChangedFile>,
            },
            LoadCheckpointFileDiff {
                work_unit_id: String,
                name: String,
                path: String,
            },
            CheckpointFileDiffLoaded {
                work_unit_id: String,
                name: String,
                path: String,
                diff: Option<String>,
            },

            // RPC-365: Checkpoint restore actions. `r/R` (single file) and `t/T`
            // (all files) open an in-view confirmation dialog; confirming emits a
            // Restore* action which App::dispatch_checkpoints folds into a
            // transport call, then routes a RestoreCheckpointResult back to drive
            // the status dialog (complete | error) and refresh the diff + counts.
            RestoreCheckpointFile {
                work_unit_id: String,
                name: String,
                path: String,
            },
            RestoreCheckpointAll {
                work_unit_id: String,
                name: String,
            },
            RestoreCheckpointResult {
                work_unit_id: String,
                name: String,
                /// `Some(path)` for a single-file restore (drives the diff
                /// reload); `None` for a restore-all.
                path: Option<String>,
                /// `None` on success, `Some(message)` on failure.
                error: Option<String>,
            },
            /// Re-fetch `checkpoint_counts()` so the board header repaints after a
            /// restore changed the working tree.
            RefreshCheckpointCounts,

            // RPC-366: Checkpoint delete actions. `d/D` (single checkpoint) opens a
            // yes/no confirmation; `a/A` (all checkpoints) opens a typed `DELETE
            // ALL` confirmation. Confirming emits a Delete* action which
            // App::dispatch_checkpoint_delete folds into a transport call, then
            // routes a DeleteCheckpointResult back to remove the row / clear the
            // list, clamp the selection (or close the view when empty) and refresh
            // the board counts.
            DeleteCheckpoint {
                work_unit_id: String,
                name: String,
            },
            DeleteAllCheckpoints,
            DeleteCheckpointResult {
                work_unit_id: String,
                name: String,
                /// `true` for a delete-all; `false` for a single-checkpoint delete.
                all: bool,
                /// `None` on success, `Some(message)` on failure.
                error: Option<String>,
            },

        }
    };
}
