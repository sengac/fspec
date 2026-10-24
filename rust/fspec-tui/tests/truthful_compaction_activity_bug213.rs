//! Feature: spec/features/truthful-compaction-activity.feature

#[path = "bug210_progress/support.rs"]
mod support;

use codelet_cli::interactive::output::StreamOutput;
use codelet_rpc_types::{CommittedContextSnapshot, ContextFillInfo, StreamChunk, TokenTracker};

fn assert_activity(row: &str, phase: &str) {
    assert!(row.contains(&format!("[compacting: {phase}]")), "{row}");
    assert!(!row.contains('▰') && !row.contains('▱'), "{row}");
    assert!(
        !row.contains("0/153") && !row.contains('%') && !row.contains("ETA"),
        "{row}"
    );
}

fn notices(fixture: &support::Fixture) -> String {
    fixture
        .app
        .agent_view_store()
        .session_context_for(&fixture.ids[0])
        .map(|context| context.scrollback.visible_window(1024))
        .unwrap_or_default()
        .iter()
        .flat_map(|chunk| &chunk.lines)
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_a_pending_automatic_builder_reports_activity_rather_than_message_count_progress(
) -> support::TestResult {
    // @step Given an original session with 153 active messages and a real pending compaction builder
    let mut fixture = support::Fixture::new().await?;
    fixture.sessions[0].inner.lock().await.messages = (0..153)
        .map(|index| rig::message::Message::user(format!("active message {index}")))
        .collect();
    // @step When automatic compaction starts that builder
    let task = fixture.start_recovery(0).await?;
    fixture.flush().await?;
    let progress = fixture.sessions[0].get_compaction_progress();
    let row = fixture.footer(0);
    task.abort();
    let _ = task.await;
    // @step Then the original session shows the builder's current activity phase
    let progress = progress.ok_or("actual builder activity missing")?;
    assert_eq!(progress.phase, "Compaction builder working");
    // @step And the producer reports unknown work counts rather than 153 completed-work units
    assert_eq!((progress.current, progress.total), (0, 0));
    // @step And the footer shows no completed-work fraction, percentage, ETA, or determinate bar
    assert_activity(&row, &progress.phase);
    // @step And no test-supplied progress update is needed to make activity visible
    assert!(fixture
        .app
        .agent_view_store()
        .compaction_progress_for(&fixture.ids[0])
        .is_some());
    // @step And the original session retains focus
    assert_eq!(
        fixture.app.agent_view_store().current_session(),
        Some(&fixture.ids[0])
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_live_activity_remains_attached_to_its_original_session() -> support::TestResult {
    // @step Given two open sessions connected through the <transport> backend
    for websocket in [false, true] {
        let mut fixture = if websocket {
            support::Fixture::websocket().await?
        } else {
            support::Fixture::new().await?
        };
        // @step And the background session has a real pending automatic compaction builder
        let task = fixture.start_recovery(1).await?;
        fixture.flush().await?;
        let initial = fixture.sessions[1].get_compaction_progress();
        // @step When the original session reports a reached activity phase
        fixture
            .output(1)
            .emit_compaction_progress("Validating DAG", 0, 0);
        fixture.flush().await?;
        let background = fixture.footer(1);
        task.abort();
        let _ = task.await;
        assert_eq!(
            initial.ok_or("builder activity")?.phase,
            "Compaction builder working"
        );
        // @step Then the viewed session does not show the background session's activity
        assert!(!fixture.footer(0).contains("[compacting:"));
        // @step And the builder does not change session focus
        assert_eq!(
            fixture.app.agent_view_store().current_session(),
            Some(&fixture.ids[0])
        );
        // @step When I focus the compacting session
        fixture.app.agent_view_store_mut().focus_session_index(1);
        // @step Then I see its latest phase-only activity without waiting for a polling interval
        assert_activity(&background, "Validating DAG");
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_terminal_cleanup_removes_activity_without_inventing_success(
) -> support::TestResult {
    // @step Given two sessions with compaction activity through both supported transports
    for websocket in [false, true] {
        for outcome in ["completes", "fails", "is cancelled"] {
            let mut fixture = if websocket {
                support::Fixture::websocket().await?
            } else {
                support::Fixture::new().await?
            };
            fixture.seed_visible_progress();
            // @step When one session's compaction <outcome>
            match outcome {
                "completes" => fixture.sessions[0].set_compaction_progress(None),
                "fails" => fixture
                    .output(0)
                    .emit_compaction_failed("candidate rejected"),
                _ => {
                    let task = fixture.start_recovery(0).await?;
                    task.abort();
                    let _ = task.await;
                }
            }
            fixture.flush().await?;
            // @step Then only that session's activity is cleared in the backend and footer
            assert!(fixture.sessions[0].get_compaction_progress().is_none());
            assert!(!fixture.footer(0).contains("[compacting:"));
            assert!(fixture.sessions[1].get_compaction_progress().is_some());
            assert_activity(&fixture.footer(1), "Summarising");
            // @step And refocusing that session does not restore stale activity
            fixture.app.agent_view_store_mut().focus_session_index(1);
            fixture.app.agent_view_store_mut().focus_session_index(0);
            assert!(!fixture.footer(0).contains("[compacting:"));
            // @step And an activity clear alone adds no successful-compaction notice or reduction badge
            assert!(!notices(&fixture).contains("[compaction]"));
            assert!(fixture
                .app
                .agent_view_store()
                .compaction_reduction_for(&fixture.ids[0])
                .is_none());
            // @step And a committed completion retains its authoritative token basis and available final coverage
            if outcome == "completes" {
                fixture.sessions[0].handle_output(StreamChunk::CommittedCompaction {
                    snapshot: CommittedContextSnapshot {
                        generation: 1,
                        original_tokens: 1000,
                        tokens: TokenTracker {
                            input_tokens: 400,
                            ..Default::default()
                        },
                        context_fill: ContextFillInfo {
                            fill_percentage: 40,
                            effective_tokens: 400.0,
                            threshold: 1000.0,
                            context_window: 2000.0,
                        },
                        covered_turns: Some(12),
                        retained_turns: Some(3),
                        estimated: true,
                        reconciled: false,
                    },
                });
                fixture.flush().await?;
                while let Some(action) = fixture.app.try_recv_action() {
                    fixture.app.dispatch(action);
                }
                let text = notices(&fixture);
                assert!(
                    text.contains("1000 → 400 tokens, estimated, 12 turns summarised"),
                    "{text}"
                );
                assert_eq!(
                    fixture
                        .app
                        .agent_view_store()
                        .compaction_reduction_for(&fixture.ids[0]),
                    Some(60)
                );
            }
        }
    }
    Ok(())
}
