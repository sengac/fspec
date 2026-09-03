#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! BUG-168: vision-capability-aware PDF default mode.
//!
//! Feature: spec/features/add-pdf-reading-support-to-read-tool.feature
//! Feature: spec/features/session-model-capabilities-registry.feature
//!
//! RED PHASE: references `codelet_tools::model_capabilities` (session-scoped
//! capability registry) which does not exist yet, so this target fails to
//! compile until the implementation lands.

use codelet_tools::model_capabilities::{
    clear_all_model_capabilities, session_has_capabilities, session_supports_vision,
    set_session_model_vision,
};
use codelet_tools::read::{ReadArgs, ReadTool};
use lopdf::{dictionary, Document, Object, Stream};
use rig::tool::Tool;
use serial_test::serial;
use uuid::Uuid;

/// Feature: spec/features/add-pdf-reading-support-to-read-tool.feature
#[tokio::test]
#[serial]
async fn scenario_non_vision_session_model_defaults_pdf_reads_to_text_mode() {
    // @step Given a PDF file "doc.pdf" with 3 pages
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let file_path = temp_dir.path().join("doc.pdf");
    std::fs::write(&file_path, create_test_pdf_with_pages(3)).expect("write test PDF");

    // @step And the session model is registered in the tool layer as lacking vision capability
    let session = Uuid::new_v4();
    set_session_model_vision(session, false);
    assert!(
        session_has_capabilities(session),
        "registry entry must be present"
    );

    // @step When the read tool is called with no pdf_mode specified
    let read_tool = ReadTool::new(session);
    let result = read_tool
        .call(ReadArgs {
            file_path: file_path.to_string_lossy().to_string(),
            offset: None,
            limit: None,
            pdf_mode: None,
        })
        .await
        .expect("PDF read should succeed");

    // @step Then text mode should be used automatically
    let parsed: serde_json::Value =
        serde_json::from_str(&result).expect("Read output must be valid JSON");
    let content = parsed
        .get("content")
        .and_then(|c| c.as_str())
        .expect("content");
    assert!(
        !content.contains("\"pages\""),
        "visual JSON must NOT be returned for a non-vision session: {content}"
    );
    assert!(
        content.contains("Page 1") || content.contains("page 1"),
        "text-mode page markers expected in output"
    );

    // @step And the output should include a one-line notice that visual mode is unavailable for this model
    assert!(
        content.to_lowercase().contains("vision"),
        "output must note that visual mode is unavailable: {content}"
    );

    // @step And no page images should be returned
    assert!(
        !content.contains("image/png"),
        "no image media types in a text fallback: {content}"
    );

    clear_all_model_capabilities();
}

/// Feature: spec/features/add-pdf-reading-support-to-read-tool.feature
#[tokio::test]
#[serial]
async fn scenario_explicit_pdf_mode_wins_even_when_the_session_model_lacks_vision() {
    // @step Given a PDF file "doc.pdf" with 3 pages
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let file_path = temp_dir.path().join("doc.pdf");
    std::fs::write(&file_path, create_test_pdf_with_pages(3)).expect("write test PDF");

    // @step And the session model is registered in the tool layer as lacking vision capability
    let session = Uuid::new_v4();
    set_session_model_vision(session, false);

    // @step When the read tool is called with pdf_mode="visual" explicitly
    let read_tool = ReadTool::new(session);
    let result = read_tool
        .call(ReadArgs {
            file_path: file_path.to_string_lossy().to_string(),
            offset: None,
            limit: None,
            pdf_mode: Some("visual".to_string()),
        })
        .await
        .expect("explicit visual must be honored");

    // @step Then visual mode should be honored and pages rendered as images
    let parsed: serde_json::Value =
        serde_json::from_str(&result).expect("Read output must be valid JSON");
    let content = parsed
        .get("content")
        .and_then(|c| c.as_str())
        .expect("content");
    let inner: serde_json::Value =
        serde_json::from_str(content).expect("visual payload must be JSON");
    let pages = inner
        .get("pages")
        .and_then(|p| p.as_array())
        .expect("pages array");
    assert_eq!(
        pages.len(),
        3,
        "all 3 pages rendered when visual is explicit"
    );

    // @step And no vision-unavailable notice should be present
    assert!(
        !content.to_lowercase().contains("vision"),
        "no vision notice when the user explicitly chose visual: {content}"
    );

    clear_all_model_capabilities();
}

