//! Feature: spec/features/dag-publication-ownership.feature

use codelet_core::persistence::compaction_commit::*;
use codelet_core::persistence::*;
use codelet_test_helpers::setup_temp_directory;
use std::time::{Duration, Instant};

#[path = "support/compaction_boundary.rs"]
mod support;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
#[serial_test::serial]
fn scenario_publication_callbacks_cannot_reenter_session_ownership() -> TestResult {
    if std::env::var_os("CMPCT055_REENTRY_CHILD").is_none() {
        let mut child = std::process::Command::new(std::env::current_exe()?)
            .args([
                "--exact",
                "scenario_publication_callbacks_cannot_reenter_session_ownership",
                "--nocapture",
            ])
            .env("CMPCT055_REENTRY_CHILD", "1")
            .spawn()?;
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if let Some(status) = child.try_wait()? {
                assert!(status.success(), "ownership child failed: {status}");
                return Ok(());
            }
            if Instant::now() >= deadline {
                child.kill()?;
                child.wait()?;
                return Err("publication callback blocked on nested ownership".into());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    // @step Given a promoted generation awaiting publication
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    let mut manifest =
        create_session_with_provider("publication ownership", directory.path(), "openai")?;
    append_message(&mut manifest, "user", "immutable source")?;
    let mut independent = SessionStore::new()?;
    let committed = commit_compaction(support::candidate(&manifest)?, "model-a", |_, _| Ok(()))?;
    let path = directory
        .path()
        .join(format!("sessions/{}.json", manifest.id));
    let before = std::fs::read(&path)?;
    let identity = committed
        .compaction_commit
        .as_ref()
        .ok_or("boundary")?
        .identity
        .clone();
    // @step When its callback attempts nested manifest reads or writes
    let result = publish_compaction(&committed, |_| {
        // @step Then nested ownership is rejected explicitly without waiting
        assert!(load_session(manifest.id).is_err_and(|reason| reason.contains("ownership")));
        assert!(save_session(&committed).is_err_and(|reason| reason.contains("ownership")));
        assert!(delete_session(manifest.id).is_err_and(|reason| reason.contains("ownership")));
        assert!(matches!(
            update_session_manifest(manifest.id, |_| Ok(())),
            Err(ManifestCommitError::Ownership { .. })
        ));
        assert!(matches!(
            independent.update(manifest.id, |_| Ok(())),
            Err(ManifestCommitError::Ownership { .. })
        ));
        assert!(matches!(
            independent.commit(&committed),
            Err(ManifestCommitError::Ownership { .. })
        ));
        assert!(independent
            .delete(manifest.id)
            .is_err_and(|reason| reason.contains("ownership")));
        Err("callback rejected nested ownership".into())
    });
    assert!(
        matches!(result, Err(CompactionCommitError::Publication { identity: actual, .. }) if actual == identity)
    );
    assert_eq!(std::fs::read(&path)?, before);
    // @step And the original pending identity remains available for reconciliation
    reset_stores_for_tests();
    let restored = load_session(manifest.id)?;
    let boundary = restored
        .compaction_commit
        .ok_or("missing pending boundary")?;
    assert_eq!(boundary.identity, identity);
    assert!(boundary.publication_pending);
    let reconciled = reconcile_compaction(&identity)?;
    assert!(
        !reconciled
            .compaction_commit
            .ok_or("boundary")?
            .publication_pending
    );
    let ready = load_session(manifest.id)?;
    commit_compaction(support::candidate(&ready)?, "model-a", |_, _| {
        assert!(load_session(manifest.id).is_err_and(|reason| reason.contains("ownership")));
        Ok(())
    })?;
    Ok(())
}

#[cfg(unix)]
#[test]
#[serial_test::serial]
fn scenario_post_promotion_durability_failure_retains_the_compaction_identity() -> TestResult {
    use std::os::unix::fs::PermissionsExt;

    // @step Given an admitted candidate with an exact operation and generation
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    let mut manifest =
        create_session_with_provider("lost directory acknowledgement", directory.path(), "openai")?;
    append_message(&mut manifest, "user", "immutable source")?;
    let candidate = support::candidate(&manifest)?;
    let operation = candidate.operation_id;
    let sessions = directory.path().join("sessions");
    let permissions = std::fs::metadata(&sessions)?.permissions();
    // @step When directory acknowledgement fails after atomic promotion
    std::fs::set_permissions(&sessions, std::fs::Permissions::from_mode(0o300))?;
    let result = commit_compaction(candidate, "model-a", |_, _| Ok(()));
    std::fs::set_permissions(&sessions, permissions)?;
    // @step Then the result distinguishes the promoted identity from a pre-promotion failure
    let error = result.err().ok_or("directory read denial must fail")?;
    assert!(
        !matches!(&error, CompactionCommitError::Persistence(_)),
        "a promoted boundary must not look like an ordinary persistence failure: {error}"
    );
    assert!(error.to_string().contains(&operation.to_string()));
    let promoted = error
        .promoted_identity()
        .ok_or("missing promoted identity")?
        .clone();
    let path = sessions.join(format!("{}.json", manifest.id));
    let before = std::fs::read(&path)?;
    // @step And a cold reader retains that pending generation without another commit
    reset_stores_for_tests();
    let restored = load_session(manifest.id)?;
    let boundary = restored
        .compaction_commit
        .as_ref()
        .ok_or("promoted boundary")?;
    assert_eq!(boundary.identity.operation_id, operation);
    assert_eq!(boundary.identity.generation, 1);
    assert_eq!(boundary.identity.revision, manifest.revision + 1);
    assert_eq!(boundary.identity, promoted);
    assert!(boundary.publication_pending);
    assert_eq!(restored.revision, manifest.revision + 1);
    assert_eq!(std::fs::read(path)?, before);
    for publish in [false, true] {
        let pending = if publish {
            let ready = load_session(manifest.id)?;
            commit_compaction(support::candidate(&ready)?, "model-a", |_, _| Ok(()))?
        } else {
            restored.clone()
        };
        let identity = pending
            .compaction_commit
            .as_ref()
            .ok_or("boundary")?
            .identity
            .clone();
        let permissions = std::fs::metadata(&sessions)?.permissions();
        std::fs::set_permissions(&sessions, std::fs::Permissions::from_mode(0o300))?;
        let result = if publish {
            publish_compaction(&pending, |_| Ok(()))
        } else {
            reconcile_compaction(&identity)
        };
        std::fs::set_permissions(&sessions, permissions)?;
        let error = result.err().ok_or("acknowledgement must fail")?;
        assert!(matches!(error, CompactionCommitError::Promoted { .. }));
        assert_eq!(error.promoted_identity(), Some(&identity));
        reset_stores_for_tests();
        let acknowledged = load_session(manifest.id)?;
        let boundary = acknowledged.compaction_commit.ok_or("boundary")?;
        assert_eq!(boundary.identity, identity);
        assert!(!boundary.publication_pending);
        assert_eq!(acknowledged.revision, pending.revision + 1);
    }
    Ok(())
}
