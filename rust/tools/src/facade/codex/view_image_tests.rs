//! Feature: spec/features/codex-view-image.feature
//!
//! BUG-112 tests, moved out of codex.rs (COV-075: one test file per feature).

use super::*;

// =========================================================================
// view_image facade tests
// Feature: spec/features/codex-view-image.feature
// =========================================================================

/// Scenario: CodexViewImageFacade maps view_image path to InternalFileParams::Read
#[test]
fn test_codex_view_image_facade_maps_path_to_read() {
    // @step Given a CodexViewImageFacade instance
    let facade = CodexViewImageFacade;

    // @step When the Codex model calls view_image with path "/tmp/screenshot.png"
    let input = json!({
        "path": "/tmp/screenshot.png"
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalFileParams::Read with file_path "/tmp/screenshot.png"
    assert_eq!(
        result,
        InternalFileParams::Read {
            file_path: "/tmp/screenshot.png".to_string(),
            offset: None,
            limit: None,
            mode: None,
            indentation: None,
        }
    );

    // @step And the facade tool name is "view_image"
    assert_eq!(facade.tool_name(), "view_image");

    // @step And the facade provider is "codex"
    assert_eq!(facade.provider(), "codex");
}

/// Scenario: CodexViewImageFacade accepts detail param for compatibility
#[test]
fn test_codex_view_image_facade_accepts_detail_param() {
    // @step Given a CodexViewImageFacade instance
    let facade = CodexViewImageFacade;

    // @step When the Codex model calls view_image with path and detail "original"
    let input = json!({
        "path": "/tmp/image.jpg",
        "detail": "original"
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalFileParams::Read ignoring the detail param
    assert_eq!(
        result,
        InternalFileParams::Read {
            file_path: "/tmp/image.jpg".to_string(),
            offset: None,
            limit: None,
            mode: None,
            indentation: None,
        }
    );
}

/// Scenario: CodexViewImageFacade rejects missing path
#[test]
fn test_codex_view_image_facade_missing_path() {
    // @step Given a CodexViewImageFacade instance
    let facade = CodexViewImageFacade;

    // @step When view_image is called with no path parameter
    let input = json!({});
    let result = facade.map_params(input);

    // @step Then the facade returns a validation error for tool "view_image" mentioning "path"
    assert!(result.is_err());
    if let Err(ToolError::Validation { tool, message }) = result {
        assert_eq!(tool, "view_image");
        assert!(message.contains("path"));
    } else {
        panic!("Expected ToolError::Validation");
    }
}

/// Scenario: CodexViewImageFacade rejects empty path
#[test]
fn test_codex_view_image_facade_empty_path() {
    // @step Given a CodexViewImageFacade instance
    let facade = CodexViewImageFacade;

    // @step When view_image is called with an empty path
    let input = json!({
        "path": ""
    });
    let result = facade.map_params(input);

    // @step Then the facade returns an error
    assert!(result.is_err());
}

/// Scenario: Tool definition matches Codex CLI spec
#[test]
fn test_codex_view_image_schema_matches_codex_spec() {
    // @step Given a CodexViewImageFacade instance
    let facade = CodexViewImageFacade;

    // @step When the tool definition is requested
    let def = facade.definition();

    // @step Then the tool name is "view_image"
    assert_eq!(def.name, "view_image");

    // @step And the description mentions viewing a local image
    assert!(def.description.contains("View a local image"));

    // @step And the parameters schema has a required "path" property of type string
    assert_eq!(def.parameters["properties"]["path"]["type"], "string");
    assert_eq!(def.parameters["required"], json!(["path"]));

    // @step And additionalProperties is false
    assert_eq!(def.parameters["additionalProperties"], false);

    // @step And a "detail" property exists for model compatibility
    assert!(def.parameters["properties"]["detail"].is_object());
}
