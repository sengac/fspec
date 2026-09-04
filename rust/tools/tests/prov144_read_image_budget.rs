//! PROV-144: per-profile Max Images limit + Read tool image budget enforcement.
//!
//! Feature: spec/features/per-profile-max-images-limit-read-tool-image-budget-enforcement.feature
//!
//! RED PHASE: references `codelet_tools::model_capabilities::{
//! set_session_model_max_images, session_model_max_images,
//! session_has_max_images }` and the `read_image_budget` module, which do not
//! exist yet — this target fails to compile until the implementation lands.
//!
//! Enforcement contract (mirrors the session-model-capabilities-registry
//! pattern):
//!   * budget 0  -> any read that WOULD return an image FAILS with a
//!     "no vision" message naming maxImages=0.
//!   * budget n>=1 -> a single image file always passes; a PDF read that would
//!     return more pages/embedded images than n FAILS with a message naming
//!     the limit + the requested count + how to read fewer (offset/limit) or
//!     raise the profile's Max Images setting.
//!   * registry entry absent (non-profile session / unregistered) -> default
//!     budget of 4.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_tools::model_capabilities::{
    clear_all_model_capabilities, session_model_max_images, set_session_model_max_images,
};
use codelet_tools::read::{ReadArgs, ReadTool};
use codelet_tools::ToolError;
use lopdf::{dictionary, Document, Object, Stream};
use rig::tool::Tool;
use serial_test::serial;
use uuid::Uuid;

/// Feature: spec/features/per-profile-max-images-limit-read-tool-image-budget-enforcement.feature
#[tokio::test]
#[serial]
async fn scenario_no_vision_profile_fails_image_file_reads_with_a_clear_message() {
    // @step Given a session against a profile with maxImages 0
    let session = Uuid::new_v4();
    set_session_model_max_images(session, Some(0));

    // @step When the Read tool is called on a PNG image file
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let png_path = temp_dir.path().join("screenshot.png");
    std::fs::write(&png_path, minimal_png()).expect("write PNG");
    let result = read_call(
        &ReadTool::new(session),
        png_path.to_string_lossy().to_string(),
        None,
        None,
        None,
    )
    .await;

    // @step Then the tool call fails with a message stating image reading is disabled because the profile is configured with Max Images 0 (no vision)
    let err = result.expect_err("image-file read must fail for a no-vision budget");
    let msg = err.to_string();
    assert!(
        msg.contains("image reading is disabled")
            || msg.to_lowercase().contains("image reading is disabled"),
        "message must state image reading is disabled: {msg}"
    );
    assert!(
        msg.contains("0"),
        "message must name the budget 0 (no vision): {msg}"
    );

    // @step And the message points at text-based alternatives
    assert!(
        msg.to_lowercase().contains("text") || msg.to_lowercase().contains("pdf_mode='text'"),
        "message must point at text-based alternatives: {msg}"
    );

    // @step When the Read tool is called on a text file
    let text_path = temp_dir.path().join("notes.txt");
    std::fs::write(&text_path, b"hello world").expect("write text");
    let text_result = read_call(
        &ReadTool::new(session),
        text_path.to_string_lossy().to_string(),
        None,
        None,
        None,
    )
    .await;

    // @step Then the call succeeds and returns the text content
    let text = text_result.expect("text-file read must still succeed");
    assert!(
        text.contains("hello world"),
        "text content expected in output: {text}"
    );

    clear_all_model_capabilities();
}

/// Feature: spec/features/per-profile-max-images-limit-read-tool-image-budget-enforcement.feature
#[tokio::test]
#[serial]
async fn scenario_no_vision_profile_forces_pdf_reads_to_text_mode() {
    // @step Given a session against a profile with maxImages 0
    let session = Uuid::new_v4();
    set_session_model_max_images(session, Some(0));

    // @step When the Read tool is called on a PDF with no pdf_mode specified
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let pdf_path = temp_dir.path().join("doc.pdf");
    std::fs::write(&pdf_path, create_test_pdf_with_pages(3)).expect("write PDF");
    let result = read_call(
        &ReadTool::new(session),
        pdf_path.to_string_lossy().to_string(),
        None,
        None,
        None,
    )
    .await;

    // @step Then text mode is used automatically
    let content = extract_text(&result.expect("default-mode PDF read must succeed"));
    assert!(
        content.contains("Page 1") || content.to_lowercase().contains("page 1"),
        "text-mode page markers expected in output: {content}"
    );

    // @step And no page images are returned
    assert!(
        !content.contains("image/png"),
        "no image media types may be returned for a no-vision budget: {content}"
    );

    clear_all_model_capabilities();
}

