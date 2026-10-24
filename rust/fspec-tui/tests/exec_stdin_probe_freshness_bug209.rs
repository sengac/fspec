// Feature: spec/features/exec-stdin-probe-freshness.feature

use codelet_fspec_tui::Action;
use codelet_rpc_types::{HitlQuestion, HitlRequest, SessionId, StreamChunk};

mod common;
mod exec_stdin_freshness_support;
use exec_stdin_freshness_support::*;

#[tokio::test]
async fn scenario_delayed_pending_snapshot_cannot_restore_a_cleared_command_prompt() -> TestResult {
    for probe in PROBES {
        for drain_clear in [false, true] {
            // @step Given a pending command snapshot is awaiting delivery from either probe path
            let (mut app, backend) = fresh_app().await?;
            let delayed = snapshot(&mut app, &backend, probe, Some(request("old"))).await?;
            // @step When the command clear arrives before the snapshot is delivered
            push(&mut app, None);
            if drain_clear {
                drain(&mut app).await?;
            }
            deliver(&mut app, delayed);
            // @step Then the input prompt remains absent after the delayed delivery
            assert_eq!(visible_command(&app), None);
            drain(&mut app).await?;
            assert_eq!(visible_command(&app), None);
        }
    }
    Ok(())
}

#[tokio::test]
async fn scenario_delayed_empty_snapshot_cannot_hide_a_newly_pushed_prompt() -> TestResult {
    for probe in PROBES {
        // @step Given an empty command snapshot is awaiting delivery from either probe path
        let (mut app, backend) = fresh_app().await?;
        let delayed = snapshot(&mut app, &backend, probe, None).await?;
        // @step When a new command input request arrives before the snapshot is delivered
        push(&mut app, Some(request("new")));
        drain(&mut app).await?;
        deliver(&mut app, delayed);
        // @step Then the new input prompt remains visible after the delayed delivery
        assert_eq!(visible_command(&app), Some("new"));
    }
    Ok(())
}

#[tokio::test]
async fn scenario_delayed_pending_snapshot_cannot_replace_a_newer_command_prompt() -> TestResult {
    for probe in PROBES {
        // @step Given a pending command snapshot is awaiting delivery from either probe path
        let (mut app, backend) = fresh_app().await?;
        let delayed = snapshot(&mut app, &backend, probe, Some(request("old"))).await?;
        // @step When a different command input request arrives before the snapshot is delivered
        push(&mut app, Some(request("new")));
        drain(&mut app).await?;
        deliver(&mut app, delayed);
        // @step Then only the newer command prompt remains visible after the delayed delivery
        assert_eq!(visible_command(&app), Some("new"));
    }
    Ok(())
}

#[tokio::test]
async fn scenario_dismissing_or_answering_a_prompt_invalidates_outstanding_snapshots() -> TestResult
{
    for probe in PROBES {
        for submit in [false, true] {
            // @step Given a pending command snapshot is awaiting delivery from either probe path
            let (mut app, backend) = fresh_app().await?;
            push(&mut app, Some(request("old")));
            drain(&mut app).await?;
            let delayed = snapshot(&mut app, &backend, probe, Some(request("old"))).await?;
            // @step When the user dismisses or successfully answers the input prompt
            app.dispatch(if submit {
                Action::ExecStdinSubmit {
                    agent_session: session(),
                    exec_session: "old".to_owned(),
                    text: "answer".to_owned(),
                }
            } else {
                Action::ExecStdinDismissed {
                    agent_session: session(),
                }
            });
            drain(&mut app).await?;
            assert_eq!(backend.write_exec_stdin_calls(), usize::from(submit));
            deliver(&mut app, delayed);
            // @step Then the input prompt remains absent after the delayed delivery
            assert_eq!(visible_command(&app), None);
        }
    }
    Ok(())
}

