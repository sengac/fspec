#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! BUG-168: PDF pagination — offset/limit honored in all modes, configurable
//! default page cap, truncation notice.
//!
//! Feature: spec/features/add-pdf-reading-support-to-read-tool.feature
//!
//! These tests exercise the Read tool end-to-end (temp file -> ReadTool::call)
//! and use only the existing `ReadArgs`/`ReadTool` API, so they compile
//! against pre-fix code and fail at runtime (red phase).

use codelet_tools::read::{ReadArgs, ReadTool};
use lopdf::{dictionary, Document, Object, Stream};
use rig::tool::Tool;
use serial_test::serial;
use uuid::Uuid;

/// Feature: spec/features/add-pdf-reading-support-to-read-tool.feature
#[tokio::test]
#[serial]
async fn scenario_visual_mode_honors_offset_and_limit_for_pdf_pages() {
    // @step Given a PDF file "requirements.pdf" with 67 pages
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let file_path = temp_dir.path().join("requirements.pdf");
    let pdf_bytes = create_test_pdf_with_pages(67);
    std::fs::write(&file_path, &pdf_bytes).expect("write test PDF");

    // @step When the read tool is called with pdf_mode="visual", offset=1, limit=4
    let read_tool = ReadTool::new(Uuid::nil());
    let result = read_tool
        .call(ReadArgs {
            file_path: file_path.to_string_lossy().to_string(),
            offset: Some(1),
            limit: Some(4),
            pdf_mode: Some("visual".to_string()),
        })
        .await
        .expect("PDF read should succeed");
    let parsed = parse_pdf_json_content(&result);

    // @step Then exactly 4 rendered pages should be returned covering pages 1 through 4
    let pages = parsed
        .get("pages")
        .and_then(|p| p.as_array())
        .expect("pages array");
    assert_eq!(pages.len(), 4, "limit=4 must return exactly 4 pages");
    let page_numbers: Vec<u64> = pages
        .iter()
        .map(|p| {
            p.get("page_number")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0)
        })
        .collect();
    assert_eq!(page_numbers, vec![1, 2, 3, 4]);

    // @step And the response should include the total page count (67)
    assert_eq!(
        parsed
            .get("total_pages")
            .and_then(serde_json::Value::as_u64),
        Some(67),
        "total_pages must report the document size"
    );

    // @step And a truncation notice should tell the reader the document has more pages
    let notice = parsed
        .get("notice")
        .and_then(|n| n.as_str())
        .expect("truncation notice present when limit < total");
    assert!(
        notice.contains("of 67"),
        "notice must state the total page count: {notice}"
    );

    // @step And the truncation notice should name the next offset (5) to continue reading with
    assert!(
        notice.contains("offset=5"),
        "notice must name the next offset: {notice}"
    );
}

/// Feature: spec/features/add-pdf-reading-support-to-read-tool.feature
#[tokio::test]
#[serial]
async fn scenario_visual_mode_honors_an_offset_that_starts_mid_document() {
    // @step Given a PDF file "report.pdf" with 10 pages
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let file_path = temp_dir.path().join("report.pdf");
    std::fs::write(&file_path, create_test_pdf_with_pages(10)).expect("write test PDF");

    // @step When the read tool is called with pdf_mode="visual", offset=3, limit=2
    let read_tool = ReadTool::new(Uuid::nil());
    let result = read_tool
        .call(ReadArgs {
            file_path: file_path.to_string_lossy().to_string(),
            offset: Some(3),
            limit: Some(2),
            pdf_mode: Some("visual".to_string()),
        })
        .await
        .expect("PDF read should succeed");
    let parsed = parse_pdf_json_content(&result);
    let pages = parsed
        .get("pages")
        .and_then(|p| p.as_array())
        .expect("pages array");

    // @step Then exactly 2 rendered pages should be returned covering pages 3 and 4
    let page_numbers: Vec<u64> = pages
        .iter()
        .map(|p| {
            p.get("page_number")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0)
        })
        .collect();
    assert_eq!(
        page_numbers,
        vec![3, 4],
        "offset=3 limit=2 must return pages 3 and 4"
    );

    // @step And the truncation notice should name the next offset (5) to continue reading with
    let notice = parsed
        .get("notice")
        .and_then(|n| n.as_str())
        .expect("truncation notice present");
    assert!(
        notice.contains("offset=5"),
        "next offset must be 5: {notice}"
    );
}