/// Feature: spec/features/per-profile-max-images-limit-read-tool-image-budget-enforcement.feature
#[tokio::test]
#[serial]
async fn scenario_pdf_read_exceeding_the_image_budget_fails_with_the_limit_message() {
    // @step Given a session against a profile with maxImages 2
    let session = Uuid::new_v4();
    set_session_model_max_images(session, Some(2));

    // @step And a PDF file with 10 pages
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let pdf_path = temp_dir.path().join("report.pdf");
    std::fs::write(&pdf_path, create_test_pdf_with_pages(10)).expect("write PDF");
    let pdf_str = pdf_path.to_string_lossy().to_string();

    // @step When the Read tool is called on the PDF in visual mode with no limit
    let result = read_call(&ReadTool::new(session), pdf_str.clone(), None, None, None).await;

    // @step Then the tool call fails with a message naming the limit 2, the requested page count, and how to read fewer with offset/limit
    let err = result.expect_err("an over-budget visual PDF read must fail");
    let msg = err.to_string();
    assert!(msg.contains("2"), "message must name the limit 2: {msg}");
    assert!(
        msg.contains("10"),
        "message must name the requested page count (10 pages): {msg}"
    );
    assert!(
        msg.contains("offset") || msg.contains("limit"),
        "message must advise reading fewer with offset/limit: {msg}"
    );

    // @step When the Read tool is retried on the PDF with limit 2
    let retry = read_call(
        &ReadTool::new(session),
        pdf_str,
        None,
        Some(2),
        Some("visual".to_string()),
    )
    .await;

    // @step Then the call succeeds and returns exactly 2 page images
    let content = extract_text(&retry.expect("a limit=2 retry must succeed"));
    let inner: serde_json::Value =
        serde_json::from_str(&content).expect("visual payload must be JSON");
    let pages = inner
        .get("pages")
        .and_then(|p| p.as_array())
        .expect("pages array");
    assert_eq!(
        pages.len(),
        2,
        "exactly 2 page images expected when limit=2 matches the budget"
    );

    clear_all_model_capabilities();
}

/// Feature: spec/features/per-profile-max-images-limit-read-tool-image-budget-enforcement.feature
#[tokio::test]
#[serial]
async fn scenario_pdf_read_within_the_image_budget_succeeds() {
    // @step Given a session against a profile with maxImages 4
    let session = Uuid::new_v4();
    set_session_model_max_images(session, Some(4));

    // @step And a PDF file with 3 pages
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let pdf_path = temp_dir.path().join("small.pdf");
    std::fs::write(&pdf_path, create_test_pdf_with_pages(3)).expect("write PDF");

    // @step When the Read tool is called on the PDF in visual mode
    let result = read_call(
        &ReadTool::new(session),
        pdf_path.to_string_lossy().to_string(),
        None,
        None,
        Some("visual".to_string()),
    )
    .await;

    // @step Then the call succeeds and returns 3 page images
    let content = extract_text(&result.expect("within-budget visual read must succeed"));
    let inner: serde_json::Value =
        serde_json::from_str(&content).expect("visual payload must be JSON");
    let pages = inner
        .get("pages")
        .and_then(|p| p.as_array())
        .expect("pages array");
    assert_eq!(pages.len(), 3, "3 page images expected (3 <= budget 4)");

    // @step When the Read tool is called on a single PNG image file
    let png_path = temp_dir.path().join("shot.png");
    std::fs::write(&png_path, minimal_png()).expect("write PNG");
    let img = read_call(
        &ReadTool::new(session),
        png_path.to_string_lossy().to_string(),
        None,
        None,
        None,
    )
    .await;

    // @step Then the call succeeds and returns 1 image
    let output: serde_json::Value =
        serde_json::from_str(&img.expect("single-image read must succeed")).expect("JSON output");
    assert_eq!(
        output["type"], "image",
        "a single image read returns exactly one image"
    );

    clear_all_model_capabilities();
}

