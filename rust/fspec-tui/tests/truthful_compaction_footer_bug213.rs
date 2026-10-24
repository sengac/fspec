//! Feature: spec/features/truthful-compaction-footer.feature

use codelet_fspec_tui::views::agent::footer::SessionFooter;
use codelet_rpc_types::{CompactionProgress, WorkspaceInfo};
use proptest::prelude::*;
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

fn render(progress: &CompactionProgress, pending: usize) -> String {
    let workspace = WorkspaceInfo {
        cwd: "/tmp/work".into(),
        git_branch: Some("main".into()),
    };
    let mut buffer = Buffer::empty(Rect::new(0, 0, 100, 1));
    SessionFooter {
        workspace: Some(&workspace),
        compaction_progress: Some(progress),
        supervisor_pending_count: pending,
        continue_indicator: None,
    }
    .render(buffer.area, &mut buffer);
    (0..100)
        .map(|column| buffer[(column, 0)].symbol())
        .collect::<String>()
        .trim_end()
        .to_owned()
}

proptest! {
    #[test]
    fn scenario_legacy_progress_counts_never_alter_the_activity_presentation(
        current in any::<u32>(), total in any::<u32>(),
    ) {
        // @step Given a compaction phase received through the existing progress wire format
        let progress = CompactionProgress { phase: "Building DAG".into(), current, total };
        // @step When any legacy current and total values are serialized and delivered to the footer
        let encoded = serde_json::to_value(&progress)?;
        let decoded: CompactionProgress = serde_json::from_value(encoded.clone())?;
        // @step Then those fields remain readable by existing snapshot consumers
        prop_assert_eq!(encoded["current"].as_u64(), Some(u64::from(current)));
        prop_assert_eq!(encoded["total"].as_u64(), Some(u64::from(total)));
        prop_assert_eq!(&decoded.phase, "Building DAG");
        // @step And the footer shows the same phase-only activity for every count combination
        let row = render(&decoded, 0);
        prop_assert!(row.contains("[compacting: Building DAG]"), "{row}");
        let baseline = render(&CompactionProgress { current: 0, total: 0, ..progress }, 0);
        prop_assert_eq!(&row, &baseline);
        // @step And zero totals and counts exceeding totals do not produce a fraction or bar
        for (current, total) in [(0, 0), (u32::MAX, 0), (u32::MAX, 1)] {
            let edge = render(&CompactionProgress { phase: "Building DAG".into(), current, total }, 0);
            prop_assert_eq!(&edge, &baseline);
            prop_assert!(!edge.contains('▰') && !edge.contains('▱'));
        }
    }
}

#[test]
fn scenario_truthful_activity_preserves_workspace_chrome_and_supervisor_priority() {
    // @step Given a compacting session with a workspace directory and Git branch
    let progress = CompactionProgress {
        phase: "Building DAG".into(),
        current: 0,
        total: 153,
    };
    // @step When its footer is rendered with no pending supervisor message
    let activity = render(&progress, 0);
    // @step Then phase-only activity appears alongside the workspace directory and branch
    assert!(
        activity.contains("[compacting: Building DAG]"),
        "{activity}"
    );
    assert!(activity.ends_with("/tmp/work [⎇ main]"));
    // @step When a supervisor message is pending
    let supervisor = render(&progress, 2);
    // @step Then the supervisor notification takes priority over compaction activity
    assert!(supervisor.contains("[2 pending from supervisor]"));
    assert!(!supervisor.contains("[compacting:"));
    // @step And the workspace directory and branch remain visible
    assert!(supervisor.ends_with("/tmp/work [⎇ main]"));
    insta::assert_yaml_snapshot!(serde_json::json!({
        "activity": activity.split_whitespace().collect::<Vec<_>>().join(" "),
        "supervisor": supervisor.split_whitespace().collect::<Vec<_>>().join(" "),
    }), @r###"
    activity: "[compacting: Building DAG] /tmp/work [⎇ main]"
    supervisor: "[2 pending from supervisor] /tmp/work [⎇ main]"
    "###);
}