/// Feature: spec/features/add-pdf-reading-support-to-read-tool.feature
#[tokio::test]
#[serial]
async fn scenario_text_mode_honors_offset_and_limit_for_pdf_pages() {
    // @step Given a PDF file "spec.pdf" with 10 pages of text
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let file_path = temp_dir.path().join("spec.pdf");
    std::fs::write(&file_path, create_test_pdf_with_pages(10)).expect("write test PDF");

    // @step When the read tool is called with pdf_mode="text", offset=2, limit=3
    let read_tool = ReadTool::new(Uuid::nil());
    let result = read_tool
        .call(ReadArgs {
            file_path: file_path.to_string_lossy().to_string(),
            offset: Some(2),
            limit: Some(3),
            pdf_mode: Some("text".to_string()),
        })
        .await
        .expect("PDF text-mode read should succeed");

    // @step Then text should be extracted only from pages 2 through 4
    assert!(result.contains("Page 2"));
    assert!(result.contains("Page 3"));
    assert!(result.contains("Page 4"));
    assert!(
        !result.contains("Page 1 ---"),
        "page 1 must not be included when offset=2"
    );
    assert!(
        !result.contains("Page 5 ---"),
        "page 5 must not be included when offset=2 limit=3"
    );

    // @step And the output should include the total page count (10)
    assert!(
        result.contains("10 pages") || result.contains("of 10"),
        "output must state the document has 10 pages"
    );

    // @step And a truncation notice should name the next offset (5) to continue reading with
    assert!(
        result.contains("offset=5"),
        "truncation notice must name the next offset: {result}"
    );
}

/// Feature: spec/features/add-pdf-reading-support-to-read-tool.feature
#[tokio::test]
#[serial]
async fn scenario_images_mode_honors_offset_and_limit_for_embedded_images() {
    // @step Given a PDF file "catalog.pdf" with 6 embedded images
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let file_path = temp_dir.path().join("catalog.pdf");
    std::fs::write(&file_path, create_test_pdf_with_images(6)).expect("write test PDF");

    // @step When the read tool is called with pdf_mode="images", offset=2, limit=3
    let read_tool = ReadTool::new(Uuid::nil());
    let result = read_tool
        .call(ReadArgs {
            file_path: file_path.to_string_lossy().to_string(),
            offset: Some(2),
            limit: Some(3),
            pdf_mode: Some("images".to_string()),
        })
        .await
        .expect("PDF images-mode read should succeed");
    let parsed = parse_pdf_json_content(&result);
    let images = parsed
        .get("images")
        .and_then(|i| i.as_array())
        .expect("images array");

    // @step Then exactly 3 embedded images should be returned (the 2nd, 3rd and 4th)
    assert_eq!(images.len(), 3, "offset=2 limit=3 must return 3 images");
    let indexes: Vec<u64> = images
        .iter()
        .map(|i| {
            i.get("index")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0)
        })
        .collect();
    assert_eq!(indexes, vec![2, 3, 4]);

    // @step And the response should include the total image count (6)
    assert_eq!(
        parsed
            .get("image_count")
            .and_then(serde_json::Value::as_u64),
        Some(6),
        "image_count must report all 6 embedded images"
    );

    // @step And a truncation notice should name the next offset (5) to continue with
    let notice = parsed
        .get("notice")
        .and_then(|n| n.as_str())
        .expect("truncation notice present");
    assert!(
        notice.contains("offset=5"),
        "next offset must be 5: {notice}"
    );
}

/// Feature: spec/features/add-pdf-reading-support-to-read-tool.feature
#[tokio::test]
#[serial]
async fn scenario_default_page_cap_bounds_an_unbounded_pdf_read() {
    // @step Given a PDF file "huge.pdf" with 25 pages
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let file_path = temp_dir.path().join("huge.pdf");
    std::fs::write(&file_path, create_test_pdf_with_pages(25)).expect("write test PDF");

    // @step And CODELET_MAX_PDF_PAGES is at its default value
    std::env::remove_var("CODELET_MAX_PDF_PAGES");

    // @step When the read tool is called with pdf_mode="visual" and no offset or limit
    let read_tool = ReadTool::new(Uuid::nil());
    let result = read_tool
        .call(ReadArgs {
            file_path: file_path.to_string_lossy().to_string(),
            offset: None,
            limit: None,
            pdf_mode: Some("visual".to_string()),
        })
        .await
        .expect("PDF read should succeed");
    let parsed = parse_pdf_json_content(&result);
    let pages = parsed
        .get("pages")
        .and_then(|p| p.as_array())
        .expect("pages array");

    // @step Then at most the default cap (20) pages should be returned
    assert_eq!(
        pages.len(),
        20,
        "default cap must bound the read to 20 pages"
    );

    // @step And the response should include the total page count (25)
    assert_eq!(
        parsed
            .get("total_pages")
            .and_then(serde_json::Value::as_u64),
        Some(25)
    );

    // @step And a truncation notice should name the next offset (21) to continue reading with
    let notice = parsed
        .get("notice")
        .and_then(|n| n.as_str())
        .expect("truncation notice present");
    assert!(
        notice.contains("offset=21"),
        "next offset must be 21: {notice}"
    );
}

