//! WT-015: isolated worktrees share the main checkout's board through one
//! state-root resolver.
//!
//! Feature: spec/features/isolated-worktrees-share-the-main-checkouts-board-through-one-state-root-resolver.feature
//!
//! The path module in codelet-common resolves every spec path of a
//! directory holding a `.fspec/state-root` pointer: shared state at the
//! pointer's target (the main checkout), branch files at the directory
//! itself. The "worktree" here is a plain directory holding a pointer; the
//! resolver never asks git.

use std::path::{Path, PathBuf};

use codelet_common::fspec_config::load_config_with_dirs;
use codelet_common::spec_lock::SpecLock;
use codelet_common::spec_paths::{
    is_shared_state_path, write_state_root_pointer, ProjectRoots, STATE_ROOT_POINTER,
};
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A main checkout (Cargo.toml marker, spec/ with a project config) and a
/// plain copy of it (its own spec/) holding a pointer to the main checkout.
fn main_and_worktree() -> Result<(tempfile::TempDir, PathBuf, PathBuf), Box<dyn std::error::Error>>
{
    let dir = tempfile::tempdir()?;
    let main = dir.path().join("main");
    std::fs::create_dir_all(main.join("spec"))?;
    std::fs::write(main.join("Cargo.toml"), "[workspace]\n")?;
    std::fs::write(
        main.join("spec").join("fspec-config.json"),
        json!({"agent": "main-checkout"}).to_string(),
    )?;
    let worktree = main.join(".fspec").join("worktrees").join("s1");
    std::fs::create_dir_all(worktree.join("spec").join("features"))?;
    std::fs::write(worktree.join("Cargo.toml"), "[workspace]\n")?;
    std::fs::write(
        worktree.join("spec").join("fspec-config.json"),
        json!({"agent": "stale-worktree-copy"}).to_string(),
    )?;
    write_state_root_pointer(&worktree, &main, "s1")?;
    Ok((dir, main, worktree))
}

fn assert_under(path: &Path, root: &Path, what: &str) {
    assert!(
        path.starts_with(root),
        "{what} resolved to {} instead of under {}",
        path.display(),
        root.display()
    );
}

#[test]
fn every_shared_file_resolves_to_the_main_checkout_and_every_branch_file_to_the_worktree(
) -> TestResult {
    // @step Given a worktree holding a state-root pointer to a main checkout
    let (_dir, main, worktree) = main_and_worktree()?;
    assert!(worktree.join(STATE_ROOT_POINTER).is_file());

    // @step When the spec paths are resolved in the worktree
    let roots = ProjectRoots::resolve(&worktree.join("spec").join("features"))?;
    assert_eq!(roots.content(), worktree.as_path());
    assert_eq!(roots.state(), main.as_path());
    assert_eq!(roots.session_id(), Some("s1"));

    // @step Then the board, epics, prefixes, tags, TAGS.md, foundation, its draft, FOUNDATION.md, attachments, fspec-config.json, schedules, the schedule log, virtual-hook scripts, the spec lock, .fspec/stage-permissions.json and .fspec/blocklist.json resolve under the main checkout
    let main_spec = main.join("spec");
    let shared = [
        (roots.work_units(), "work-units.json"),
        (roots.epics(), "epics.json"),
        (roots.prefixes(), "prefixes.json"),
        (roots.tags(), "tags.json"),
        (roots.tags_md(), "TAGS.md"),
        (roots.foundation(), "foundation.json"),
        (roots.foundation_draft(), "foundation.json.draft"),
        (roots.foundation_md(), "FOUNDATION.md"),
        (roots.attachments_dir(), "attachments"),
        (roots.config(), "fspec-config.json"),
        (roots.schedules(), "schedules.json"),
        (roots.schedule_log(), "schedule-log.jsonl"),
    ];
    for (path, name) in &shared {
        assert_eq!(path, &main_spec.join(name), "{name}");
    }
    assert_eq!(
        roots.virtual_hooks_dir(),
        main_spec.join("hooks").join(".virtual")
    );
    assert_eq!(roots.state_spec_dir(), main_spec);
    assert_eq!(
        roots.stage_permissions(),
        main.join(".fspec").join("stage-permissions.json")
    );
    assert_eq!(
        roots.blocklist(),
        main.join(".fspec").join("blocklist.json")
    );
    let lock = SpecLock::acquire(&worktree.join("spec"))?;
    assert_eq!(lock.dir(), main_spec.canonicalize()?.as_path());
    drop(lock);
    for shared_path in [
        "spec/work-units.json",
        "spec/epics.json",
        "spec/prefixes.json",
        "spec/tags.json",
        "spec/TAGS.md",
        "spec/foundation.json",
        "spec/foundation.json.draft",
        "spec/FOUNDATION.md",
        "spec/attachments/WT-015/notes.md",
        "spec/fspec-config.json",
        "spec/schedules.json",
        "spec/schedule-log.jsonl",
        "spec/hooks/.virtual/WT-015-lint.sh",
        "spec/.fspec-write.lock",
        ".fspec/stage-permissions.json",
        ".fspec/blocklist.json",
    ] {
        assert!(
            is_shared_state_path(Path::new(shared_path)),
            "{shared_path}"
        );
    }

    // @step And features, coverage files and fspec-hooks.json resolve under the worktree
    assert_eq!(roots.content_spec_dir(), worktree.join("spec"));
    assert_eq!(roots.features_dir(), worktree.join("spec").join("features"));
    assert_eq!(
        roots.hooks_config(),
        worktree.join("spec").join("fspec-hooks.json")
    );
    assert_under(&roots.features_dir(), &worktree, "features");
    for branch_path in [
        "spec/features/login.feature",
        "spec/features/login.feature.coverage",
        "spec/fspec-hooks.json",
        "spec/hooks/lint.sh",
        "spec/work-units.json.bak",
        "src/main.rs",
    ] {
        assert!(
            !is_shared_state_path(Path::new(branch_path)),
            "{branch_path}"
        );
    }

    // @step And the fspec config loaded in the worktree is the main checkout's spec/fspec-config.json
    let data = tempfile::tempdir()?;
    let config = load_config_with_dirs(data.path(), &worktree.join("spec"))?;
    assert_eq!(config["agent"], json!("main-checkout"), "{config}");

    // @step And a directory without a pointer resolves every path under itself
    let plain = ProjectRoots::resolve(&main)?;
    assert_eq!(plain.content(), main.as_path());
    assert_eq!(plain.state(), main.as_path());
    assert_eq!(plain.session_id(), None);
    assert_eq!(plain.work_units(), main_spec.join("work-units.json"));
    assert_eq!(plain.features_dir(), main_spec.join("features"));
    Ok(())
}
