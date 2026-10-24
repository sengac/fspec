use codelet_core::persistence::compaction_boundary::capture_compaction_source;
use codelet_core::persistence::compaction_commit::{
    commit_compaction, publish_compaction, reconcile_compaction, CompactionCommitError,
};
use codelet_core::persistence::*;
use codelet_test_helpers::setup_temp_directory;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[path = "support/compaction_boundary.rs"]
mod support;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Clone, Copy, Debug)]
enum Boundary {
    Capture,
    Promote,
    Reconcile,
    Publish,
    Replay,
}

fn snapshot(root: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>, std::io::Error> {
    let mut files = BTreeMap::new();
    for entry in std::fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            files.extend(snapshot(&path)?);
        } else {
            files.insert(path.clone(), std::fs::read(path)?);
        }
    }
    Ok(files)
}

fn append_blob_evidence(
    session: &mut SessionManifest,
    kind: usize,
) -> Result<Value, Box<dyn std::error::Error>> {
    let payloads = [
        "tool result 原文",
        r#"{"path":"original.txt","body":"exact input"}"#,
        "private reasoning",
        "aW1hZ2U=",
        "ZG9jdW1lbnQ=",
    ];
    let hashes = payloads
        .iter()
        .map(|payload| store_blob(payload.as_bytes()))
        .collect::<Result<Vec<_>, _>>()?;
    let marker = |index: usize| make_blob_reference(&hashes[index]);
    let blocks = json!([
        {"type":"tool_result","tool_use_id":"item:exact","call_id":"call-exact","content":marker(0)},
        {"type":"tool_use","id":"item:exact","call_id":"call-exact","name":"read","input":{"_blob_ref":marker(1)}},
        {"type":"thinking","thinking":marker(2),"signature":"exact-signature"},
        {"type":"image","source":{"type":"base64","media_type":"image/png","data":marker(3)}},
        {"type":"document","source":{"type":"base64","media_type":"application/pdf","data":marker(4)}}
    ]);
    let keys = [
        "tool_result:0:item:exact",
        "tool_use:0:item:exact",
        "thinking:0",
        "image:0",
        "document:0",
    ];
    let role = if matches!(kind, 1 | 2) {
        "assistant"
    } else {
        "user"
    };
    let envelope = json!({
        "uuid": uuid::Uuid::new_v4(), "provider":"openai", "opaque":{"exact":true},
        "message":{"role":role,"content":[blocks[kind]]},
        "_blobRefs":[{"key":keys[kind],"hash":hashes[kind]}]
    });
    append_message_with_metadata(
        session,
        role,
        "blob-backed original",
        serde_json::from_value(envelope.clone())?,
    )?;
    Ok(envelope)
}

fn damage_evidence(
    root: &Path,
    session: &SessionManifest,
    mut envelope: Value,
    damage: usize,
) -> TestResult {
    if damage < 10 {
        let hash = envelope["_blobRefs"][0]["hash"]
            .as_str()
            .ok_or("missing hash")?;
        let path = root.join("blobs").join(&hash[..2]).join(hash);
        if damage.is_multiple_of(2) {
            std::fs::remove_file(path)?;
        } else {
            std::fs::write(path, b"corrupt bytes")?;
        }
        return Ok(());
    }
    match damage {
        10 => envelope["_blobRefs"][0]["hash"] = json!(store_blob(b"different evidence")?),
        11 => envelope["_blobRefs"][0]["key"] = json!("tool_result:0:wrong-id"),
        12 => envelope["_blobRefs"] = json!({"malformed":"not an array"}),
        13 => {
            let duplicate = envelope["_blobRefs"][0].clone();
            envelope["_blobRefs"]
                .as_array_mut()
                .ok_or("missing references")?
                .push(duplicate);
        }
        14 => {
            let hash = store_blob(&[0xff])?;
            envelope["_blobRefs"][0]["hash"] = json!(hash);
            envelope["message"]["content"][0]["content"] = json!(make_blob_reference(&hash));
        }
        15 => {
            let hash = store_blob(b"not JSON")?;
            envelope["_blobRefs"][0]["hash"] = json!(hash);
            envelope["message"]["content"][0]["input"]["_blob_ref"] =
                json!(make_blob_reference(&hash));
        }
        _ => return Err("unknown damage case".into()),
    }
    let message_id = session.messages[0].message_id;
    update_message_metadata(message_id, serde_json::from_value(envelope)?)?;
    Ok(())
}

