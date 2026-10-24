//! Feature: spec/features/truthful-compaction-entry-routes.feature

#[path = "bug213/support.rs"]
mod support;

use codelet_cli::interactive::{begin_compaction_recovery, run_agent_stream, StreamEvent};
use codelet_core::{RigAgent, StreamingTokenDisplay, TokenState};
use rig::client::CompletionClient;
use std::sync::{atomic::AtomicBool, Arc, Mutex};
use std::time::Duration;

#[tokio::test]
#[serial_test::serial]
async fn scenario_every_compaction_entry_route_reports_unknown_work_honestly() -> support::TestResult
{
    let mut measured_placeholders = Vec::new();
    // @step Given a session requiring <route> compaction without measured completed-work units
    for route in ["pre-prompt", "context-limit", "manual"] {
        let mut fixture = support::Fixture::new()?;
        let server = wiremock::MockServer::start().await;
        let busy = Arc::new(AtomicBool::new(false));
        let state = Arc::new(Mutex::new(TokenState::default()));
        // @step When that route begins its real compaction activity
        if route == "context-limit" {
            begin_compaction_recovery(
                &mut fixture.session,
                &state,
                &StreamingTokenDisplay::new(0, 0, 0, 0),
                &mut String::new(),
                &fixture.output,
                false,
            )?;
        }
        if route == "manual" {
            codelet_cli::interactive_helpers::execute_compaction(
                &mut fixture.session,
                busy.clone(),
                None,
            )
            .await?;
        }
        let client: rig::providers::openai::CompletionsClient =
            rig::providers::openai::CompletionsClient::builder()
                .api_key("offline")
                .base_url(server.uri())
                .build()?;
        let agent = RigAgent::new(client.agent("o3").build(), 1);
        let started = fixture.started.clone();
        let observed = tokio::time::timeout(Duration::from_secs(3), async {
            if route == "pre-prompt" {
                tokio::select! {
                    _ = started.notified() => Ok(()),
                    result = run_agent_stream(agent, "resume work", &mut fixture.session,
                        Arc::new(AtomicBool::new(false)), busy.clone(),
                        Arc::new(tokio::sync::Notify::new()), &fixture.output, fixture.id) => {
                        Err(format!("pre-prompt ended before pending builder: {result:?}"))
                    }
                }
            } else {
                tokio::select! {
                    _ = started.notified() => Ok(()),
                    result = codelet_cli::compactor_sub_agent::run_compactor_sub_agent_round(
                        &mut fixture.session, fixture.id, &state, &busy, &fixture.output) => {
                        Err(format!("builder ended before pending signal: {result:?}"))
                    }
                }
            }
        })
        .await?;
        observed?;
        let events = fixture
            .output
            .0
            .lock()
            .map_err(|error| error.to_string())?
            .clone();
        let progress = events
            .iter()
            .filter_map(|event| match event {
                StreamEvent::CompactionProgress(progress) => Some(progress),
                _ => None,
            })
            .collect::<Vec<_>>();
        // @step Then its activity reports the phase actually reached and unknown work counts
        assert!(
            progress
                .iter()
                .any(|progress| progress.phase == "Compaction builder working"),
            "{route}"
        );
        if progress
            .iter()
            .any(|progress| (progress.current, progress.total) != (0, 0))
        {
            measured_placeholders.push(format!("{route}: {progress:?}"));
        }
        // @step And no active-message count, approximate-turn count, or synthetic total becomes work progress
        assert!(!events
            .iter()
            .any(|event| matches!(event, StreamEvent::CompactionComplete(_))));
        assert!(server
            .received_requests()
            .await
            .ok_or("offline request log")?
            .is_empty());
    }
    assert!(
        measured_placeholders.is_empty(),
        "{}",
        measured_placeholders.join("; ")
    );
    Ok(())
}
