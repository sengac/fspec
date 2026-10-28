//! Feature: spec/features/automatic-compaction-progress.feature

#[path = "bug210_progress/support.rs"]
pub mod support;

use codelet_cli::interactive::output::StreamOutput;
use codelet_rpc_types::SessionStatus;
use serial_test::serial;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_automatic_compaction_shows_live_progress_on_the_original_session(
) -> support::TestResult {
    // @step Given I am viewing a session whose unfinished work triggers automatic compaction
    let mut fixture = support::Fixture::new().await?;
    // @step When its compaction builder starts and reports updated progress
    let recovery = fixture.start_recovery(0).await?;
    fixture
        .output(0)
        .emit_compaction_progress("Summarising", 1, 4);
    fixture.flush().await?;
    let initial_footer = fixture.footer(0);
    let initial_status = fixture
        .app
        .agent_view_store()
        .session_status_for(&fixture.ids[0])
        .copied();
    // @step And successive progress updates appear without waiting for a polling interval
    fixture
        .output(0)
        .emit_compaction_progress("Validating DAG", 0, 0);
    fixture.flush().await?;
    let updated_footer = fixture.footer(0);
    recovery.abort();
    let _ = recovery.await;
    // @step Then that session visibly shows the current compaction activity phase without counts or a determinate bar
    assert!(
        initial_footer.contains("[compacting: Summarising]") && !initial_footer.contains('▱'),
        "initial footer: {initial_footer}"
    );
    assert_eq!(
        initial_status,
        Some(SessionStatus::Compacting),
        "automatic builder must visibly start compaction"
    );
    assert!(
        updated_footer.contains("[compacting: Validating DAG]") && !updated_footer.contains('▰'),
        "updated footer: {updated_footer}"
    );
    // @step And the compaction builder does not take focus away from the original session
    assert_eq!(
        fixture.app.agent_view_store().current_session(),
        Some(&fixture.ids[0])
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_background_compaction_progress_stays_attached_to_its_original_session(
) -> support::TestResult {
    // @step Given automatic compaction is running for one open session
    let mut fixture = support::Fixture::websocket().await?;
    let recovery = fixture.start_recovery(1).await?;
    // @step And I am viewing a different open session
    fixture.app.agent_view_store_mut().focus_session_index(0);
    // @step When the compacting session reports updated progress
    fixture.sessions[1].update_compaction_progress("Building DAG".into(), 3, 7);
    fixture.flush().await?;
    recovery.abort();
    let _ = recovery.await;
    // @step Then the viewed session does not show the other session's compaction progress
    assert!(!fixture.footer(0).contains("[compacting:"));
    assert_eq!(
        fixture.app.agent_view_store().current_session(),
        Some(&fixture.ids[0])
    );
    // @step When I focus the compacting session
    fixture.app.agent_view_store_mut().focus_session_index(1);
    // @step Then I see its latest compaction activity phase without counts or a determinate bar
    let footer = fixture.footer(1);
    assert!(
        footer.contains("[compacting: Building DAG]") && !footer.contains('▰'),
        "background footer: {footer}"
    );
    // @step And its compaction builder has not changed which session I was viewing
    assert_eq!(
        fixture.app.agent_view_store().current_session(),
        Some(&fixture.ids[1])
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_compaction_progress_disappears_when_its_operation_ends() -> support::TestResult {
    let mut stale_outcomes = Vec::new();
    for outcome in ["completes", "fails", "is cancelled"] {
        // @step Given two open sessions each have visible automatic compaction progress
        let mut fixture = support::Fixture::new().await?;
        fixture.seed_visible_progress();
        assert!(fixture.footer(0).contains("[compacting:"));
        assert!(fixture.footer(1).contains("[compacting:"));
        // @step When automatic compaction for one session <outcome>
        match outcome {
            "completes" => fixture.output(0).emit_compaction_complete(1000, 400, 0.6),
            "fails" => fixture.output(0).emit_compaction_failed("builder rejected"),
            _ => {
                let recovery = fixture.start_recovery(0).await?;
                recovery.abort();
                let _ = recovery.await;
            }
        }
        fixture.flush().await?;
        assert!(
            fixture.sessions[0].get_compaction_progress().is_none(),
            "backend cleanup for {outcome}"
        );
        // @step Then that session no longer shows compaction progress
        if fixture.footer(0).contains("[compacting:") {
            stale_outcomes.push(format!("stale progress after operation {outcome}"));
        }
        // @step And the other session keeps its own compaction progress
        assert!(fixture.footer(1).contains("[compacting: Summarising]"));
        // @step And focusing the finished session again does not restore stale progress
        fixture.app.agent_view_store_mut().focus_session_index(1);
        fixture.app.agent_view_store_mut().focus_session_index(0);
        if fixture.footer(0).contains("[compacting:") {
            stale_outcomes.push(format!("stale refocused progress after {outcome}"));
        }
    }
    assert!(stale_outcomes.is_empty(), "{}", stale_outcomes.join("; "));
    Ok(())
}