fn reject_damaged_evidence(boundary: Boundary) -> TestResult {
    for damage in 0..16 {
        let directory = setup_temp_directory()?;
        codelet_common::set_data_directory(directory.path().to_path_buf())?;
        reset_stores_for_tests();
        let mut session =
            create_session_with_provider("blob evidence", directory.path(), "openai")?;
        let kind = if damage < 10 {
            damage / 2
        } else {
            usize::from(damage == 15)
        };
        let envelope = append_blob_evidence(&mut session, kind)?;
        let mut candidate = Some(support::candidate(&session)?);
        if matches!(
            boundary,
            Boundary::Reconcile | Boundary::Publish | Boundary::Replay
        ) {
            session = commit_compaction(
                candidate.take().ok_or("missing candidate")?,
                "model-a",
                |_, _| Ok(()),
            )?;
            if matches!(boundary, Boundary::Replay) {
                let identity = &session
                    .compaction_commit
                    .as_ref()
                    .ok_or("missing commit")?
                    .identity;
                session = reconcile_compaction(identity)?;
            }
        }
        damage_evidence(directory.path(), &session, envelope, damage)?;
        let before = snapshot(directory.path())?;
        reset_stores_for_tests();
        let result = match boundary {
            Boundary::Capture => {
                capture_compaction_source(&session, "model-a", "dag-v1").map(|_| ())
            }
            Boundary::Promote => commit_compaction(
                candidate.take().ok_or("missing candidate")?,
                "model-a",
                |_, _| Ok(()),
            )
            .map(|_| ()),
            Boundary::Reconcile => reconcile_compaction(
                &session
                    .compaction_commit
                    .as_ref()
                    .ok_or("missing commit")?
                    .identity,
            )
            .map(|_| ()),
            Boundary::Publish => {
                publish_compaction(&session, |_| Err("publication must not run".into())).map(|_| ())
            }
            Boundary::Replay => get_session_message_envelopes(session.id)
                .map(|_| ())
                .map_err(CompactionCommitError::Rejected),
        };
        assert!(
            matches!(result, Err(CompactionCommitError::Rejected(_))),
            "{boundary:?} accepted damage {damage}: {result:?}"
        );
        assert_eq!(
            snapshot(directory.path())?,
            before,
            "{boundary:?} mutated bytes for damage {damage}"
        );
    }
    Ok(())
}

#[test]
#[serial_test::serial]
fn source_capture_rejects_unavailable_blob_evidence() -> TestResult {
    reject_damaged_evidence(Boundary::Capture)
}

#[test]
#[serial_test::serial]
fn promotion_rejects_unavailable_blob_evidence() -> TestResult {
    reject_damaged_evidence(Boundary::Promote)
}

#[test]
#[serial_test::serial]
fn reconciliation_rejects_unavailable_blob_evidence() -> TestResult {
    reject_damaged_evidence(Boundary::Reconcile)
}

#[test]
#[serial_test::serial]
fn publication_rejects_unavailable_blob_evidence() -> TestResult {
    reject_damaged_evidence(Boundary::Publish)
}

#[test]
#[serial_test::serial]
fn cold_replay_rejects_unavailable_covered_blob_evidence() -> TestResult {
    reject_damaged_evidence(Boundary::Replay)
}

#[test]
#[serial_test::serial]
fn valid_blob_evidence_preserves_original_bytes_and_identities() -> TestResult {
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    let mut session = create_session_with_provider("valid blobs", directory.path(), "openai")?;
    append_blob_evidence(&mut session, 0)?;
    let before_capture = snapshot(directory.path())?;
    let candidate = support::candidate(&session)?;
    assert_eq!(snapshot(directory.path())?, before_capture);
    let mut envelopes = Vec::new();
    for kind in 0..5 {
        envelopes.push(append_blob_evidence(&mut session, kind)?);
    }
    append_message(&mut session, "user", "literal blob:sha256:not-evidence")?;
    let originals = snapshot(&directory.path().join("messages"))?;
    let blobs = snapshot(&directory.path().join("blobs"))?;
    let committed = commit_compaction(candidate, "model-a", |_, _| Ok(()))?;
    let ready = publish_compaction(&committed, |_| Ok(()))?;
    assert!(
        !ready
            .compaction_commit
            .as_ref()
            .ok_or("missing commit")?
            .publication_pending
    );
    reset_stores_for_tests();
    let before_replay = snapshot(directory.path())?;
    let replay = get_session_message_envelopes(session.id)?;
    let paths = [
        "/content",
        "/input",
        "/thinking",
        "/source/data",
        "/source/data",
    ];
    let values = [
        json!("tool result 原文"),
        json!({"path":"original.txt","body":"exact input"}),
        json!("private reasoning"),
        json!("aW1hZ2U="),
        json!("ZG9jdW1lbnQ="),
    ];
    for (index, mut expected) in envelopes.into_iter().enumerate() {
        let suffix: Value = serde_json::from_str(&replay[index + 1])?;
        *expected["message"]["content"][0]
            .pointer_mut(paths[index])
            .ok_or("missing blob target")? = values[index].clone();
        assert_eq!(suffix["message"], expected["message"]);
        assert_eq!(suffix["_blobRefs"], expected["_blobRefs"]);
    }
    assert_eq!(snapshot(directory.path())?, before_replay);
    assert_eq!(snapshot(&directory.path().join("messages"))?, originals);
    assert_eq!(snapshot(&directory.path().join("blobs"))?, blobs);
    Ok(())
}
