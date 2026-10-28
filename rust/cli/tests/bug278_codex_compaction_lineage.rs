//! Feature: spec/features/codex-durable-compaction-lineage.feature
#[path = "fixtures/bug278_binding.rs"]
mod binding;
#[path = "fixtures/bug278.rs"]
mod fixture;
use codelet_core::persistence::*;
use fixture::*;
use uuid::Uuid;

#[tokio::test]
#[serial_test::serial]
async fn scenario_first_and_successive_codex_compactions_preserve_independent_authoritative_generations(
) -> TestResult {
    // @step Given a selected Codex runtime with genuine persisted checkpoint and tool outcomes
    let mut fixture = fixture()?;
    assert_eq!(history(&fixture.session)?.generation()?, 1);
    assert!(fixture.manifest.compaction_commit.is_none());
    assert!(history(&fixture.session)?
        .prepare_generation(1, MODEL)
        .is_err());
    // @step When first and successive admitted compactions publish and restore cold
    for durable_generation in 1..=2 {
        let source = fixture
            .session
            .capture_durable_compaction(fixture.manifest.id)?;
        let call = format!("call-{durable_generation}");
        fixture.manifest = load_session(fixture.manifest.id)?;
        append_pair(&mut fixture.manifest, &call, true)?;
        checkpoint_pair(&fixture.session, &call)?;
        let originals = original_bytes(&fixture.manifest)?;
        let dag = format!("<dag-node depth=\"D1\" turns=\"0-{}\" label=\"Work\">original task; genuine completed effects retained</dag-node>", source.prefix.len()-1);
        let mut published = None;
        let committed = fixture.session.commit_selected_compaction(
            source,
            Uuid::new_v4(),
            &dag,
            |boundary| {
                let durable = load_session(boundary.identity.session_id)?;
                assert_eq!(durable.compaction_commit.as_ref(), Some(boundary));
                published = Some(boundary.identity.clone());
                Ok(())
            },
        )?;
        // @step Then durable and provider generations advance independently and continuation retains genuine outcomes exactly once
        let boundary = committed.compaction_commit.as_ref().ok_or("boundary")?;
        assert_eq!(boundary.identity.generation, durable_generation);
        assert_eq!(published.as_ref(), Some(&boundary.identity));
        assert_eq!(
            boundary.checkpoint.payload["codexContinuation"]["generation"],
            durable_generation + 1
        );
        assert!(!boundary.publication_pending);
        assert_eq!(original_bytes(&committed)?, originals);
        fixture.session.ensure_compaction_inference_ready()?;
        reset_stores_for_tests();
        let durable = load_session(committed.id)?;
        let envelopes = get_session_message_envelopes(committed.id)?;
        let mut cold = runtime()?;
        cold.preflight_compaction_restore(&durable, &envelopes)?;
        cold.restore_compaction_boundary(&durable)?;
        cold.restore_provider_state(&envelopes)?;
        cold.ensure_compaction_inference_ready()?;
        let mut continuation_messages = vec![rig::message::Message::user(
            &durable.compaction.as_ref().ok_or("DAG")?.summary,
        )];
        continuation_messages.extend(pair(&call));
        let mut continuation = wire(&continuation_messages)?;
        history(&cold)?.prepare(&mut continuation)?;
        assert_eq!(
            continuation["input"]
                .as_array()
                .ok_or("input")?
                .iter()
                .filter(|item| item["type"] == "function_call_output")
                .count(),
            1
        );
        assert!(continuation.to_string().contains("genuine effect=1"));
        assert_eq!(original_bytes(&load_session(committed.id)?)?, originals);
        cold.bind_provider_persistence(committed.id)?;
        continue_cold(&cold, &fixture.session.messages).await?;
        assert_eq!(
            std::fs::read_to_string(fixture.directory.path().join(format!("effect-{call}")))?,
            "genuine effect=1"
        );
        cold.restore_compaction_boundary(&load_session(committed.id)?)?;
        assert_eq!(history(&cold)?.generation()?, durable_generation + 1);
        cold.messages = fixture.session.messages.clone();
        fixture.session = cold;
    }
    Ok(())
}

#[test]
#[serial_test::serial]
fn scenario_unproven_or_incompatible_codex_checkpoint_authority_is_refused() -> TestResult {
    // @step Given selected Codex checkpoint evidence that is stale absent divergent for another model or lacks a committed generation binding
    for invalid in ["absent", "stale", "divergent", "model", "selection"] {
        let (mut fixture, source) = binding::invalid_authority(invalid)?;
        // @step When the runtime attempts an admitted compaction
        // @step Then original history stays unchanged without promotion publication or inference
        let error = assert_refused(&mut fixture, source)?;
        assert!(
            error.contains("lineage mismatch") || error.contains("identity changed"),
            "{invalid}: {error}"
        );
    }
    binding::invalid_bindings_are_refused()?;
    Ok(())
}

#[test]
#[serial_test::serial]
fn scenario_missing_originals_or_genuine_outcomes_cannot_authorize_compaction() -> TestResult {
    // @step Given a selected Codex source with missing original or tool outcome evidence
    for missing in ["original", "outcome"] {
        let mut fixture = fixture()?;
        let source = fixture
            .session
            .capture_durable_compaction(fixture.manifest.id)?;
        if missing == "original" {
            update_session_manifest(fixture.manifest.id, |manifest| {
                manifest.messages[0].message_id = Uuid::new_v4();
                Ok(())
            })?;
        } else {
            append_pair(&mut fixture.manifest, "missing-outcome", false)?;
            checkpoint_pair(&fixture.session, "missing-outcome")?;
        }
        // @step When the runtime attempts an admitted compaction
        // @step Then the missing evidence blocks promotion publication and inference without fabricated success
        let error = assert_refused(&mut fixture, source)?;
        assert!(
            error.contains("missing committed message") || error.contains("ancestry"),
            "{missing}: {error}"
        );
    }
    Ok(())
}

