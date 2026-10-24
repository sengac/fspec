// Feature: spec/features/committed-compaction-watchdog-lifecycle.feature
use codelet_cli::compaction_operation::{CompactionFailure, CompactionOperation};
use std::time::Duration;

#[test]
fn committed_success_resets_operation_attempts() -> Result<(), CompactionFailure> {
    // @step Given a failed compaction operation followed by explicit resolution
    let mut operation = CompactionOperation::new(Duration::from_secs(1), 2);
    operation.begin()?;
    operation.block(CompactionFailure::Builder("provider failure".into()));
    assert!(
        operation.begin().is_err(),
        "blocked work needs explicit resolution"
    );
    operation.resolve();
    let identity = operation.begin()?;
    // @step When the resolved operation commits successfully
    operation.committed(identity)?;
    // @step And another compaction operation begins
    let next = operation.begin()?;
    // @step Then its attempts start fresh
    assert_ne!(next, identity);
    assert_eq!(operation.attempts(), 1);
    Ok(())
}

#[tokio::test]
async fn explicit_resolution_retains_the_operation_deadline() -> Result<(), CompactionFailure> {
    // @step Given an operation whose builder fails before its deadline
    let mut operation = CompactionOperation::new(Duration::from_millis(20), 2);
    let identity = operation.begin()?;
    assert!(matches!(
        operation
            .execute(async { Err::<(), _>(CompactionFailure::Builder("failed".into())) })
            .await,
        Err(CompactionFailure::Builder(_))
    ));
    tokio::time::sleep(Duration::from_millis(30)).await;
    // @step When explicit resolution retries the same operation after its deadline
    operation.resolve();
    // @step Then its identity is unchanged and the retry fails with a deadline outcome
    assert_eq!(operation.begin()?, identity);
    assert!(matches!(
        operation.execute(async { Ok(()) }).await,
        Err(CompactionFailure::Deadline)
    ));
    Ok(())
}

#[tokio::test]
async fn executing_work_is_bounded_by_its_operation_deadline(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given a builder that never completes
    let mut operation = CompactionOperation::new(Duration::from_millis(20), 1);
    operation.begin()?;
    // @step When its declared operation deadline expires
    let result = tokio::time::timeout(
        Duration::from_secs(1),
        operation.execute(std::future::pending::<Result<(), CompactionFailure>>()),
    )
    .await?;
    // @step Then a typed deadline failure is returned
    assert!(matches!(result, Err(CompactionFailure::Deadline)));
    // @step And no synthetic continuation or completion is emitted
    assert!(operation.is_blocked());
    assert!(operation.begin().is_err());
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn blocked_work_cannot_implicitly_start_another_builder(
) -> Result<(), Box<dyn std::error::Error>> {
    use codelet_cli::compactor_sub_agent::{
        run_compactor_sub_agent_round, set_compactor_sub_agent_handler,
    };
    use codelet_cli::interactive::output::{StreamEvent, StreamOutput};
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    };
    struct Output;
    impl StreamOutput for Output {
        fn emit(&self, event: StreamEvent) {
            assert!(!matches!(event, StreamEvent::CompactionComplete { .. }));
        }
    }
    // @step Given a failed runtime compaction operation
    let mut session = codelet_cli::session::Session::from_provider_manager(
        codelet_providers::ProviderManager::for_testing(
            codelet_providers::ProviderType::OpenAI,
            Some(200_000),
            Some(4096),
        ),
    );
    session.provider_manager_mut().set_model_direct(
        "openai",
        "o3",
        Some(200_000),
        Some(4096),
        None,
    )?;
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    codelet_core::persistence::reset_stores_for_tests();
    let mut manifest = codelet_core::persistence::create_session_with_provider(
        "blocked builder",
        directory.path(),
        "openai",
    )?;
    codelet_core::persistence::append_message(&mut manifest, "user", "durable builder input")?;
    let id = manifest.id;
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = calls.clone();
    set_compactor_sub_agent_handler(
        id,
        Some(Arc::new(move |_, _| {
            observed.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { Err("declared builder failure".into()) })
        })),
    );
    let state = Arc::new(Mutex::new(codelet_core::TokenState::default()));
    let busy = Arc::new(AtomicBool::new(false));
    let failed = run_compactor_sub_agent_round(&mut session, id, &state, &busy, &Output).await;
    assert!(
        matches!(&failed, Err(CompactionFailure::Builder(reason)) if reason == "declared builder failure"),
        "the fixture must reach its builder: {failed:?}"
    );
    // @step When automatic recovery invokes the operation again without explicit resolution
    let result = run_compactor_sub_agent_round(&mut session, id, &state, &busy, &Output).await;
    set_compactor_sub_agent_handler(id, None);
    // @step Then the builder is not invoked again
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    // @step And the failure remains blocked without completion
    assert!(matches!(result, Err(CompactionFailure::Blocked)));
    assert!(session.ensure_compaction_inference_ready().is_err());
    assert!(!busy.load(Ordering::Acquire));
    Ok(())
}
