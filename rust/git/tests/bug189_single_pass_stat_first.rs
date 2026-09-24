//! BUG-189 — single-pass, stat-first git capture.
//!
//! Feature: spec/features/git-capture-single-pass-stat-first.feature
//!
//! Four code-level wastes made every GitState capture O(repo size):
//! fresh cold `gix::open` per collector, double `lookup_entry_by_path`
//! per staged path, `fs::read` + sha1 over EVERY indexed file, and a
//! whole-worktree walk per capture. This file pins:
//!
//! - R1: one capture = one shared `gix::Repository` handle
//!   (`capture_changed_files`), with byte-identical results to the three
//!   separate public collectors;
//! - R2: exactly ONE `lookup_entry_by_path` call site in the whole
//!   staged-detection + change-type path (source-shape) plus behavior
//!   parity for A/M/D letters;
//! - R3: stat-first unstaged check — unchanged files are NOT read or
//!   hashed, genuinely modified files are still detected, and a
//!   stat-restored file is treated as unchanged (git parity).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::fs;
use std::path::Path;
use std::process::Command;

use codelet_git::status::{
    capture_changed_files, get_staged_files_with_change_type, get_unstaged_files,
    get_unstaged_files_with_change_type, get_untracked_files,
};

fn git(repo: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .expect("git");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Rewrite the index entry for `rel_path` to carry `oid` with a
/// ZEROED stat (via `git update-index --cacheinfo`, which writes the
/// entry without re-probing the on-disk file). This forces the
/// stat quick-check (mtime+size vs index stat) to fail on the next
/// capture, exercising the read + hash verification fallback — and
/// with an `oid` that does not match the on-disk content's hash, the
/// fallback reports the file as unstaged.
fn git_update_index_info(repo: &Path, oid: &str, rel_path: &str) {
    git(
        repo,
        &["update-index", "--cacheinfo", "100644", oid, rel_path],
    );
}

/// The blob OID recorded in the committed HEAD tree for `rel_path`.
fn head_oid(repo: &Path, rel_path: &str) -> String {
    let out = Command::new("git")
        .args(["rev-parse", &format!("HEAD:{rel_path}")])
        .current_dir(repo)
        .output()
        .expect("git rev-parse");
    assert!(out.status.success(), "git rev-parse failed");
    String::from_utf8(out.stdout)
        .expect("utf8")
        .trim()
        .to_string()
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A GitState capture opens the repository exactly once across
// all collectors
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn a_git_state_capture_opens_the_repository_exactly_once_across_all_collectors() {
    // @step Given a temp git repo with committed tracked files
    let tmp = common::setup_test_repo();
    let repo = tmp.path();
    // Stage one new file so the staged leg is non-trivial; leave one
    // untracked file so the untracked leg is non-trivial.
    fs::write(repo.join("new-staged.txt"), "staged\n").expect("write");
    git(repo, &["add", "new-staged.txt"]);
    fs::write(repo.join("loose-untracked.txt"), "loose\n").expect("write");

    // @step When a GitState snapshot is captured via the shared-handle path
    let shared = capture_changed_files(repo).expect("capture_changed_files");

    // @step Then the staged, unstaged, and untracked collectors all ran against the SAME gix::Repository handle
    // Source-shape pin: the shared capture path is one function that opens
    // the repo once (via open_repo) and reuses that handle — no collector
    // opens its own cold handle inside it.
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/capture.rs"))
        .expect("read capture.rs");
    assert_eq!(
        src.matches("open_repo").count(),
        1,
        "R1: capture_changed_files must open the repo exactly once and share the handle (found {} open_repo calls)",
        src.matches("open_repo").count()
    );

    // @step And the combined changed_files list is byte-identical to what the three separate public dir-based calls produce
    let mut separate = get_staged_files_with_change_type(repo)
        .expect("staged")
        .into_iter()
        .map(|e| (e.path, e.change_type.as_letter().to_string(), true))
        .collect::<Vec<_>>();
    separate.extend(
        get_unstaged_files_with_change_type(repo)
            .expect("unstaged")
            .into_iter()
            .map(|e| (e.path, e.change_type.as_letter().to_string(), false)),
    );
    separate.extend(
        get_untracked_files(repo)
            .expect("untracked")
            .into_iter()
            .map(|p| (p, "A".to_string(), false)),
    );
    let shared_list = shared
        .iter()
        .map(|f| (f.path.clone(), f.change_type.clone(), f.staged))
        .collect::<Vec<_>>();
    assert_eq!(
        shared_list, separate,
        "R1: shared-handle capture must be byte-identical to the three separate collectors"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Staged-with-change-type issues exactly one tree lookup per
// staged path
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn staged_with_change_type_issues_exactly_one_tree_lookup_per_staged_path() {
    // @step Given a temp git repo with committed tracked files, some of which are staged with differing content
    let tmp = common::setup_test_repo();
    let repo = tmp.path();
    // Modified-and-staged (M):
    fs::write(repo.join("README.md"), "# Test Repository\nchanged\n").expect("modify README");
    git(repo, &["add", "README.md"]);
    // New-and-staged (A):
    fs::write(repo.join("brand-new.txt"), "fresh\n").expect("write new");
    git(repo, &["add", "brand-new.txt"]);
    // Deleted-and-staged (D):
    fs::remove_file(repo.join("src/main.rs")).expect("delete main.rs");
    git(repo, &["add", "src/main.rs"]);

    // @step When get_staged_files_with_change_type is called against that repo
    let staged = get_staged_files_with_change_type(repo).expect("staged with change type");

    // @step Then exactly one tree lookup is issued per staged path (the in-HEAD verdict is reused, not re-derived) and the returned paths + A/M/D letters match the previous double-pass result
    // Source-shape pin: exactly ONE `lookup_entry_by_path` call site in the
    // whole staged-detection + change-type path — the verdict is computed in
    // the staged pass and reused, never re-derived with a second lookup.
    let status_src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/status.rs"))
        .expect("read status.rs");
    let change_type_src =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/change_type.rs"))
            .expect("read change_type.rs");
    let status_lookups = status_src.matches("lookup_entry_by_path").count();
    let change_type_lookups = change_type_src.matches("lookup_entry_by_path").count();
    assert_eq!(
        status_lookups, 1,
        "R2: status.rs must keep exactly one lookup_entry_by_path call site (found {status_lookups})"
    );
    assert_eq!(
        change_type_lookups, 0,
        "R2: change_type.rs must issue ZERO lookup_entry_by_path (verdict is reused from the staged pass; found {change_type_lookups})"
    );

    // @step And a staged NEW file (absent from HEAD) is reported as Added, a staged MODIFIED file as Modified, and a staged file deleted from the working directory as Deleted
    let by_path: std::collections::HashMap<&str, &str> = staged
        .iter()
        .map(|e| (e.path.as_str(), e.change_type.as_letter()))
        .collect();
    assert_eq!(
        by_path.get("README.md"),
        Some(&"M"),
        "staged modified must be M"
    );
    assert_eq!(
        by_path.get("brand-new.txt"),
        Some(&"A"),
        "staged new must be A"
    );
    assert_eq!(
        by_path.get("src/main.rs"),
        Some(&"D"),
        "staged deleted must be D"
    );
    assert_eq!(
        staged.len(),
        3,
        "exactly the three staged files, no more: {staged:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Unstaged detection is stat-first (no full-file read when the
// stat matches)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn unstaged_detection_is_stat_first_no_full_file_read_when_the_stat_matches() {
    // @step Given a temp git repo with committed tracked files whose index stat matches the on-disk mtime+size
    let tmp = common::setup_test_repo();
    let repo = tmp.path();
    // Clean repo: README.md + src/main.rs committed, worktree untouched →
    // index stat matches on-disk mtime+size for every tracked file.

    // @step When get_unstaged_files is called against that repo
    let unstaged = get_unstaged_files(repo).expect("unstaged");

    // @step Then NO file read + hash occurs for a file whose mtime+size match the index stat (the quick stat check short-circuits)
    assert!(
        unstaged.is_empty(),
        "clean tree must report zero unstaged files (found {unstaged:?})"
    );
    // Source-shape pin: the unstaged path must stat (metadata) BEFORE any
    // fs::read, and only read+hash when the stat quick-check fails.
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/status.rs"))
        .expect("read status.rs");
    let metadata_pos = src
        .find("fs::metadata")
        .expect("status.rs must stat the file (fs::metadata)");
    let read_pos = src
        .find("fs::read")
        .expect("status.rs must keep the fs::read hash fallback");
    assert!(
        metadata_pos < read_pos,
        "R3: the stat quick-check (fs::metadata) must precede the fs::read hash fallback in the unstaged path"
    );

    // @step And a file whose content was modified (mtime or size changed) IS still reported as unstaged via the read + hash verification path
    // Modify the file on disk, then rewrite the index entry with a
    // ZEROED stat (cacheinfo does not re-probe the file) — the stat
    // quick-check MUST fail (zeroed index stat never matches on-disk
    // mtime+size), forcing the read + hash verification path. The
    // index entry carries the PRISTINE HEAD OID, so the computed hash
    // of the modified content differs from it → reported as unstaged.
    fs::write(
        repo.join("README.md"),
        "# Test Repository\nmodified for unstaged\n",
    )
    .expect("modify README");
    let pristine_oid = head_oid(repo, "README.md");
    git_update_index_info(repo, &pristine_oid, "README.md");
    let unstaged2 = get_unstaged_files(repo).expect("unstaged after modify");
    assert!(
        unstaged2.iter().any(|p| p == "README.md"),
        "a genuinely modified file (stat differs from index) must still be reported unstaged: {unstaged2:?}"
    );
}

#[test]
fn stat_restored_modified_file_is_treated_as_unchanged_git_parity() {
    // @step Given a temp git repo with committed tracked files whose index stat matches the on-disk mtime+size
    let tmp = common::setup_test_repo();
    let repo = tmp.path();
    // Record the pristine on-disk mtime — the value git recorded into the
    // index stat when the fixture committed (unchanged since).
    let pristine_mtime = fs::metadata(repo.join("README.md"))
        .expect("metadata")
        .modified()
        .expect("mtime");

    // @step When a file's content is modified but its on-disk mtime+size are restored to match the index stat
    // New content that keeps the SAME byte length (18 bytes) as the
    // committed "# Test Repository\n", so restoring the mtime alone
    // makes mtime+size match the index stat exactly.
    fs::write(repo.join("README.md"), "# Test Repositorx\n").expect("modify README");
    assert_eq!(
        fs::metadata(repo.join("README.md"))
            .expect("metadata")
            .len(),
        18,
        "fixture premise: modified content must be the same length as the committed file"
    );
    // Force the on-disk mtime back to the value recorded in the index.
    filetime::set_file_mtime(
        repo.join("README.md"),
        filetime::FileTime::from(pristine_mtime),
    )
    .expect("set restored mtime");

    // @step And a file whose content changed but whose mtime+size were artificially restored to match the index is treated as unchanged (accepted git parity, not a defect)
    // The stat quick-check (mtime + size) matches the index entry, so the
    // file is neither read nor hashed and does NOT surface as unstaged —
    // exactly what `git status` does for a stat-restored file (git trusts
    // the stat until it re-probes the index; the hash fallback is only a
    // defensive net for stat-MISMATCH edge cases, never a second opinion
    // on a stat MATCH).
    let unstaged = get_unstaged_files(repo).expect("unstaged on stat-restored file");
    assert!(
        unstaged.is_empty(),
        "stat match (mtime+size) must short-circuit to unchanged — a stat-restored modified file is accepted git parity, not a defect (found {unstaged:?})"
    );
}