/// Feature: spec/features/per-profile-max-images-limit-read-tool-image-budget-enforcement.feature
#[tokio::test]
#[serial]
async fn scenario_pdf_images_mode_read_exceeding_the_budget_fails() {
    // @step Given a session against a profile with maxImages 1
    let session = Uuid::new_v4();
    set_session_model_max_images(session, Some(1));

    // @step And a PDF file with 5 embedded images
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let pdf_path = temp_dir.path().join("catalog.pdf");
    std::fs::write(&pdf_path, create_test_pdf_with_images(5)).expect("write PDF");

    // @step When the Read tool is called on the PDF with pdf_mode images and no limit
    let result = read_call(
        &ReadTool::new(session),
        pdf_path.to_string_lossy().to_string(),
        None,
        None,
        Some("images".to_string()),
    )
    .await;

    // @step Then the tool call fails with a message naming the limit 1 and how to read fewer
    let err = result.expect_err("an over-budget images-mode read must fail");
    let msg = err.to_string();
    assert!(msg.contains("1"), "message must name the limit 1: {msg}");
    assert!(
        msg.contains("offset") || msg.contains("limit"),
        "message must advise reading fewer with offset/limit: {msg}"
    );

    clear_all_model_capabilities();
}

/// Feature: spec/features/per-profile-max-images-limit-read-tool-image-budget-enforcement.feature
#[tokio::test]
#[serial]
async fn scenario_a_mid_session_model_switch_updates_the_image_budget() {
    // @step Given a session against a profile with maxImages 8
    let session = Uuid::new_v4();
    set_session_model_max_images(session, Some(8));

    let temp_dir = tempfile::tempdir().expect("temp dir");
    let png_path = temp_dir.path().join("shot.png");
    std::fs::write(&png_path, minimal_png()).expect("write PNG");
    let png_str = png_path.to_string_lossy().to_string();

    // The budget of 8 permits a single-image read.
    let ok = read_call(&ReadTool::new(session), png_str.clone(), None, None, None)
        .await
        .expect("a single image passes an 8-image budget");
    let _ = ok;

    // @step When the session switches mid-session to a profile with maxImages 0
    // (the set-site registry update)
    set_session_model_max_images(session, Some(0));

    // @step Then the Read tool fails subsequent image-file reads with the no-vision message
    let err = read_call(&ReadTool::new(session), png_str, None, None, None)
        .await
        .expect_err("after the switch the image read must fail");
    let msg = err.to_string();
    assert!(
        msg.to_lowercase().contains("image reading is disabled"),
        "the no-vision message must be returned after the mid-session switch: {msg}"
    );

    // @step And the session was not recreated
    // (the same Uuid now resolves the no-vision budget — no new session id)
    assert_eq!(
        session_model_max_images(session),
        Some(0),
        "the same session Uuid must now resolve the no-vision budget"
    );

    clear_all_model_capabilities();
}

/// Feature: spec/features/per-profile-max-images-limit-read-tool-image-budget-enforcement.feature
#[tokio::test]
#[serial]
async fn scenario_a_non_profile_session_resolves_the_default_budget_of_4() {
    // @step Given a session created with a cloud model that has no profile behind it
    // (no max-images registry entry -> default budget of 4)
    let session = Uuid::nil();
    clear_all_model_capabilities();
    assert!(
        session_model_max_images(session).is_none(),
        "a non-profile session must have no max-images entry (default 4 applies)"
    );

    // @step When the Read tool is called on a PDF with 5 pages in visual mode
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let pdf_path = temp_dir.path().join("cloud.pdf");
    std::fs::write(&pdf_path, create_test_pdf_with_pages(5)).expect("write PDF");
    let over = read_call(
        &ReadTool::new(session),
        pdf_path.to_string_lossy().to_string(),
        None,
        None,
        Some("visual".to_string()),
    )
    .await;

    // @step Then the tool call fails with an over-budget message naming the limit 4
    let err = over.expect_err("5 pages exceeds the default budget of 4");
    let msg = err.to_string();
    assert!(
        msg.contains("4"),
        "message must name the default limit 4: {msg}"
    );

    // @step When the Read tool is called on a single JPEG image file
    let jpeg_path = temp_dir.path().join("photo.jpg");
    std::fs::write(&jpeg_path, minimal_jpeg()).expect("write JPEG");
    let jpeg = read_call(
        &ReadTool::new(session),
        jpeg_path.to_string_lossy().to_string(),
        None,
        None,
        None,
    )
    .await;

    // @step Then the call succeeds
    let _ = jpeg.expect("a single JPEG is within the default budget of 4");

    clear_all_model_capabilities();
}

