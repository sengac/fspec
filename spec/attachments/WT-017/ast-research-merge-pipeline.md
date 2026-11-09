# WT-017 AST research: the session merge pipeline

Commands (2026-11-07, HEAD 9cf49483f):

- fspec astgrep --pattern 'pub fn get_session_diff($$$) -> $R { $$$ }' --lang rust --path rust/git/src
    rust/git/src/session_result.rs:57:1:pub fn get_session_diff(repo_path: impl AsRef<Path>, session_id: &str) -> Result<SessionResult> {
    
- fspec astgrep --pattern 'pub fn apply_session_changes($$$) -> $R { $$$ }' --lang rust --path rust/git/src
    rust/git/src/session_result.rs:117:1:pub fn apply_session_changes(
    
- fspec astgrep --pattern 'pub fn compute_session_diff($$$) -> $R { $$$ }' --lang rust --path rust/git/src
    rust/git/src/session_diff.rs:30:1:pub fn compute_session_diff(
    
- fspec astgrep --pattern 'pub fn detect_conflicts($$$) -> $R { $$$ }' --lang rust --path rust/git/src
    rust/git/src/session_diff.rs:199:1:pub fn detect_conflicts(
    
- fspec astgrep --pattern 'pub fn apply_snapshot_to_main($$$) -> $R { $$$ }' --lang rust --path rust/git/src
    rust/git/src/session_diff.rs:125:1:pub fn apply_snapshot_to_main(
    
- fspec astgrep --pattern 'pub fn commit_merged_session($$$) -> $R { $$$ }' --lang rust --path rust/git/src
    rust/git/src/merge_commit.rs:41:1:pub fn commit_merged_session(
    

Findings: get_session_diff (inspect and MergeResult lists) and apply_session_changes (conflict detection, apply) both take the base-tree snapshot and the worktree snapshot; commit_merged_session commits only the diff's delta (WT-013). Filtering the two snapshots in those two functions keeps shared paths out of every stage.
