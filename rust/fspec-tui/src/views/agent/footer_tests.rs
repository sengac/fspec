//! Unit tests for `footer.rs` (moved out to keep the widget file under
//! the 300-LoC source-shape ceiling).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

fn line_text(line: &Line<'_>) -> String {
    line.spans
        .iter()
        .map(|s| s.content.as_ref())
        .collect::<String>()
}

#[test]
fn shorten_with_home_replaces_home_prefix() {
    std::env::set_var("HOME", "/Users/rquast");
    assert_eq!(
        shorten_with_home("/Users/rquast/projects/fspec"),
        "~/projects/fspec"
    );
}

#[test]
fn shorten_with_home_leaves_other_paths_alone() {
    std::env::set_var("HOME", "/Users/rquast");
    assert_eq!(shorten_with_home("/tmp/scratch"), "/tmp/scratch");
}

#[test]
fn build_right_line_uses_alternative_key_glyph() {
    let ws = WorkspaceInfo {
        cwd: "/tmp/scratch".to_string(),
        git_branch: Some("main".to_string()),
    };
    let text = line_text(&build_right_line(&ws));
    assert!(text.contains("[\u{2387} main]"));
    assert!(!text.contains("\u{2325}"));
}

#[test]
fn build_right_line_omits_branch_suffix_when_none() {
    let ws = WorkspaceInfo {
        cwd: "/tmp/scratch".to_string(),
        git_branch: None,
    };
    let line = build_right_line(&ws);
    assert_eq!(line.spans.len(), 1);
    assert_eq!(line.spans[0].content.as_ref(), "/tmp/scratch");
}

#[test]
fn build_right_line_paints_cwd_dim_and_branch_cyan() {
    let ws = WorkspaceInfo {
        cwd: "/tmp/scratch".to_string(),
        git_branch: Some("main".to_string()),
    };
    let line = build_right_line(&ws);
    assert_eq!(line.spans[0].style.fg, Some(Color::DarkGray));
    assert_eq!(line.spans[1].style.fg, Some(Color::Cyan));
}

#[test]
fn build_left_compaction_line_contains_only_activity() {
    let progress = CompactionProgress {
        phase: "summarising messages".to_string(),
        current: 5,
        total: 10,
    };
    let text = line_text(&build_left_compaction_line(&progress));
    assert_eq!(text, "[compacting: summarising messages]");
}
