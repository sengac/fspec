//! Feature: spec/features/provider-checkpoint-restore-preflight.feature

use codelet_providers::session::ProviderSession;
use serde_json::json;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

#[path = "cmpct078/support.rs"]
pub mod support;
use support::{history, request};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn invalid_retained_checkpoint_preflight_leaves_live_authority_usable() -> TestResult {
    // @step Given a usable installed Codex checkpoint with a persistence observer
    let session = ProviderSession::for_provider("codex");
    history(&session)?.commit(&request("authoritative DAG"), &[])?;
    let original = history(&session)?.snapshot()?;
    let writes = Arc::new(AtomicUsize::new(0));
    let observed = writes.clone();
    session.bind_writers(|_| {
        let observed = observed.clone();
        Arc::new(move |_| {
            observed.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    })?;
    // @step When retained replay is preflighted with malformed or incompatible checkpoint metadata
    for failure in ["malformed", "version", "protocol", "null", "model"] {
        let mut checkpoint = serde_json::to_value(&original)?;
        match failure {
            "version" => checkpoint["version"] = json!(999),
            "protocol" => checkpoint["protocol"] = json!("removed-delta"),
            "null" => checkpoint = serde_json::Value::Null,
            "model" => checkpoint["model"] = json!("different-model"),
            _ => {}
        }
        let envelope = if failure == "malformed" {
            "{invalid".into()
        } else {
            json!({"codexContinuation":checkpoint}).to_string()
        };
        let result = session.validate_restore(&[envelope]);
        // @step Then each invalid replay returns an error without changing the checkpoint or invoking persistence
        assert!(result.is_err(), "{failure} accepted");
        assert_eq!(history(&session)?.snapshot()?, original);
        assert_eq!(writes.load(Ordering::SeqCst), 0);
        // @step And the original request remains preparable and an authentic retry validates
        history(&session)?.prepare(&mut request("authoritative DAG"))?;
    }
    session.validate_restore(&[json!({"codexContinuation":original}).to_string()])?;
    assert_eq!(history(&session)?.snapshot()?, original);
    Ok(())
}

#[test]
fn prospective_committed_boundary_preflight_does_not_install_the_boundary() -> TestResult {
    // @step Given an existing Codex checkpoint and a newer committed boundary
    let session = ProviderSession::for_provider("codex");
    history(&session)?.commit(&request("original DAG"), &[])?;
    let original = history(&session)?.snapshot()?;
    let metadata = session.prepare_generation(1, "shared-model")?;
    let valid = support::boundary()?.to_string();
    // @step When full retained replay is validated against the prospective boundary
    session.validate_committed_restore(1, &metadata, std::slice::from_ref(&valid))?;
    // @step Then valid replay succeeds and incompatible retained identity fails
    for failure in [
        "model",
        "future",
        "version",
        "protocol",
        "null",
        "malformed",
    ] {
        let mut incompatible = json!({"codexContinuation":metadata["codexContinuation"]});
        match failure {
            "model" => incompatible["codexContinuation"]["model"] = json!("wrong-model"),
            "future" => incompatible["codexContinuation"]["generation"] = json!(2),
            "version" => incompatible["codexContinuation"]["version"] = json!(999),
            "protocol" => incompatible["codexContinuation"]["protocol"] = json!("removed-delta"),
            "null" => incompatible["codexContinuation"] = serde_json::Value::Null,
            _ => {}
        }
        let envelope = if failure == "malformed" {
            "{invalid".into()
        } else {
            incompatible.to_string()
        };
        assert!(
            session
                .validate_committed_restore(1, &metadata, &[valid.clone(), envelope])
                .is_err(),
            "{failure} accepted"
        );
        // @step And the original checkpoint and request authority remain unchanged
        assert_eq!(history(&session)?.snapshot()?, original);
        history(&session)?.prepare(&mut request("original DAG"))?;
    }
    Ok(())
}
