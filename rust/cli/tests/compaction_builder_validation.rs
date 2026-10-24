use codelet_cli::compaction_operation::build_validated_dag;
use codelet_core::compaction::DagValidationContext;
use std::collections::VecDeque;

fn context() -> DagValidationContext {
    DagValidationContext {
        persisted_turn_count: 1,
        required_coverage_end: Some(0),
        ..Default::default()
    }
}

fn valid() -> String {
    "<dag-node depth=\"D1\" turns=\"0-0\" label=\"review\">Original evidence.</dag-node>\n<dag-files></dag-files>".into()
}

#[tokio::test]
async fn malformed_builder_output_is_rejected_and_returned_for_correction(
) -> Result<(), Box<dyn std::error::Error>> {
    let invalid = valid().replace(
        "<dag-files></dag-files>",
        "<dag-files>\n- No tracked repository files were Created, Modified, or Deleted.\n</dag-files>",
    );
    let mut outputs = VecDeque::from([invalid.clone(), valid()]);
    let mut prompts = Vec::new();
    let result = build_validated_dag("original task", &context(), |prompt| {
        prompts.push(prompt);
        std::future::ready(outputs.pop_front().ok_or_else(|| "unexpected build".into()))
    })
    .await?;
    assert_eq!(result, valid());
    assert_eq!(prompts.len(), 2);
    assert_eq!(prompts[0], "original task");
    assert!(prompts[1].starts_with("original task"));
    assert!(prompts[1].contains("invalid dag-files line"));
    assert!(prompts[1].contains(&invalid));
    assert!(prompts[1].contains("complete corrected DAG"));
    Ok(())
}

#[tokio::test]
async fn correction_attempts_are_bounded_and_never_salvage_invalid_memory() {
    let mut attempts = 0;
    let result = build_validated_dag("original task", &context(), |_| {
        attempts += 1;
        std::future::ready(Ok("<dag-node>malformed".into()))
    })
    .await;
    assert_eq!(attempts, 3);
    assert!(result.is_err());
}

#[tokio::test]
async fn invalid_authoritative_source_and_provider_errors_do_not_trigger_corrections() {
    let mut source = context();
    source.existing_dag = Some("<dag-node>malformed".into());
    let mut attempts = 0;
    let result = build_validated_dag("original task", &source, |_| {
        attempts += 1;
        std::future::ready(Ok(valid()))
    })
    .await;
    assert_eq!(attempts, 1);
    assert!(result.is_err());
    attempts = 0;
    let result = build_validated_dag("original task", &context(), |_| {
        attempts += 1;
        std::future::ready(Err("provider unavailable".into()))
    })
    .await;
    assert_eq!(attempts, 1);
    assert_eq!(result, Err("provider unavailable".into()));
}
