use super::fixture::{Harness, TestResult};
use super::receipt_support as support;
use codelet_core::persistence::input_receipts::{
    self, InputAuthorityError, InputDisposition, InputOrigin, InputReceipt, InputReceiptError,
};
use codelet_core::persistence::messages::durability::observe;
use codelet_core::persistence::*;
use serde_json::json;
use std::io::Write;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_process_death_after_queue_acknowledgment_retains_the_pending_submission(
) -> TestResult {
    if let Some(mailbox) = std::env::var_os("BUG284_PENDING_CHILD") {
        let harness = Harness::new()?;
        harness
            .session
            .send_input("acknowledged pending original".into(), None)?;
        let bytes = serde_json::to_vec(
            &json!({"root":harness.directory.path(),"session":harness.session.id}),
        )?;
        let mut receipt = std::fs::File::create(mailbox)?;
        receipt.write_all(&bytes)?;
        receipt.sync_all()?;
        loop {
            std::thread::park();
        }
    }
    // @step Given a pending submission was acknowledged before the task loop dequeued it
    let directory = codelet_test_helpers::setup_temp_directory()?;
    let mailbox = directory.path().join("pending-child.json");
    let name = "pending_crash::scenario_process_death_after_queue_acknowledgment_retains_the_pending_submission";
    let mut child = support::spawn(name, "BUG284_PENDING_CHILD", &mailbox)?;
    Ok::<_, Box<dyn std::error::Error>>(support::mailbox(&mut child, &mailbox).await?).and_then(
        |identity| {
            // @step When the submitting process is killed and persisted state is reopened
            child.0.kill()?;
            let status = child.0.wait()?;
            assert!(!status.success());
            let root = std::path::PathBuf::from(identity["root"].as_str().ok_or("child root")?);
            codelet_test_helpers::isolated_user_directory()?;
            codelet_common::set_data_directory(root.clone())?;
            reset_stores_for_tests();
            let id = serde_json::from_value(identity["session"].clone())?;
            let originals = input_receipts::list(id)?;
            let manifest = load_session(id)?;
            let replay = crate::replay_content::Replay::parse(&get_session_message_envelopes(id)?)?;
            reset_stores_for_tests();
            std::fs::remove_dir_all(root)?;
            // @step Then the pending original remains recoverable exactly once
            let pending: Vec<_> = originals
                .iter()
                .filter(|original| original.text() == "acknowledged pending original")
                .collect();
            assert_eq!(
                pending.len(),
                1,
                "acknowledged input was lost when its process died before dequeue"
            );
            assert_eq!(pending[0].disposition, InputDisposition::Pending);
            // @step And it is absent from conversational replay and cannot execute automatically
            assert!(replay.messages.is_empty());
            assert!(manifest.messages.is_empty());
            assert!(pending[0].execution.is_none());
            Ok(())
        },
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_submission_identity_survives_retries_images_and_identical_prompt_text(
) -> TestResult {
    // @step Given two intentional submissions have identical text and image payloads
    let harness = Harness::new()?;
    let session = harness.session.id;
    codelet_agent_loop::persist::persist_user_message(&session, "genuine task")?;
    for _ in 0..2 {
        let input = codelet_sessions::background_session::IncomingMessage::with_images(
            "synthetic-controller".into(),
            "operator".into(),
            "identical payload".into(),
            support::image(),
        )?;
        harness.session.receive_incoming_message(input)?;
    }
    let receipts = input_receipts::list(session)?;
    assert_eq!(receipts.len(), 2);
    assert_ne!(receipts[0].id, receipts[1].id);
    assert_eq!(receipts[0].content, receipts[1].content);
    assert!(serde_json::to_string(&receipts[0].content)?.contains("iVBORw0KGgo"));
    // @step When one receipt is retried and a conflicting identity is attempted
    input_receipts::retain(&receipts[0])?;
    let mut conflicting = receipts[0].clone();
    conflicting.content = vec![UserContent::Text {
        text: "altered under a reused identity".into(),
    }];
    let conflict = input_receipts::retain(&conflicting);
    // @step Then the two distinct originals remain recoverable and the retry adds no duplicate
    assert!(matches!(conflict, Err(InputReceiptError::Conflict(id)) if id == receipts[0].id));
    assert_eq!(input_receipts::list(session)?, receipts);
    // @step And the conflicting identity or unknown receipt version blocks inference
    let unknown = harness.directory.path().join(format!(
        "input-receipts/{session}/{}.json",
        uuid::Uuid::new_v4()
    ));
    std::fs::write(&unknown, r#"{"version":99}"#)?;
    let refused = input_receipts::inference_authority(&load_session(session)?);
    assert!(matches!(
        refused,
        Err(InputAuthorityError::Receipt(
            InputReceiptError::UnknownVersion { version: 99, .. }
        ))
    ));
    std::fs::remove_file(&unknown)?;
    input_receipts::inference_authority(&load_session(session)?)?;
    // @step And garbage collection preserves pending receipt and recovery spool blob evidence
    codelet_agent_loop::persist::persist_input_original(
        &session,
        receipts[0].id,
        receipts[0].content.clone(),
    )?;
    let orphan = uuid::Uuid::new_v4();
    conversation_commit::store_original_idempotent(orphan, "user", "orphan", Default::default())?;
    reset_stores_for_tests();
    assert!(cleanup_orphaned_messages()? >= 1);
    assert!(
        get_message(orphan)?.is_none(),
        "garbage collection did not run"
    );
    let retained = get_message(receipts[0].id)?.ok_or("receipt original collected")?;
    let evidence = serde_json::to_string(&retained.metadata)?;
    assert!(evidence.contains("iVBORw0KGgo") || retained.metadata.contains_key("_blobRefs"));
    assert_eq!(input_receipts::list(session)?, receipts);
    // @step And fork or merge cannot grant execution rights from pending or canceled receipts
    input_receipts::settle(session, receipts[1].id, InputDisposition::Cancelled)?;
    let plain = create_session("BUG-284 plain", harness.directory.path())?;
    codelet_agent_loop::persist::persist_user_message(&plain.id, "plain task")?;
    let mut held = Vec::new();
    for disposition in [InputDisposition::Pending, InputDisposition::Cancelled] {
        let content = receipts[0].content.clone();
        let receipt = InputReceipt::pending(plain.id, InputOrigin::User, None, content)?;
        input_receipts::retain(&receipt)?;
        input_receipts::settle(plain.id, receipt.id, disposition)?;
        held.push(receipt);
    }
    let source = load_session(plain.id)?;
    let mut fork = fork_session(&source, source.messages.len() - 1, "BUG-284 fork")?;
    merge_messages(&mut fork, session, &[0])?;
    assert!(input_receipts::list(fork.id)?.is_empty());
    for receipt in receipts.iter().chain(&held) {
        assert!(!fork.messages.iter().any(|m| m.message_id == receipt.id));
        assert!(input_receipts::admit(fork.id, receipt.id).is_err());
    }
    let cancelled = input_receipts::admit(session, receipts[1].id);
    assert!(matches!(
        cancelled,
        Err(InputAuthorityError::NotAdmissible(..))
    ));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_durability_acknowledgment_loss_never_grants_unresolved_execution_authority(
) -> TestResult {
    // @step Given an input is being promoted at a manifest durability boundary
    let harness = Harness::new()?;
    let session = harness.session.id;
    let mut ids = Vec::new();
    for text in ["promoted original", "unpromoted original"] {
        harness.session.send_input(text.into(), None)?;
        let receipt = input_receipts::list(session)?
            .into_iter()
            .find(|receipt| receipt.text() == text)
            .ok_or("receipt")?;
        codelet_agent_loop::persist::persist_input_original(&session, receipt.id, receipt.content)?;
        ids.push(receipt.id);
    }
    let (promoted, unpromoted) = (ids[0], ids[1]);
    // @step When durability acknowledgment is lost after rename or storage remains unavailable
    let lost = std::thread::spawn(move || {
        observe(Some("manifest.directory"), || {
            input_receipts::admit(session, promoted)
        })
    })
    .join()
    .map_err(|_| "admission worker panicked")?;
    assert!(lost.1.contains(&"manifest.rename"));
    assert!(matches!(lost.0, Err(InputAuthorityError::Unresolved { id, .. }) if id == promoted));
    let unavailable = std::thread::spawn(move || {
        observe(Some("manifest.rename"), || {
            input_receipts::admit(session, unpromoted)
        })
    })
    .join()
    .map_err(|_| "admission worker panicked")?;
    assert!(
        matches!(unavailable.0, Err(InputAuthorityError::Commit { id, .. }) if id == unpromoted)
    );
    // @step Then the same receipt identity is settled or inference remains blocked
    assert!(matches!(
        input_receipts::inference_authority(&load_session(session)?),
        Err(InputAuthorityError::Unresolved { id, .. }) if id == promoted
    ));
    input_receipts::settle_admission(session, promoted)?;
    let settled = input_receipts::get(session, promoted)?.ok_or("receipt")?;
    assert_eq!(settled.disposition, InputDisposition::Admitted);
    assert_eq!(support::memberships(session, promoted)?, 1);
    input_receipts::inference_authority(&load_session(session)?)?;
    // @step And the original evidence is preserved without requeueing it under a new identity
    let pending = input_receipts::get(session, unpromoted)?.ok_or("receipt")?;
    assert_eq!(pending.disposition, InputDisposition::Pending);
    assert_eq!(support::memberships(session, unpromoted)?, 0);
    assert!(get_message(unpromoted)?.is_some());
    assert_eq!(input_receipts::list(session)?.len(), 2);
    // @step And a local attempt retry cannot be mistaken for a new externally identified submission
    input_receipts::admit(session, unpromoted)?;
    input_receipts::admit(session, promoted)?;
    assert_eq!(support::memberships(session, unpromoted)?, 1);
    assert_eq!(support::memberships(session, promoted)?, 1);
    let receipts = input_receipts::list(session)?;
    assert_eq!(receipts.iter().map(|r| r.id).collect::<Vec<_>>(), ids);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_process_death_preserves_input_disposition_without_replaying_effects() -> TestResult
{
    if let Some(mailbox) = std::env::var_os("BUG284_DISPOSITION_CHILD") {
        return support::dispositions_child(std::path::Path::new(&mailbox));
    }
    // @step Given an input is pending admitted installed or beginning provider dispatch
    let directory = codelet_test_helpers::setup_temp_directory()?;
    let path = directory.path().join("disposition-child.json");
    let name = "pending_crash::scenario_process_death_preserves_input_disposition_without_replaying_effects";
    let mut child = support::spawn(name, "BUG284_DISPOSITION_CHILD", &path)?;
    let identity = support::mailbox(&mut child, &path).await?;
    // @step When the process dies and persisted state is reopened
    child.0.kill()?;
    assert!(!child.0.wait()?.success());
    let root = std::path::PathBuf::from(identity["root"].as_str().ok_or("root")?);
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(root.clone())?;
    reset_stores_for_tests();
    let session: uuid::Uuid = serde_json::from_value(identity["session"].clone())?;
    let id = |key: &str| serde_json::from_value::<uuid::Uuid>(identity["ids"][key].clone());
    let replay = |session| -> Result<String, Box<dyn std::error::Error>> {
        let envelopes = get_session_message_envelopes(session)?;
        Ok(serde_json::to_string(
            &crate::replay_content::Replay::parse(&envelopes)?.messages,
        )?)
    };
    // @step Then only admitted inputs restore into the conversation exactly once
    let restored = replay(session)?;
    for (text, members) in [
        ("pending", 0),
        ("canceled", 0),
        ("admitted", 1),
        ("dispatching", 1),
    ] {
        assert_eq!(
            restored.matches(&format!("{text} original")).count(),
            members
        );
        assert_eq!(support::memberships(session, id(text)?)?, members);
    }
    // @step And pending and canceled inputs remain recoverable without automatic execution
    let pending = input_receipts::get(session, id("pending")?)?.ok_or("pending")?;
    let canceled = input_receipts::get(session, id("canceled")?)?.ok_or("canceled")?;
    assert_eq!(pending.disposition, InputDisposition::Pending);
    assert_eq!(canceled.disposition, InputDisposition::Cancelled);
    assert!(pending.execution.is_none() && canceled.execution.is_none());
    // @step And uncertain dispatch blocks continuation until explicit recovery records a truthful model visible disposition
    let blocked = input_receipts::inference_authority(&load_session(session)?);
    let dispatching = id("dispatching")?;
    assert!(
        matches!(blocked, Err(InputAuthorityError::UncertainDispatch(found)) if found == dispatching)
    );
    let text =
        codelet_agent_loop::input_admission::recover_uncertain_dispatch(session, dispatching)?;
    input_receipts::inference_authority(&load_session(session)?)?;
    let recovered = replay(session)?;
    assert_eq!(recovered.matches(&text).count(), 1);
    assert_eq!(recovered.matches("dispatching original").count(), 1);
    reset_stores_for_tests();
    std::fs::remove_dir_all(root)?;
    Ok(())
}