#[test]
#[serial_test::serial]
fn scenario_mutation_during_admission_cannot_promote_a_different_candidate() -> TestResult {
    // @step Given a selected Codex candidate with authoritative lineage
    let mut fixture = fixture()?;
    let source = fixture
        .session
        .capture_durable_compaction(fixture.manifest.id)?;
    let native = history(&fixture.session)?;
    let before = serde_json::to_value(load_session(fixture.manifest.id)?)?;
    let mut published = false;
    let mutated = std::cell::Cell::new(false);
    // @step When the provider checkpoint changes during selected request admission
    let result = fixture.session.commit_selected_compaction_guarded(
        source,
        Uuid::new_v4(),
        DAG,
        || {
            mutated.set(true);
            let mut checkpoint = native
                .snapshot()
                .map_err(|error| {
                    codelet_providers::request_admission::RequestAdmissionError::Preparation(
                        error.to_string(),
                    )
                })?
                .ok_or_else(|| {
                    codelet_providers::request_admission::RequestAdmissionError::Preparation(
                        "missing checkpoint".into(),
                    )
                })?;
            checkpoint.items.clear();
            native.restore(Some(checkpoint)).map_err(|error| {
                codelet_providers::request_admission::RequestAdmissionError::Preparation(
                    error.to_string(),
                )
            })
        },
        |_| {
            published = true;
            Ok(())
        },
    );
    // @step Then the changed candidate is rejected without promotion publication or inference
    assert!(mutated.get(), "mutation callback was not exercised");
    let error = result.err().ok_or("mutated checkpoint promoted")?;
    assert!(
        error.to_string().contains("runtime checkpoint differs"),
        "{error}"
    );
    assert!(!published);
    assert_eq!(
        serde_json::to_value(load_session(fixture.manifest.id)?)?,
        before
    );
    assert!(fixture.session.ensure_compaction_inference_ready().is_err());
    Ok(())
}

#[test]
#[serial_test::serial]
fn scenario_failed_codex_publication_reconciles_only_the_promoted_identity() -> TestResult {
    // @step Given a selected Codex candidate with authoritative lineage
    let mut fixture = fixture()?;
    let source = fixture
        .session
        .capture_durable_compaction(fixture.manifest.id)?;
    // @step When publication fails after durable promotion
    let error = fixture
        .session
        .commit_selected_compaction(source, Uuid::new_v4(), DAG, |_| {
            Err("publication unavailable".into())
        })
        .err()
        .ok_or("publication failure")?;
    // @step Then inference stays blocked until the exact promoted identity reconciles without another generation
    assert!(
        matches!(&error, codelet_core::persistence::compaction_commit::CompactionCommitError::Publication {reason, ..} if reason == "publication unavailable"),
        "{error}"
    );
    reconcile_failure(&mut fixture, error)?;
    Ok(())
}

#[cfg(unix)]
#[test]
#[serial_test::serial]
fn scenario_lost_codex_durability_acknowledgement_retains_exact_ownership() -> TestResult {
    use std::os::unix::fs::PermissionsExt;
    // @step Given a selected Codex candidate with authoritative lineage
    let mut fixture = fixture()?;
    let source = fixture
        .session
        .capture_durable_compaction(fixture.manifest.id)?;
    let sessions = fixture.directory.path().join("sessions");
    let permissions = std::fs::metadata(&sessions)?.permissions();
    // @step When manifest promotion succeeds but directory synchronization fails
    let result = fixture.session.commit_selected_compaction_guarded(
        source,
        Uuid::new_v4(),
        DAG,
        || {
            std::fs::set_permissions(&sessions, std::fs::Permissions::from_mode(0o300)).map_err(
                |error| {
                    codelet_providers::request_admission::RequestAdmissionError::Preparation(
                        error.to_string(),
                    )
                },
            )
        },
        |_| Err("publication must not run after lost acknowledgement".into()),
    );
    std::fs::set_permissions(&sessions, permissions)?;
    // @step Then inference stays blocked until the exact promoted identity reconciles without another generation
    let error = result.err().ok_or("lost acknowledgement")?;
    assert!(
        matches!(
            &error,
            codelet_core::persistence::compaction_commit::CompactionCommitError::Promoted { .. }
        ),
        "{error}"
    );
    reconcile_failure(&mut fixture, error)?;
    Ok(())
}

#[test]
#[serial_test::serial]
fn scenario_stale_durable_codex_sources_cannot_advance_lineage() -> TestResult {
    // @step Given a selected Codex source superseded by a successful compaction
    let mut fixture = fixture()?;
    let source = fixture
        .session
        .capture_durable_compaction(fixture.manifest.id)?;
    fixture
        .session
        .commit_selected_compaction(source.clone(), Uuid::new_v4(), DAG, |_| Ok(()))?;
    // @step When the runtime attempts to promote the old source again
    // @step Then original history stays unchanged without another promotion publication or inference
    assert!(assert_refused(&mut fixture, source)?.contains("stale source"));
    Ok(())
}