/// Feature: spec/features/add-pdf-reading-support-to-read-tool.feature
#[tokio::test]
#[serial]
async fn scenario_configurable_page_cap_via_environment_variable() {
    // @step Given a PDF file "configurable.pdf" with 25 pages
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let file_path = temp_dir.path().join("configurable.pdf");
    std::fs::write(&file_path, create_test_pdf_with_pages(25)).expect("write test PDF");

    // @step And CODELET_MAX_PDF_PAGES is set to 5
    std::env::set_var("CODELET_MAX_PDF_PAGES", "5");

    // @step When the read tool is called with pdf_mode="visual" and no offset or limit
    let read_tool = ReadTool::new(Uuid::nil());
    let result = read_tool
        .call(ReadArgs {
            file_path: file_path.to_string_lossy().to_string(),
            offset: None,
            limit: None,
            pdf_mode: Some("visual".to_string()),
        })
        .await
        .expect("PDF read should succeed");
    let parsed = parse_pdf_json_content(&result);
    let pages = parsed
        .get("pages")
        .and_then(|p| p.as_array())
        .expect("pages array");

    // @step Then exactly 5 pages should be returned
    assert_eq!(
        pages.len(),
        5,
        "CODELET_MAX_PDF_PAGES=5 must cap at 5 pages"
    );

    // @step And a truncation notice should name the next offset (6) to continue reading with
    let notice = parsed
        .get("notice")
        .and_then(|n| n.as_str())
        .expect("truncation notice present");
    assert!(
        notice.contains("offset=6"),
        "next offset must be 6: {notice}"
    );

    std::env::remove_var("CODELET_MAX_PDF_PAGES");
}

/// Feature: spec/features/add-pdf-reading-support-to-read-tool.feature
#[tokio::test]
#[serial]
async fn scenario_explicit_limit_wins_over_the_default_page_cap() {
    // @step Given a PDF file "limited.pdf" with 25 pages
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let file_path = temp_dir.path().join("limited.pdf");
    std::fs::write(&file_path, create_test_pdf_with_pages(25)).expect("write test PDF");

    // @step And CODELET_MAX_PDF_PAGES is set to 10
    std::env::set_var("CODELET_MAX_PDF_PAGES", "10");

    // @step When the read tool is called with pdf_mode="visual", offset=1, limit=3
    let read_tool = ReadTool::new(Uuid::nil());
    let result = read_tool
        .call(ReadArgs {
            file_path: file_path.to_string_lossy().to_string(),
            offset: Some(1),
            limit: Some(3),
            pdf_mode: Some("visual".to_string()),
        })
        .await
        .expect("PDF read should succeed");
    let parsed = parse_pdf_json_content(&result);
    let pages = parsed
        .get("pages")
        .and_then(|p| p.as_array())
        .expect("pages array");

    // @step Then exactly 3 pages should be returned
    assert_eq!(pages.len(), 3, "explicit limit must win over the env cap");

    // @step And a truncation notice should name the next offset (4) to continue reading with
    let notice = parsed
        .get("notice")
        .and_then(|n| n.as_str())
        .expect("truncation notice present");
    assert!(
        notice.contains("offset=4"),
        "next offset must be 4: {notice}"
    );

    std::env::remove_var("CODELET_MAX_PDF_PAGES");
}