/// Feature: spec/features/per-profile-max-images-limit-read-tool-image-budget-enforcement.feature
#[test]
#[serial]
fn registry_get_set_has_clear_for_max_images() {
    // @step Given a fresh registry with no max-images entry for the session
    let session = Uuid::new_v4();
    assert!(session_model_max_images(session).is_none());

    // @step When a max-images budget is registered for the session
    set_session_model_max_images(session, Some(7));

    // @step Then the registry reports it
    assert_eq!(session_model_max_images(session), Some(7));

    // @step When the budget is updated to 0
    set_session_model_max_images(session, Some(0));

    // @step Then the registry reports the update
    assert_eq!(session_model_max_images(session), Some(0));

    // @step When all entries are cleared
    clear_all_model_capabilities();

    // @step Then the registry has no entry
    assert!(session_model_max_images(session).is_none());
}

// ─────────────────────────────────────────────────────────────────────────────
// Test helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Call the Read tool and return the raw string output.
async fn read_call(
    tool: &ReadTool,
    file_path: String,
    offset: Option<usize>,
    limit: Option<usize>,
    pdf_mode: Option<String>,
) -> Result<String, ToolError> {
    tool.call(ReadArgs {
        file_path,
        offset,
        limit,
        pdf_mode,
    })
    .await
}

/// Extract the `content` field (a string) from a `ReadOutput::Text` JSON.
fn extract_text(raw: &str) -> String {
    let parsed: serde_json::Value = serde_json::from_str(raw).expect("Read output must be JSON");
    parsed
        .get("content")
        .and_then(|c| c.as_str())
        .expect("content")
        .to_string()
}

/// A minimal valid 1x1 red PNG (69 bytes).
fn minimal_png() -> Vec<u8> {
    vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77,
        0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xFF,
        0xFF, 0x3F, 0x00, 0x05, 0xFE, 0x02, 0xFE, 0xDC, 0xCC, 0x59, 0xE7, 0x00, 0x00, 0x00, 0x00,
        0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ]
}

/// A minimal JPEG marker (JFIF SOI + APP0 start).
fn minimal_jpeg() -> Vec<u8> {
    vec![
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01,
    ]
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

/// Create a test PDF whose pages each embed one small JPEG image XObject.
fn create_test_pdf_with_images(image_count: usize) -> Vec<u8> {
    let mut doc = Document::with_version("1.5");

    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });

    let mut page_ids = Vec::new();
    for i in 1..=image_count {
        let image_id = doc.add_object(Stream::new(
            dictionary! {
                "Type" => "XObject",
                "Subtype" => "Image",
                "Width" => 4,
                "Height" => 4,
                "ColorSpace" => "DeviceRGB",
                "BitsPerComponent" => 8,
                "Filter" => "DCTDecode",
            },
            vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x00, 0x01],
        ));
        let xobject_name = format!("Im{i}");
        let resources = doc.add_object(dictionary! {
            "Font" => dictionary! {
                "F1" => font_id,
            },
            "XObject" => dictionary! {
                xobject_name.as_str() => image_id,
            },
        });

        let content_stream =
            format!("BT /F1 12 Tf 50 700 Td (Product catalog photo {i}) Tj ET\n/{xobject_name} Do");
        let content_id = doc.add_object(Stream::new(
            dictionary! {},
            content_stream.as_bytes().to_vec(),
        ));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
            "Resources" => resources,
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
