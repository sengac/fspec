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
    fn compaction_bar_renders_five_filled_for_5_of_10() {
        let bar = compaction_bar(5, 10, 10);
        assert_eq!(
            bar,
            "\u{25B0}\u{25B0}\u{25B0}\u{25B0}\u{25B0}\u{25B1}\u{25B1}\u{25B1}\u{25B1}\u{25B1}"
        );
    }

    #[test]
    fn compaction_bar_renders_all_empty_when_total_is_zero() {
        assert_eq!(compaction_bar(0, 0, 10), "\u{25B1}".repeat(10));
    }

    #[test]
    fn compaction_bar_saturates_when_current_exceeds_total() {
        assert_eq!(compaction_bar(99, 10, 10), "\u{25B0}".repeat(10));
    }

    #[test]
    fn build_left_compaction_line_contains_chip_and_bar() {
        let progress = CompactionProgress {
            phase: "summarising messages".to_string(),
            current: 5,
            total: 10,
        };
        let text = line_text(&build_left_compaction_line(&progress));
        assert!(text.contains("[compacting: summarising messages 5/10]"));
        assert!(text.contains(
            "\u{25B0}\u{25B0}\u{25B0}\u{25B0}\u{25B0}\u{25B1}\u{25B1}\u{25B1}\u{25B1}\u{25B1}"
        ));
    }

