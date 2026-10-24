//! Feature: spec/features/compaction-malformed-generation-recovery.feature

#[path = "cmpct056/support.rs"]
mod durable;
#[path = "support/dag_generation_recovery.rs"]
mod recovery;

use durable::DAG;

#[tokio::test]
#[serial_test::serial]
async fn scenario_fresh_and_incremental_generation_explain_complete_node_syntax() {
    // @step Given a live session with persisted originals
    let server = wiremock::MockServer::start().await;
    let fixture = durable::Fixture::new(&server.uri()).await;
    // @step When fresh and incremental compaction requests reach the provider
    for _ in 0..2 {
        let outcome = recovery::drive(&fixture, &server, vec![recovery::hierarchy()]).await;
        assert!(outcome.result.is_ok(), "{:?}", outcome.result);
        // @step Then both requests require complete closed nodes with exact attributes and nonempty bodies
        let prompt = &outcome.prompts[0];
        assert!(prompt.contains("exactly depth, turns and label"));
        assert!(prompt.contains("nonempty label and body"));
        assert!(prompt.contains("Close every <dag-node> with </dag-node>"));
        assert!(prompt.contains("not prose, a patch, or an unfinished fragment"));
    }
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_malformed_generation_receives_actionable_whole_candidate_correction() {
    // @step Given generated prose unclosed nodes or malformed trailing structure
    let malformed = [
        "summary without nodes".into(),
        "<dag-node depth=\"D1\" turns=\"0-1\" label=\"Work\">unclosed".into(),
        format!("{DAG}<dag-node broken>"),
    ];
    for candidate in malformed {
        let server = wiremock::MockServer::start().await;
        let fixture = durable::Fixture::new(&server.uri()).await;
        // @step When the production builder receives a valid replacement within its correction budget
        let outcome = recovery::drive(&fixture, &server, vec![candidate.clone(), DAG.into()]).await;
        assert!(outcome.result.is_ok(), "{:?}", outcome.result);
        assert_eq!(outcome.prompts.len(), 2);
        // @step Then correction explains complete node syntax and identifies the operation and attempt
        let correction = &outcome.prompts[1];
        assert!(correction.contains("Close every <dag-node> with </dag-node>"));
        assert!(correction.contains("operation="));
        assert!(correction.contains("attempt=2/3"));
        assert!(correction.contains(&fixture.caller.id.to_string()));
        assert!(correction.contains(&candidate));
        let warning = outcome
            .logs
            .lines()
            .find(|line| line.contains("compaction builder output requires correction"))
            .expect("correction diagnostic");
        assert!(
            warning.contains(&format!("target_session_id={}", fixture.caller.id)),
            "{warning}"
        );
        assert!(
            warning.contains("operation=") && warning.contains("attempt=1"),
            "{warning}"
        );
        assert!(
            !warning.contains(&candidate),
            "candidate payload must not be logged"
        );
        // @step And only the complete validated replacement becomes committed memory
        assert_eq!(outcome.summary.as_deref(), Some(DAG));
        assert!(!outcome.blocked);
    }
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_exhausted_malformed_generation_keeps_inference_blocked() {
    // @step Given every generated candidate has malformed node structure
    let server = wiremock::MockServer::start().await;
    let fixture = durable::Fixture::new(&server.uri()).await;
    // @step When the production summary handoff exhausts three candidate attempts
    let outcome = recovery::drive(&fixture, &server, vec!["<dag-node broken>".into(); 3]).await;
    assert_eq!(outcome.prompts.len(), 3);
    // @step Then failure identifies the target operation attempt and syntax remedy
    let error = outcome.result.expect_err("invalid candidates must fail");
    assert!(error.contains(&fixture.caller.id.to_string()), "{error}");
    assert!(error.contains("operation="), "{error}");
    assert!(error.contains("after 3 attempts"), "{error}");
    assert!(
        error.contains("Close every <dag-node> with </dag-node>"),
        "{error}"
    );
    // @step And durable originals and committed memory remain unchanged with ordinary inference blocked
    assert_eq!(outcome.before, outcome.after);
    assert!(outcome.blocked);
}