#[tokio::test]
async fn scenario_latest_probe_for_a_session_wins_regardless_of_delivery_order() -> TestResult {
    for first in PROBES {
        for second in PROBES {
            for old_is_empty in [false, true] {
                for newest_first in [false, true] {
                    // @step Given two command snapshots for the same session disagree
                    let (mut app, backend) = fresh_app().await?;
                    let old = (!old_is_empty).then(|| request("old"));
                    let newest = old_is_empty.then(|| request("new"));
                    let older = snapshot(&mut app, &backend, first, old).await?;
                    // @step When the newer probe is started before the older snapshot is delivered
                    let newer = snapshot(&mut app, &backend, second, newest).await?;
                    if newest_first {
                        deliver(&mut app, newer);
                        deliver(&mut app, older);
                    } else {
                        deliver(&mut app, older);
                        assert_eq!(visible_command(&app), None);
                        deliver(&mut app, newer);
                    }
                    // @step Then only the newer probe can change the input prompt
                    assert_eq!(visible_command(&app), old_is_empty.then_some("new"));
                }
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn scenario_prompt_updates_in_another_session_do_not_invalidate_a_snapshot() -> TestResult {
    for probe in PROBES {
        // @step Given a pending command snapshot is awaiting delivery for one session
        let (mut app, backend) = fresh_app().await?;
        let delayed = snapshot(&mut app, &backend, probe, Some(request("target"))).await?;
        // @step When another session receives command input requests and clears
        for chunk in [
            StreamChunk::ExecStdinRequest {
                request: request("other"),
            },
            StreamChunk::ExecStdinRequestCleared,
        ] {
            app.dispatch(Action::ChunkReceived(SessionId::new("other"), chunk));
        }
        drain(&mut app).await?;
        deliver(&mut app, delayed);
        // @step Then the first session snapshot still displays its input prompt
        assert_eq!(visible_command(&app), Some("target"));
        assert!(app
            .agent_view_store()
            .exec_stdin_for(&SessionId::new("other"))
            .is_none());
    }
    Ok(())
}

#[tokio::test]
async fn scenario_fresh_snapshots_still_display_and_clear_command_prompts() -> TestResult {
    for probe in PROBES {
        // @step Given no newer command update supersedes either probe path
        let (mut app, backend) = fresh_app().await?;
        // @step When fresh pending and empty snapshots are delivered in order
        let pending = snapshot(&mut app, &backend, probe, Some(request("live"))).await?;
        deliver(&mut app, pending);
        // @step Then the input prompt appears and subsequently disappears
        assert_eq!(visible_command(&app), Some("live"));
        let empty = snapshot(&mut app, &backend, probe, None).await?;
        deliver(&mut app, empty);
        assert_eq!(visible_command(&app), None);
    }
    Ok(())
}

#[tokio::test]
async fn scenario_human_approval_retains_precedence_over_a_fresh_command_snapshot() -> TestResult {
    for probe in PROBES {
        // @step Given a human approval prompt is visible for a session
        let (mut app, backend) = fresh_app().await?;
        app.dispatch(Action::HitlPromptFetched {
            session_id: session(),
            request: HitlRequest {
                questions: vec![HitlQuestion {
                    id: "approval".to_owned(),
                    header: "Approval".to_owned(),
                    question: "Continue?".to_owned(),
                    options: vec![],
                }],
            },
        });
        // @step When a fresh pending command snapshot is delivered from either probe path
        let pending = snapshot(&mut app, &backend, probe, Some(request("live"))).await?;
        deliver(&mut app, pending);
        // @step Then human approval remains visible without a command input prompt
        assert!(app.agent_view_store().hitl_prompt_for(&session()).is_some());
        assert_eq!(visible_command(&app), None);
    }
    Ok(())
}

#[tokio::test]
async fn scenario_finishing_a_model_turn_preserves_a_live_command_prompt() -> TestResult {
    // @step Given an input prompt is visible for a command still running
    let (mut app, _) = fresh_app().await?;
    push(&mut app, Some(request("live")));
    drain(&mut app).await?;
    // @step When the model turn finishes
    app.dispatch(Action::ChunkReceived(session(), StreamChunk::Done));
    drain(&mut app).await?;
    // @step Then the command input prompt remains available
    assert_eq!(visible_command(&app), Some("live"));
    Ok(())
}
