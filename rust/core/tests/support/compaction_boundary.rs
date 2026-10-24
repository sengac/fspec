use codelet_core::compaction::{validate_dag_candidate, DagValidationContext};
use codelet_core::persistence::compaction_boundary::*;
use codelet_core::persistence::compaction_commit::CompactionCandidate;
use codelet_core::persistence::SessionManifest;
use uuid::Uuid;

pub fn candidate(
    manifest: &SessionManifest,
) -> Result<CompactionCandidate, Box<dyn std::error::Error>> {
    let source = capture_compaction_source(manifest, "model-a", "dag-v1")?;
    let dag = validate_dag_candidate(
        "<dag-node depth=\"D1\" turns=\"0-0\" label=\"Work\">exact DAG</dag-node>\n<dag-files>\n</dag-files>",
        &DagValidationContext {
            persisted_turn_count: 1,
            required_coverage_end: Some(0),
            ..Default::default()
        },
    )?;
    Ok(CompactionCandidate {
        operation_id: Uuid::new_v4(),
        checkpoint: CompactionCheckpoint {
            provider: manifest.provider.clone(),
            model: "model-a".into(),
            generation: source.generation + 1,
            payload: serde_json::json!({"lineage":"explicit-full-DAG"}),
        },
        source,
        dag,
        input_tokens: 23,
        output_reservation: 8,
        token_provenance: CompactionTokenProvenance::Estimated,
    })
}
