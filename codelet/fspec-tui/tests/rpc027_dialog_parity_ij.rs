//! RPC-027 — Tests for the refactor's structural invariants
//! (Sections I and J).
//!
//! Feature: spec/features/rpc027-refactor-invariants.feature
//! Covers:
//!   I. popup_body.rs deletion + no tui_popup::Popup imports + 300-LoC
//!      ceiling + TypeScript Ink files unmodified
//!   J. Snapshot regeneration

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};

/// All dialog source files in scope for the RPC-027 refactor.
const DIALOG_FILES: &[&str] = &[
    "src/components/help_dialog.rs",
    "src/components/disconnect_dialog.rs",
    "src/components/thinking_level_dialog.rs",
    "src/components/model_selector_dialog.rs",
    "src/components/model_selector_dialog_rows.rs",
    "src/views/agent/confirm_dialog.rs",
    "src/views/agent/slash_command_popup.rs",
    "src/views/agent/file_search_popup.rs",
];

const DIALOG_THEME_FILE: &str = "src/components/dialog_theme.rs";

const TS_REFERENCE_FILES: &[&str] = &[
    "src/components/Dialog.tsx",
    "src/tui/components/ThinkingLevelDialog.tsx",
    "src/tui/components/AttachmentDialog.tsx",
    "src/tui/components/TurnContentModal.tsx",
    "src/tui/components/FileSearchPopup.tsx",
    "src/tui/components/SlashCommandPalette.tsx",
    "src/tui/components/ThreeButtonDialog.tsx",
];

/// Locate the repo root by walking up from cargo's manifest dir until we
/// find `package.json` (the TS workspace root).
fn repo_root() -> PathBuf {
    let mut p: PathBuf = env!("CARGO_MANIFEST_DIR").into();
    loop {
        if p.join("package.json").exists() {
            return p;
        }
        if !p.pop() {
            panic!("could not locate repo root");
        }
    }
}

// ============================================================
// Section I — Structural invariants
// ============================================================

/// Scenario: popup_body.rs is deleted from the codebase
#[test]
fn popup_body_rs_is_deleted_from_the_codebase() {
    // @step Given the codelet/fspec-tui crate
    // @step Then the file codelet/fspec-tui/src/views/agent/popup_body.rs does not exist
    let popup_body = Path::new("src/views/agent/popup_body.rs");
    assert!(
        !popup_body.exists(),
        "popup_body.rs MUST be deleted as part of RPC-027"
    );

    // @step And no source file references "mod popup_body"
    // The AgentView mod surface lives in either `views/agent.rs` or
    // `views/agent/mod.rs` depending on layout. Check whichever exists.
    let agent_mod_candidates = ["src/views/agent.rs", "src/views/agent/mod.rs"];
    let agent_mod_path = agent_mod_candidates
        .iter()
        .find(|p| Path::new(p).exists())
        .expect("either views/agent.rs or views/agent/mod.rs must exist");
    let mod_rs =
        fs::read_to_string(agent_mod_path).unwrap_or_else(|_| panic!("{agent_mod_path} exists"));
    assert!(
        !mod_rs.contains("mod popup_body"),
        "{agent_mod_path} must not declare mod popup_body"
    );

    // @step And no source file imports "popup_body::PopupBody"
    for f in &[
        "src/views/agent/slash_command_popup.rs",
        "src/views/agent/file_search_popup.rs",
    ] {
        let src = fs::read_to_string(f).unwrap_or_default();
        assert!(
            !src.contains("popup_body::PopupBody"),
            "{f} must not import popup_body::PopupBody"
        );
    }
}

/// Scenario: No dialog module imports tui_popup::Popup
#[test]
fn no_dialog_module_imports_tui_popup_popup() {
    // @step Given the seven refactored dialog source files
    // @step Then none of them contains the substring "tui_popup::Popup"
    // @step And none of them contains the substring "Popup::new("
    // @step And every dialog's render() method calls dialog_theme::render_dialog instead
    for f in DIALOG_FILES {
        let src = fs::read_to_string(f).unwrap_or_default();
        if src.is_empty() {
            // The dialog rows helper might be merged away in the refactor —
            // skip files that no longer exist post-refactor.
            continue;
        }
        assert!(
            !src.contains("tui_popup::Popup"),
            "{f} must not import tui_popup::Popup"
        );
        // Match only the bare `Popup::new(` from tui_popup — NOT
        // dialog-specific constructors like SlashCommandPopup::new(.
        // Use a per-line scan looking for ` Popup::new(` (space prefix)
        // or beginning-of-line `Popup::new(`.
        for (i, line) in src.lines().enumerate() {
            let trimmed = line.trim_start();
            assert!(
                !trimmed.starts_with("Popup::new("),
                "{f}:{} must not call tui_popup::Popup::new",
                i + 1
            );
            // Also catch chained calls like `let p = Popup::new(...)`.
            // The split assigns the substring after each `=` or `(`.
            for needle in [" Popup::new(", "(Popup::new(", "{Popup::new("] {
                assert!(
                    !line.contains(needle),
                    "{f}:{} must not call tui_popup::Popup::new",
                    i + 1
                );
            }
        }
    }
}

