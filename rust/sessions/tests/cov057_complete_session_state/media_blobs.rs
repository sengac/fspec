//! COV-057: images, documents and large tool results go through the real
//! blob store on save (content-addressed, 10 KiB threshold, deduplicated)
//! and are rehydrated from it on a cold restore; URL references stay inline.

use super::persisted::{blob_files, fresh, restore, save, stored, tool_result, user, Outcome};
use super::*;
use base64::Engine;
use codelet_core::persistence::{
    blob_exists, compute_hash, extract_blob_hash, get_blob, DocumentSource, ImageSource,
    UserContent,
};

/// Deterministic, non-repeating binary payload of `size` bytes, base64 encoded.
fn base64_payload(size: usize) -> (Vec<u8>, String) {
    let bytes: Vec<u8> = (0..size)
        .map(|index| (index.wrapping_mul(31) ^ (index >> 7)) as u8)
        .collect();
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    (bytes, encoded)
}

fn document(source: DocumentSource) -> UserContent {
    UserContent::Document {
        source,
        title: None,
        context: None,
        cache_control: None,
    }
}

/// The on-disk value at `pointer` inside the single stored message.
fn stored_at(session: uuid::Uuid, pointer: &str) -> Outcome<serde_json::Value> {
    let raw = stored(session)?;
    assert_eq!(raw.len(), 1);
    Ok(raw[0].pointer(pointer).ok_or("missing field")?.clone())
}

