//! Feature: spec/features/deep-search-diagnostic-severity.feature

#[path = "../../napi/src/deep_search_handler.rs"]
mod napi_handler;
#[path = "support/deep_search_logging.rs"]
mod support;

#[path = "../src/deep_search_provider_config.rs"]
mod deep_search_provider_config;
pub use codelet_agent_loop::{graph_search_handler, session_search_handler};
pub use codelet_graph as graph;
use serial_test::serial;
use support::{Fixture, Logs, TestResult};
use tracing::instrument::WithSubscriber;

async fn search(fixture: &Fixture, napi: bool) -> Result<String, String> {
    let result = if napi {
        napi_handler::execute_deep_search(
            fixture.directory.path(),
            "Answer locally",
            None,
            2,
            "severity",
            Some("default"),
            0,
            0,
            Some(10000),
            Some(200),
        )
        .await
    } else {
        codelet_agent_loop::deep_search_handler::execute_deep_search(
            fixture.directory.path(),
            "Answer locally",
            None,
            2,
            "severity",
            Some("default"),
            0,
            0,
            Some(10000),
            Some(200),
        )
        .await
    };
    assert_eq!(
        fixture
            .server
            .received_requests()
            .await
            .ok_or("recorded HTTP requests missing")?
            .len(),
        1,
        "handler result: {result:?}"
    );
    result
}

#[tokio::test]
#[serial]
async fn scenario_shared_deep_search_success_stays_out_of_incident_logs() -> TestResult {
    // @step Given a local custom provider that returns a successful answer
    let fixture = Fixture::new(true).await?;
    let logs = Logs::default();
    // @step When the shared DeepSearch handler completes a search
    let result = search(&fixture, false)
        .with_subscriber(logs.subscriber())
        .await?;
    assert_eq!(result, "local answer");
    // @step Then routine entry routing construction and completion records are DEBUG
    logs.assert_routine_debug();
    // @step And the records retain provider model and ephemeral session correlation
    logs.assert_correlation()?;
    // @step And the handler emits no WARN or ERROR records
    logs.assert_no_incidents();
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_napi_deep_search_success_stays_out_of_incident_logs() -> TestResult {
    // @step Given a local custom provider that returns a successful answer
    let fixture = Fixture::new(true).await?;
    let logs = Logs::default();
    // @step When the NAPI DeepSearch handler completes a search
    let result = search(&fixture, true)
        .with_subscriber(logs.subscriber())
        .await?;
    assert_eq!(result, "local answer");
    // @step Then routine entry routing construction and completion records are DEBUG
    logs.assert_routine_debug();
    // @step And the records retain provider model and ephemeral session correlation
    logs.assert_correlation()?;
    // @step And the handler emits no WARN or ERROR records
    logs.assert_no_incidents();
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_shared_deep_search_failures_remain_actionable() -> TestResult {
    // @step Given a local custom provider whose request fails
    let fixture = Fixture::new(false).await?;
    let logs = Logs::default();
    // @step When the shared DeepSearch handler returns the failure
    let error = search(&fixture, false)
        .with_subscriber(logs.subscriber())
        .await
        .err()
        .ok_or("provider request must fail")?;
    assert!(error.contains("DeepSearch sub-agent failed"));
    // @step Then the genuine failure remains visible at WARN or ERROR
    logs.assert_failure_warning();
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_napi_deep_search_failures_remain_actionable() -> TestResult {
    // @step Given a local custom provider whose request fails
    let fixture = Fixture::new(false).await?;
    let logs = Logs::default();
    // @step When the NAPI DeepSearch handler returns the failure
    let error = search(&fixture, true)
        .with_subscriber(logs.subscriber())
        .await
        .err()
        .ok_or("provider request must fail")?;
    assert!(error.contains("DeepSearch sub-agent failed"));
    // @step Then the genuine failure remains visible at WARN or ERROR
    logs.assert_failure_warning();
    Ok(())
}