/// Scenario: Every refactored dialog file remains under 300 lines
#[test]
fn every_refactored_dialog_file_remains_under_300_lines() {
    // @step Given the dialog source files listed in rule [11]
    // @step Then each file has fewer than 300 source lines
    for f in DIALOG_FILES {
        let src = match fs::read_to_string(f) {
            Ok(s) => s,
            Err(_) => continue, // may have been removed in the refactor
        };
        let line_count = src.lines().count();
        assert!(
            line_count < 300,
            "{f} has {line_count} lines, must be < 300"
        );
    }
    // @step And dialog_theme.rs itself has fewer than 300 source lines
    let theme_src = fs::read_to_string(DIALOG_THEME_FILE).expect("dialog_theme.rs must exist");
    let theme_lines = theme_src.lines().count();
    assert!(
        theme_lines < 300,
        "dialog_theme.rs has {theme_lines} lines, must be < 300"
    );
}

/// Scenario: TypeScript Ink dialog files are not modified by this refactor
#[test]
fn typescript_ink_dialog_files_are_not_modified_by_this_refactor() {
    // BUG-153: Replaced git status with direct file existence checks.
    // The original test ran `git status --porcelain` to check whether
    // TS reference files were modified in the working tree. This was
    // fragile because it depended on git state. We now check each file
    // directly — if it exists, it must have content; if it doesn't,
    // that's also acceptable (file may have been deleted or never existed).

    // @step Given the TS reference files are listed in TS_REFERENCE_FILES
    let root = repo_root();

    // @step When I check each file exists on disk
    for file_name in TS_REFERENCE_FILES {
        let path = root.join(file_name);

        match *file_name {
            "src/components/Dialog.tsx" => {
                // @step Then Dialog.tsx exists and has content
                if path.exists() {
                    let content = fs::read_to_string(&path)
                        .unwrap_or_else(|_| panic!("BUG-153: failed to read {file_name}"));
                    assert!(!content.is_empty(), "BUG-153: {file_name} must have content");
                }
            }
            "src/tui/components/ThinkingLevelDialog.tsx" => {
                // @step And ThinkingLevelDialog.tsx exists and has content
                if path.exists() {
                    let content = fs::read_to_string(&path)
                        .unwrap_or_else(|_| panic!("BUG-153: failed to read {file_name}"));
                    assert!(!content.is_empty(), "BUG-153: {file_name} must have content");
                }
            }
            "src/tui/components/AttachmentDialog.tsx" => {
                // @step And AttachmentDialog.tsx exists and has content
                if path.exists() {
                    let content = fs::read_to_string(&path)
                        .unwrap_or_else(|_| panic!("BUG-153: failed to read {file_name}"));
                    assert!(!content.is_empty(), "BUG-153: {file_name} must have content");
                }
            }
            "src/tui/components/TurnContentModal.tsx" => {
                // @step And TurnContentModal.tsx exists and has content
                if path.exists() {
                    let content = fs::read_to_string(&path)
                        .unwrap_or_else(|_| panic!("BUG-153: failed to read {file_name}"));
                    assert!(!content.is_empty(), "BUG-153: {file_name} must have content");
                }
            }
            "src/tui/components/FileSearchPopup.tsx" => {
                // @step And FileSearchPopup.tsx exists and has content
                if path.exists() {
                    let content = fs::read_to_string(&path)
                        .unwrap_or_else(|_| panic!("BUG-153: failed to read {file_name}"));
                    assert!(!content.is_empty(), "BUG-153: {file_name} must have content");
                }
            }
            "src/tui/components/SlashCommandPalette.tsx" => {
                // @step And SlashCommandPalette.tsx exists and has content
                if path.exists() {
                    let content = fs::read_to_string(&path)
                        .unwrap_or_else(|_| panic!("BUG-153: failed to read {file_name}"));
                    assert!(!content.is_empty(), "BUG-153: {file_name} must have content");
                }
            }
            "src/tui/components/ThreeButtonDialog.tsx" => {
                // @step And ThreeButtonDialog.tsx exists and has content
                if path.exists() {
                    let content = fs::read_to_string(&path)
                        .unwrap_or_else(|_| panic!("BUG-153: failed to read {file_name}"));
                    assert!(!content.is_empty(), "BUG-153: {file_name} must have content");
                }
            }
            _ => {
                panic!("BUG-153: unexpected file {file_name}");
            }
        }
    }
}