/// Feature: spec/features/add-pdf-reading-support-to-read-tool.feature
#[tokio::test]
#[serial]
async fn scenario_unregistered_session_keeps_the_historical_visual_default() {
    // @step Given a PDF file "doc.pdf" with 3 pages
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let file_path = temp_dir.path().join("doc.pdf");
    std::fs::write(&file_path, create_test_pdf_with_pages(3)).expect("write test PDF");

    // @step And the session model capability is NOT registered in the tool layer
    let session = Uuid::new_v4();
    assert!(!session_has_capabilities(session), "no entry yet");

    // @step When the read tool is called with no pdf_mode specified
    let read_tool = ReadTool::new(session);
    let result = read_tool
        .call(ReadArgs {
            file_path: file_path.to_string_lossy().to_string(),
            offset: None,
            limit: None,
            pdf_mode: None,
        })
        .await
        .expect("PDF read should succeed");

    // @step Then visual mode should be used (historical default preserved)
    let parsed: serde_json::Value =
        serde_json::from_str(&result).expect("Read output must be valid JSON");
    let content = parsed
        .get("content")
        .and_then(|c| c.as_str())
        .expect("content");
    let inner: serde_json::Value =
        serde_json::from_str(content).expect("visual payload must be JSON");
    let pages = inner
        .get("pages")
        .and_then(|p| p.as_array())
        .expect("pages array");
    assert_eq!(pages.len(), 3, "visual default for unregistered sessions");

    // @step And no vision-unavailable notice should be present
    assert!(
        !content.to_lowercase().contains("vision"),
        "no vision notice for unregistered sessions: {content}"
    );
}

/// Feature: spec/features/session-model-capabilities-registry.feature
#[test]
#[serial]
fn registry_get_set_has_clear() {
    // @step Given a fresh registry with no entry for the session
    let session = Uuid::new_v4();
    assert!(!session_has_capabilities(session));

    // @step When a capability is registered for the session
    set_session_model_vision(session, true);

    // @step Then the registry reports it
    assert!(session_has_capabilities(session));
    assert!(session_supports_vision(session));

    // @step When the capability is updated to false
    set_session_model_vision(session, false);

    // @step Then the registry reports the update
    assert!(!session_supports_vision(session));
    assert!(session_has_capabilities(session));

    // @step When all entries are cleared
    clear_all_model_capabilities();

    // @step Then the registry has no entry
    assert!(!session_has_capabilities(session));
}

/// Create a test PDF with `page_count` text pages.
fn create_test_pdf_with_pages(page_count: usize) -> Vec<u8> {
    let pages: Vec<String> = (1..=page_count)
        .map(|i| format!("Report page {i} content"))
        .collect();
    let refs: Vec<&str> = pages.iter().map(std::string::String::as_str).collect();
    build_pdf_with_text_pages(&refs)
}

fn build_pdf_with_text_pages(page_contents: &[&str]) -> Vec<u8> {
    let mut doc = Document::with_version("1.5");

    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });

    let resources_id = doc.add_object(dictionary! {
        "Font" => dictionary! {
            "F1" => font_id,
        },
    });

    let mut page_ids = Vec::new();
    for content in page_contents {
        let content_stream = format!(
            "BT\n/F1 12 Tf\n50 700 Td\n({}) Tj\nET",
            content.replace('(', "\\(").replace(')', "\\)")
        );
        let content_id = doc.add_object(Stream::new(
            dictionary! {},
            content_stream.as_bytes().to_vec(),
        ));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
            "Resources" => resources_id,
            "Contents" => content_id,
        });
        page_ids.push(page_id);
    }

    let page_refs: Vec<Object> = page_ids.iter().map(|id| (*id).into()).collect();
    let pages_id = doc.add_object(dictionary! {
        "Type" => "Pages",
        "Count" => page_ids.len() as i64,
        "Kids" => page_refs,
    });

    for page_id in &page_ids {
        if let Ok(Object::Dictionary(ref mut dict)) = doc.get_object_mut(*page_id) {
            dict.set("Parent", pages_id);
        }
    }

    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);

    let mut buffer = Vec::new();
    doc.save_to(&mut buffer).unwrap_or_default();
    buffer
}
