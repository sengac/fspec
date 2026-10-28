use crate::fixture::*;
use codelet_core::persistence::*;
use serde_json::json;

pub fn invalid_authority(
    invalid: &str,
) -> Result<
    (
        Fixture,
        codelet_core::persistence::compaction_boundary::CompactionSource,
    ),
    Box<dyn std::error::Error>,
> {
    let mut fixture = fixture()?;
    let source = fixture
        .session
        .capture_durable_compaction(fixture.manifest.id)?;
    let native = history(&fixture.session)?;
    let mut checkpoint = native.snapshot()?.ok_or("checkpoint")?;
    match invalid {
        "absent" => {
            update_session_manifest(fixture.manifest.id, |manifest| {
                manifest.provider_state = None;
                Ok(())
            })?;
        }
        "stale" => {
            checkpoint.generation += 1;
            native.restore(Some(checkpoint))?;
        }
        "selection" => fixture.session.provider_manager_mut().set_model_direct(
            "codex",
            "other-model",
            Some(200_000),
            Some(4096),
            None,
        )?,
        "divergent" => {
            checkpoint.items.clear();
            native.restore(Some(checkpoint))?;
        }
        "model" => {
            checkpoint.model = Some("other-model".into());
            native.restore(Some(checkpoint))?;
        }
        _ => return Err("invalid fixture case".into()),
    }
    Ok((fixture, source))
}

pub fn invalid_bindings_are_refused() -> TestResult {
    let mut fixture = fixture()?;
    let source = fixture
        .session
        .capture_durable_compaction(fixture.manifest.id)?;
    let durable = fixture.session.commit_selected_compaction(
        source,
        uuid::Uuid::new_v4(),
        DAG,
        |_| Ok(()),
    )?;
    let originals = original_bytes(&durable)?;
    for defect in [
        "missing", "version", "durable", "provider", "source", "digest",
    ] {
        let mut altered = durable.clone();
        let checkpoint = &mut altered
            .compaction_commit
            .as_mut()
            .ok_or("boundary")?
            .checkpoint;
        let payload = checkpoint.payload.as_object_mut().ok_or("metadata")?;
        if defect == "missing" {
            payload.remove("_codexCompaction");
        } else {
            let binding = payload
                .entry("_codexCompaction")
                .or_insert_with(|| json!({}));
            match defect {
                "version" => binding["version"] = json!(999),
                "durable" => binding["durableGeneration"] = json!(7),
                "provider" => binding["providerGeneration"] = json!(7),
                "source" => binding["sourceGeneration"] = json!(7),
                "digest" => binding["sourceStateId"] = json!("invalid"),
                _ => return Err("unknown binding defect".into()),
            }
        }
        let mut cold = runtime()?;
        assert!(
            cold.restore_compaction_boundary(&altered).is_err(),
            "{defect} binding authorized cold continuation"
        );
        assert!(cold.ensure_compaction_inference_ready().is_err());
        assert_eq!(original_bytes(&load_session(durable.id)?)?, originals);
        assert_eq!(
            load_session(durable.id)?.compaction_commit,
            durable.compaction_commit
        );
    }
    Ok(())
}