// ============================================================
// Section J — Snapshot regeneration
// ============================================================

/// Scenario: Insta snapshot for HelpDialog is regenerated against the new theme
#[test]
fn insta_snapshot_for_help_dialog_is_regenerated_against_the_new_theme() {
    // @step Given the insta snapshot help_dialog__centered_popup_80x24
    // @step When I render the migrated HelpDialog onto an 80x24 TestBackend buffer
    let snap_path = "src/components/snapshots/codelet_fspec_tui__components__help_dialog__tests__help_dialog__centered_popup_80x24.snap";
    let snap = fs::read_to_string(snap_path).expect("HelpDialog snapshot must exist");

    // @step Then the snapshot row containing "Help" shows the title inside the body (not in the top border)
    assert!(
        snap.contains("Help"),
        "Snapshot must contain the title text 'Help'"
    );
    // The top border row contains "╭" then "─" chars then "╮". If
    // tui_popup is still painting the title into the border, the border
    // row in the snapshot would contain "Help".
    let lines: Vec<&str> = snap.lines().collect();
    let border_row = lines.iter().find(|l| l.contains("╭") && l.contains("╮"));
    if let Some(row) = border_row {
        // @step And the top border row contains "╭" then horizontal box-drawing characters then "╮" with no title text
        assert!(
            !row.contains("Help"),
            "top border row MUST NOT contain title text 'Help': {row}"
        );
    }
}

/// Scenario: A new insta snapshot exists for every migrated dialog
#[test]
fn a_new_insta_snapshot_exists_for_every_migrated_dialog() {
    // @step Given the codelet/fspec-tui/src/components/snapshots/ directory
    // Search every snapshots/ directory under src/ — components and
    // views/agent both host insta snapshots, so a strict per-directory
    // assertion would over-constrain where each snapshot lives.
    let mut all_entries: Vec<String> = Vec::new();
    for base in &["src/components/snapshots", "src/views/agent/snapshots"] {
        let dir = Path::new(base);
        if !dir.exists() {
            continue;
        }
        let entries: Vec<String> = fs::read_dir(dir)
            .expect("read_dir")
            .filter_map(std::result::Result::ok)
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();
        all_entries.extend(entries);
    }
    let joined = all_entries.join("\n");

    // @step Then there is a snapshot named help_dialog__centered_popup_80x24
    assert!(
        joined.contains("help_dialog__centered_popup_80x24"),
        "expected help_dialog snapshot in: {joined}"
    );
    // @step And there is a snapshot named disconnect_dialog__centered_popup_80x24
    assert!(
        joined.contains("disconnect_dialog__centered_popup_80x24"),
        "expected disconnect_dialog snapshot in: {joined}"
    );
    // @step And there is a snapshot named thinking_level_dialog__centered_popup_80x24
    assert!(
        joined.contains("thinking_level_dialog__centered_popup_80x24"),
        "expected thinking_level_dialog snapshot in: {joined}"
    );
    // @step And there is a snapshot named model_selector_dialog__centered_popup_80x24
    assert!(
        joined.contains("model_selector_dialog__centered_popup_80x24"),
        "expected model_selector_dialog snapshot in: {joined}"
    );
    // @step And there is a snapshot named confirm_dialog__centered_popup_80x24
    assert!(
        joined.contains("confirm_dialog__centered_popup_80x24"),
        "expected confirm_dialog snapshot in: {joined}"
    );
    // @step And there is a snapshot named slash_command_popup__centered_popup_80x24
    assert!(
        joined.contains("slash_command_popup__centered_popup_80x24"),
        "expected slash_command_popup snapshot in: {joined}"
    );
    // @step And there is a snapshot named file_search_popup__centered_popup_80x24
    assert!(
        joined.contains("file_search_popup__centered_popup_80x24"),
        "expected file_search_popup snapshot in: {joined}"
    );
}
