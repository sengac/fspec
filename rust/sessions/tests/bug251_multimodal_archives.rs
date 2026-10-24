//! Feature: spec/features/faithful-multimodal-archives.feature

#[path = "bug251/support.rs"]
mod support;
#[path = "bug251/transport.rs"]
mod transport;

use codelet_core::{persistence::*, session_manager_handle::SessionManagerHandle};
use codelet_rpc_types::{SessionId, StreamChunk};
use codelet_sessions::SessionManager;
use serde_json::json;
use std::sync::Arc;
use support::{archive, images, media_blocks, originals, replay::*, TestResult};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn accepted_images_survive_originals_and_cold_next_request() -> TestResult {
    if let Some(id) = cold_process_session()? {
        codelet_providers::stub_provider::register_stub_provider();
        let manager = cold_resume(&id)?;
        let chunks = manager.get_session(&id.value)?.get_buffered_output(1000);
        let attachments = chunks
            .iter()
            .filter_map(|chunk| match chunk {
                StreamChunk::IncomingMessage { images, .. } => images.as_ref(),
                _ => None,
            })
            .flatten()
            .collect::<Vec<_>>();
        assert_eq!(
            attachments.len(),
            2,
            "restored transcript must identify both original images"
        );
        for (attachment, block) in attachments
            .iter()
            .zip(images()?.as_array().ok_or("images")?.iter().skip(1))
        {
            assert!(
                attachment.data == block["source"]["data"].as_str().ok_or("data")?,
                "display lost original image data"
            );
            assert!(
                attachment.media_type == block["source"]["media_type"].as_str().ok_or("type")?,
                "display lost image metadata"
            );
        }
        let wire = next_anthropic_wire(&manager, &id, 0).await?;
        assert!(
            media_blocks(&wire) == images()?.as_array().ok_or("image array")?[1..],
            "cold physical request lost image content or ordering"
        );
        return Ok(());
    }
    // @step Given a real session accepting Unicode text and two ordered images
    let (directory, _, _, _) = manager().await?;
    codelet_providers::stub_provider::register_stub_provider();
    let manager = Arc::new(SessionManager::new());
    manager.set_hooks(Arc::new(codelet_agent_loop::FspecAgentHooks::new()));
    for websocket in [false, true] {
        let id = SessionId::new(
            manager
                .create_session("stub/canned", &directory.path().to_string_lossy())
                .await?,
        );
        let session = manager.get_session(&id.value)?;
        let mut chunks = manager.chunks_tx().subscribe();
        let content = images()?;
        let attachments = content.as_array().ok_or("image array")?[1..]
            .iter()
            .map(|block| {
                Ok(codelet_rpc_types::IncomingMessageImage {
                    data: block["source"]["data"].as_str().ok_or("image data")?.into(),
                    media_type: block["source"]["media_type"]
                        .as_str()
                        .ok_or("image type")?
                        .into(),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        // @step When the input is sent through the real agent loop
        transport::send(
            directory.path(),
            manager.clone(),
            id.clone(),
            websocket,
            attachments,
        )
        .await?;
        tokio::time::timeout(std::time::Duration::from_secs(20), async {
            loop {
                let (_, chunk) = chunks.recv().await?;
                if matches!(chunk, StreamChunk::Done) {
                    break;
                }
            }
            Ok::<_, Box<dyn std::error::Error>>(())
        })
        .await??;
        // @step Then bounded originals preserve the original text image order types and bytes
        let page = originals(&id)?;
        let stored = page.records[0].envelope["message"]["content"]
            .as_array()
            .ok_or("stored content")?;
        assert!(stored[0]["text"]
            .as_str()
            .ok_or("text")?
            .ends_with("原文 α"));
        assert!(
            stored[1..] == content.as_array().ok_or("images")?[1..],
            "accepted input lost original image content or ordering"
        );
        assert!(page.records[0].envelope["_blobRefs"]
            .as_array()
            .is_some_and(|refs| !refs.is_empty()));
        assert!(matches!(
            originals::retrieve_originals(
                codelet_tools::session_search::originals::OriginalsRequest {
                    session_id: id.value.clone(),
                    start_turn: Some(0),
                    end_turn: Some(0),
                    next_turn: None,
                    limit: Some(1),
                    max_bytes: Some(1024),
                }
            ),
            Err(codelet_tools::session_search::originals::OriginalsError::OversizedEnvelope { .. })
        ));
        // @step And a fresh process sends those images on the next physical provider request
        drop(session);
        run_cold_process(
            directory.path(),
            &id,
            "accepted_images_survive_originals_and_cold_next_request",
        )
        .await?;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn stored_documents_and_remote_media_retain_supported_source_identities() -> TestResult {
    // @step Given ordered stored PDF documents and remote image sources
    let (_directory, manager, id, baseline) = manager().await?;
    let content = json!([
        {"type":"document","source":{"type":"base64","media_type":"application/pdf","data":"JVBERi0xLjQ="},"title":"original title","context":"original context","cache_control":{"type":"ephemeral"}},
        {"type":"image","source":{"type":"url","url":"https://example.invalid/changing.png"}},
        {"type":"document","source":{"type":"url","url":"https://example.invalid/paper.pdf"}}
    ]);
    // @step When the structured user content is archived and cold restored
    archive(&id, content.clone())?;
    drop(manager);
    let manager = cold_resume(&id)?;
    // @step Then the next physical provider request preserves supported media source identities
    let wire = next_wire(&manager, &id, baseline).await?;
    let encoded = wire.to_string();
    for identity in [
        "data:application/pdf;base64,JVBERi0xLjQ=",
        "https://example.invalid/changing.png",
        "https://example.invalid/paper.pdf",
    ] {
        assert!(
            encoded.contains(identity),
            "lost original media source identity"
        );
    }
    // @step And originals retain document metadata without claiming captured remote bytes
    assert!(
        originals(&id)?.records[0].envelope["message"]["content"] == content,
        "original document metadata changed"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn invalid_or_unsupported_restored_media_fails_visibly() -> TestResult {
    // @step Given stored media with unsupported formats or invalid source metadata
    let (_directory, manager, id, _) = manager().await?;
    for block in [
        json!({"type":"image","source":{"type":"base64","media_type":"image/mystery","data":"YQ=="}}),
        json!({"type":"document","source":{"type":"base64","media_type":"application/x-mystery","data":"YQ=="}}),
        json!({"type":"image","source":{"type":"base64","media_type":"image/png","data":"not base64!"}}),
        json!({"type":"image","source":{"type":"url","url":"file:///private/local.png"}}),
        json!({"type":"document","source":{"type":"url","url":""}}),
        support::oversized_image()?,
    ] {
        // @step When cold restoration decodes the media
        let result =
            manager.restore_session_messages(&id, vec![envelope(json!([block])).to_string()]);
        // @step Then restoration returns a compatibility error instead of text substitution
        assert!(result.is_err(), "invalid media silently restored");
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn referenced_media_survives_cleanup_and_missing_blobs_fail_visibly() -> TestResult {
    // @step Given archived media using the existing content addressed blob store
    let (_directory, manager, id, _) = manager().await?;
    archive(&id, images()?)?;
    let blob_store = BlobStore::new()?;
    let garbage = blob_store.store(b"unreferenced garbage")?;
    // @step When unrelated blobs are cleaned and the session is reopened
    cleanup_orphaned_messages()?;
    blob_store.delete(&garbage)?;
    drop(manager);
    let _manager = cold_resume(&id)?;
    // @step Then referenced media bytes remain available in bounded originals
    let page = originals(&id)?;
    assert!(
        page.records[0].envelope["message"]["content"] == images()?,
        "cleanup lost original image bytes"
    );
    let manifest = load_session(uuid::Uuid::parse_str(&id.value)?)?;
    let stored = get_session_messages_full(&manifest)?;
    let hash = stored[0].metadata["_blobRefs"][0]["hash"]
        .as_str()
        .ok_or("blob hash")?;
    // @step And removing a referenced blob returns a typed missing source error
    blob_store.delete(hash)?;
    let error = originals(&id).err().ok_or("missing blob must fail")?;
    assert!(matches!(
        error.downcast_ref::<codelet_tools::session_search::originals::OriginalsError>(),
        Some(codelet_tools::session_search::originals::OriginalsError::InvalidBlob { .. })
    ));
    assert!(cold_resume(&id).is_err());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn legacy_unicode_text_remains_compatible() -> TestResult {
    // @step Given a legacy text only conversation containing Unicode
    let (_directory, manager, id, baseline) = manager().await?;
    let mut manifest = load_session(uuid::Uuid::parse_str(&id.value)?)?;
    append_message_with_metadata(
        &mut manifest,
        "user",
        "legacy",
        [(
            "message".into(),
            json!({"role":"user","content":"原文 λ 🦀"}),
        )]
        .into_iter()
        .collect(),
    )?;
    // @step When the session is cold restored
    codelet_agent_loop::user_persistence::persist_user_input(
        &manifest.id,
        "current text-only λ",
        &[],
    )?;
    drop(manager);
    let manager = cold_resume(&id)?;
    // @step Then the next physical provider request retains the original text
    let wire = next_anthropic_wire(&manager, &id, baseline)
        .await?
        .to_string();
    assert!(wire.contains("原文 λ 🦀"));
    assert!(wire.contains("current text-only λ"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn tool_image_budgets_do_not_discard_original_attachments() -> TestResult {
    // @step Given an archived input with two images and a tool image budget of one
    let (_directory, manager, id, _) = manager().await?;
    archive(&id, images()?)?;
    let uuid = uuid::Uuid::parse_str(&id.value)?;
    manager.save_provider_settings(
        "anthropic",
        &codelet_rpc_types::ProfileDefinition {
            max_images: Some(1),
            ..Default::default()
        },
    )?;
    // @step When the session resumes and the tool image budget is queried
    drop(manager);
    let _manager = cold_resume(&id)?;
    // @step Then the tool image budget remains one and originals still retain both images
    let budget = codelet_tools::read_image_budget::effective_budget(uuid);
    assert_eq!(budget, 1);
    assert!(codelet_tools::read_image_budget::check_image_count(budget, 2, "images").is_err());
    assert!(
        originals(&id)?.records[0].envelope["message"]["content"] == images()?,
        "tool budget must not discard original images"
    );
    codelet_tools::model_capabilities::clear_session_model_max_images(uuid);
    Ok(())
}
