//! Feature: spec/features/resumed-sessions-refuse-input-until-restored.feature
//!
//! BUG-683: after a checkpoint persistence failure the session was blocked
//! with "selected request requires resolution: {error:#}", which printed the
//! same cause once per error layer (three times) and did not say how to get
//! out. The restore-gate scenarios are in
//! rust/sessions/tests/bug683_resumed_session_restore_gate.rs.

use codelet_cli::session::Session;
use codelet_providers::codex::history::CheckpointError;
use codelet_providers::{ProviderManager, ProviderType};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn a_provider_state_failure_names_its_cause_once_and_says_how_to_recover() -> TestResult {
    // @step Given a request whose error chain repeats the same checkpoint persistence failure in three layers
    let cause = "Codex checkpoint persistence failed: provider session persistence failed: \
        session store unavailable: incompatible provider state: generation rollback";
    let error = anyhow::Error::new(CheckpointError::Persistence(
        "provider session persistence failed: session store unavailable: \
            incompatible provider state: generation rollback"
            .into(),
    ))
    .context(format!("RequestError: {cause}"))
    .context(format!("RequestError: {cause}"));
    assert_eq!(format!("{error:#}").matches(cause).count(), 3);
    let mut session = Session::from_provider_manager(ProviderManager::for_testing(
        ProviderType::Codex,
        Some(200_000),
        Some(4096),
    ));
    // @step When the failure blocks the session
    assert!(codelet_cli::interactive::block_terminal_request_failure(
        &mut session,
        &error
    ));
    let blocked = session
        .ensure_compaction_inference_ready()
        .err()
        .ok_or("the failure did not block the session")?
        .to_string();
    // @step Then the session's error names the failure once
    assert_eq!(
        blocked.matches(cause).count(),
        1,
        "repeated cause: {blocked}"
    );
    assert!(blocked.contains(&format!(
        "selected request requires resolution: RequestError: {cause}"
    )));
    // @step And it says to close the session and resume it to reload its saved state
    assert!(
        blocked.contains("Close this session and resume it to reload its saved state"),
        "no recovery instruction: {blocked}"
    );
    Ok(())
}