/// Feature: spec/features/add-pdf-reading-support-to-read-tool.feature
#[tokio::test]
#[serial]
async fn scenario_a_pdf_that_fits_entirely_is_returned_without_a_truncation_notice() {
    // @step Given a PDF file "short.pdf" with 4 pages
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let file_path = temp_dir.path().join("short.pdf");
    std::fs::write(&file_path, create_test_pdf_with_pages(4)).expect("write test PDF");

    // @step And CODELET_MAX_PDF_PAGES is at its default value
    std::env::remove_var("CODELET_MAX_PDF_PAGES");

    // @step When the read tool is called with pdf_mode="visual" and no offset or limit
    let read_tool = ReadTool::new(Uuid::nil());
    let result = read_tool
        .call(ReadArgs {
            file_path: file_path.to_string_lossy().to_string(),
            offset: None,
            limit: None,
            pdf_mode: Some("visual".to_string()),
        })
        .await
        .expect("PDF read should succeed");
    let parsed = parse_pdf_json_content(&result);
    let pages = parsed
        .get("pages")
        .and_then(|p| p.as_array())
        .expect("pages array");

    // @step Then all 4 pages should be returned
    assert_eq!(pages.len(), 4, "all 4 pages must be returned");

    // @step And no truncation notice should be present
    let notice = parsed.get("notice").and_then(|n| n.as_str());
    assert!(
        notice.is_none_or(str::is_empty),
        "no truncation notice when the document fits entirely, got: {notice:?}"
    );
}

/// Feature: spec/features/add-pdf-reading-support-to-read-tool.feature
#[tokio::test]
#[serial]
async fn scenario_an_offset_beyond_the_document_still_reports_the_total() {
    // @step Given a PDF file "tail.pdf" with 5 pages
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let file_path = temp_dir.path().join("tail.pdf");
    std::fs::write(&file_path, create_test_pdf_with_pages(5)).expect("write test PDF");

    // @step When the read tool is called with pdf_mode="visual", offset=10, limit=4
    let read_tool = ReadTool::new(Uuid::nil());
    let result = read_tool
        .call(ReadArgs {
            file_path: file_path.to_string_lossy().to_string(),
            offset: Some(10),
            limit: Some(4),
            pdf_mode: Some("visual".to_string()),
        })
        .await
        .expect("PDF read should succeed");
    let parsed = parse_pdf_json_content(&result);
    let pages = parsed
        .get("pages")
        .and_then(|p| p.as_array())
        .expect("pages array");

    // @step Then zero rendered pages should be returned
    assert!(pages.is_empty(), "offset past end must return zero pages");

    // @step And the response should still include the total page count (5)
    assert_eq!(
        parsed
            .get("total_pages")
            .and_then(serde_json::Value::as_u64),
        Some(5)
    );

    // @step And a truncation-free notice should make clear there are no pages at that offset
    let notice = parsed
        .get("notice")
        .and_then(|n| n.as_str())
        .expect("notice present when the offset is past the end");
    assert!(
        !notice.contains("Continue"),
        "no pagination hint when there is nothing left to read: {notice}"
    );
}

/// Decode the Read tool output envelope ({"type":"text","content":"<json>"})
/// and parse the inner PDF JSON payload.
fn parse_pdf_json_content(output: &str) -> serde_json::Value {
    let envelope: serde_json::Value =
        serde_json::from_str(output).expect("Read output must be valid JSON");
    let content = envelope
        .get("content")
        .and_then(|c| c.as_str())
        .expect("Read output must carry a content string");
    serde_json::from_str(content).expect("PDF payload must be valid JSON")
}

/// Create a test PDF with `page_count` text pages (one line per page).
fn create_test_pdf_with_pages(page_count: usize) -> Vec<u8> {
    let pages: Vec<String> = (1..=page_count)
        .map(|i| format!("Report page {i} content"))
        .collect();
    let refs: Vec<&str> = pages.iter().map(std::string::String::as_str).collect();
    build_pdf_with_text_pages(&refs)
}

/// Create a test PDF whose pages each embed one small JPEG image XObject.
fn create_test_pdf_with_images(image_count: usize) -> Vec<u8> {
    let mut doc = Document::with_version("1.5");

    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });

    // One distinct image XObject per page so images mode sees `image_count` images.
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

/// Shared lopdf document builder (text pages; optional image XObject).
fn build_pdf_with_text_pages(page_contents: &[&str]) -> Vec<u8> {
    let mut doc = Document::with_version("1.5");

    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });

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

    let resources_id = doc.add_object(dictionary! {
        "Font" => dictionary! {
            "F1" => font_id,
        },
        "XObject" => dictionary! {
            "Im0" => image_id,
        },
    });

    let mut page_ids = Vec::new();
    for content in page_contents {
        let content_stream = format!(
            "BT /F1 12 Tf 50 700 Td ({}) Tj ET\n/Im0 Do",
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