fn only_part(session: uuid::Uuid) -> Outcome<UserContent> {
    let restored = restore(session)?;
    assert_eq!(restored.len(), 1);
    Ok(super::persisted::user_of(&restored[0])?[0].clone())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_base64_image_content_persisted_and_restored() -> TestResult {
    let (_directory, _manager, _, session) = fresh().await?;
    // @step Given a session with a user message containing an image:
    let (bytes, data) = base64_payload(50_000);
    let source = ImageSource::Base64 {
        media_type: "image/png".into(),
        data: data.clone(),
    };
    // @step When the session is saved and restored
    save(session, &[user(vec![UserContent::Image { source }])])?;
    let on_disk = stored_at(session, "/message/content/0/source/data")?;
    let part = only_part(session)?;
    // @step Then the image data is stored in blob storage
    let hash = extract_blob_hash(on_disk.as_str().ok_or("data")?).ok_or("no blob ref")?;
    assert_eq!(get_blob(hash)?, data.as_bytes());
    // @step And the restored message has content type "image"
    let UserContent::Image { source } = part else {
        return Err("expected an image part".into());
    };
    // @step And the restored image source type equals "base64"
    let ImageSource::Base64 { media_type, data } = source else {
        return Err("expected base64 source".into());
    };
    // @step And the restored image media_type equals "image/png"
    assert_eq!(media_type, "image/png");
    // @step And the restored image data matches the original bytes
    assert_eq!(
        base64::engine::general_purpose::STANDARD.decode(data)?,
        bytes
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_url_image_reference_persisted_inline() -> TestResult {
    let (_directory, _manager, _, session) = fresh().await?;
    // @step Given a session with a user message containing an image:
    let url = "https://example.com/screenshot.jpg";
    let source = ImageSource::Url { url: url.into() };
    // @step When the session is saved and restored
    save(session, &[user(vec![UserContent::Image { source }])])?;
    let on_disk = stored_at(session, "/message/content/0/source")?;
    let part = only_part(session)?;
    // @step Then the image URL is stored inline in the message
    assert_eq!(on_disk["url"], url);
    // @step And no blob reference is created for the image
    assert!(blob_files()?.is_empty());
    assert!(stored(session)?[0].get("_blobRefs").is_none());
    // @step And the restored image source type equals "url"
    // @step And the restored image url equals "https://example.com/screenshot.jpg"
    assert_eq!(
        part,
        UserContent::Image {
            source: ImageSource::Url { url: url.into() }
        }
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_base64_document_content_persisted_and_restored() -> TestResult {
    let (_directory, _manager, _, session) = fresh().await?;
    // @step Given a session with a user message containing a document:
    let (bytes, data) = base64_payload(100_000);
    let source = DocumentSource::Base64 {
        media_type: "application/pdf".into(),
        data: data.clone(),
    };
    // @step When the session is saved and restored
    save(session, &[user(vec![document(source)])])?;
    let on_disk = stored_at(session, "/message/content/0/source/data")?;
    let part = only_part(session)?;
    // @step Then the document data is stored in blob storage
    let hash = extract_blob_hash(on_disk.as_str().ok_or("data")?).ok_or("no blob ref")?;
    assert_eq!(get_blob(hash)?, data.as_bytes());
    // @step And the restored message has content type "document"
    let UserContent::Document { source, .. } = part else {
        return Err("expected a document part".into());
    };
    // @step And the restored document source type equals "base64"
    let DocumentSource::Base64 { media_type, data } = source else {
        return Err("expected base64 source".into());
    };
    // @step And the restored document media_type equals "application/pdf"
    assert_eq!(media_type, "application/pdf");
    // @step And the restored document data matches the original bytes
    assert_eq!(
        base64::engine::general_purpose::STANDARD.decode(data)?,
        bytes
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_url_document_reference_persisted_inline() -> TestResult {
    let (_directory, _manager, _, session) = fresh().await?;
    // @step Given a session with a user message containing a document:
    let url = "https://example.com/report.pdf";
    // @step When the session is saved and restored
    save(
        session,
        &[user(vec![document(DocumentSource::Url {
            url: url.into(),
        })])],
    )?;
    let on_disk = stored_at(session, "/message/content/0/source")?;
    let part = only_part(session)?;
    // @step Then the document URL is stored inline in the message
    assert_eq!(on_disk["url"], url);
    assert!(blob_files()?.is_empty());
    // @step And the restored document source type equals "url"
    // @step And the restored document url equals "https://example.com/report.pdf"
    assert_eq!(part, document(DocumentSource::Url { url: url.into() }));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_content_above_10kb_threshold_stored_in_blob() -> TestResult {
    let (_directory, _manager, _, session) = fresh().await?;
    // @step Given a session with a tool result containing 15000 bytes of content
    let content = "a".repeat(15_000);
    // @step When the session is saved
    save(
        session,
        &[user(vec![tool_result("toolu_big", &content, false)])],
    )?;
    let reference = stored_at(session, "/message/content/0/content")?;
    let reference = reference.as_str().ok_or("content")?;
    // @step Then the content is stored in blob storage
    assert_eq!(blob_files()?.len(), 1);
    // @step And the message contains a blob hash reference
    let hash = extract_blob_hash(reference).ok_or("no blob hash reference")?;
    assert!(blob_exists(hash)?);
    // @step And the blob hash is a valid SHA-256 hex string
    assert!(hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert_eq!(hash, compute_hash(content.as_bytes()));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_content_at_exactly_10kb_threshold_stored_inline() -> TestResult {
    let (_directory, _manager, _, session) = fresh().await?;
    // @step Given a session with a tool result containing exactly 10240 bytes
    let content = "b".repeat(10_240);
    // @step When the session is saved
    save(
        session,
        &[user(vec![tool_result("toolu_edge", &content, false)])],
    )?;
    // @step Then the content is stored inline in the message
    assert_eq!(stored_at(session, "/message/content/0/content")?, content);
    // @step And no blob reference is created
    assert!(stored(session)?[0].get("_blobRefs").is_none());
    assert!(blob_files()?.is_empty());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_duplicate_large_content_deduplicated_by_hash() -> TestResult {
    let (_directory, _manager, _, session) = fresh().await?;
    // @step Given a session with two tool results containing identical 20KB content
    let content = "dup-".repeat(5 * 1024);
    let first = user(vec![tool_result("toolu_one", &content, false)]);
    let second = user(vec![tool_result("toolu_two", &content, false)]);
    // @step When the session is saved
    save(session, &[first, second])?;
    // @step Then only one blob is stored
    assert_eq!(blob_files()?.len(), 1);
    // @step And both messages reference the same blob hash
    let references: Vec<_> = stored(session)?
        .iter()
        .map(|raw| raw.pointer("/message/content/0/content").cloned())
        .collect();
    assert_eq!(references.len(), 2);
    assert_eq!(references[0], references[1]);
    let reference = references[0].as_ref().and_then(|value| value.as_str());
    assert!(reference.and_then(extract_blob_hash).is_some());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_blob_content_retrieved_on_restore() -> TestResult {
    let (_directory, _manager, _, session) = fresh().await?;
    // @step Given a session with a tool result stored as blob reference
    let content: String = (0..2_000).map(|index| format!("{index:05}\n")).collect();
    save(
        session,
        &[user(vec![tool_result("toolu_blob", &content, false)])],
    )?;
    let reference = stored_at(session, "/message/content/0/content")?;
    assert!(extract_blob_hash(reference.as_str().ok_or("content")?).is_some());
    // @step When the session is restored
    let part = only_part(session)?;
    // @step Then the blob content is retrieved from storage
    let UserContent::ToolResult {
        content: restored, ..
    } = part
    else {
        return Err("expected a tool result part".into());
    };
    assert_ne!(serde_json::Value::String(restored.clone()), reference);
    // @step And the restored tool result content matches the original
    assert_eq!(restored, content);
    Ok(())
}
