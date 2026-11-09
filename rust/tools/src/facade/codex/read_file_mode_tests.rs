//! Feature: spec/features/codex-read-file-mode-indentation.feature
//!
//! BUG-109 tests, moved out of codex.rs (COV-075: one test file per feature).

use super::*;

// =========================================================================
// BUG-109: read_file mode and indentation params tests
// Feature: spec/features/codex-read-file-mode-indentation.feature
// =========================================================================

/// Scenario: CodexReadFileFacade maps mode indentation to InternalFileParams::Read
#[test]
fn test_codex_read_file_maps_mode_indentation() {
    use crate::facade::traits::InternalIndentationParams;

    // @step Given a CodexReadFileFacade instance
    let facade = CodexReadFileFacade;

    // @step When the Codex model calls read_file with file_path "/src/main.rs" mode "indentation" and indentation {anchor_line: 50, max_levels: 2}
    let input = json!({
        "file_path": "/src/main.rs",
        "mode": "indentation",
        "indentation": {
            "anchor_line": 50,
            "max_levels": 2
        }
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalFileParams::Read with file_path "/src/main.rs"
    // @step And mode is Some("indentation")
    // @step And indentation anchor_line is Some(50)
    // @step And indentation max_levels is Some(2)
    // @step And indentation include_siblings is None
    // @step And indentation include_header is None
    // @step And indentation max_lines is None
    assert_eq!(
        result,
        InternalFileParams::Read {
            file_path: "/src/main.rs".to_string(),
            offset: None,
            limit: None,
            mode: Some("indentation".to_string()),
            indentation: Some(InternalIndentationParams {
                anchor_line: Some(50),
                max_levels: Some(2),
                include_siblings: None,
                include_header: None,
                max_lines: None,
            }),
        }
    );
}

/// Scenario: CodexReadFileFacade maps mode slice without indentation
#[test]
fn test_codex_read_file_maps_mode_slice() {
    // @step Given a CodexReadFileFacade instance
    let facade = CodexReadFileFacade;

    // @step When the Codex model calls read_file with file_path "/src/main.rs" and mode "slice"
    let input = json!({
        "file_path": "/src/main.rs",
        "mode": "slice"
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalFileParams::Read with file_path "/src/main.rs"
    // @step And mode is Some("slice")
    // @step And indentation is None
    assert_eq!(
        result,
        InternalFileParams::Read {
            file_path: "/src/main.rs".to_string(),
            offset: None,
            limit: None,
            mode: Some("slice".to_string()),
            indentation: None,
        }
    );
}

/// Scenario: CodexReadFileFacade backward compatible without mode or indentation
#[test]
fn test_codex_read_file_backward_compatible_no_mode_no_indentation() {
    // @step Given a CodexReadFileFacade instance
    let facade = CodexReadFileFacade;

    // @step When the Codex model calls read_file with only file_path "/src/main.rs"
    let input = json!({
        "file_path": "/src/main.rs"
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalFileParams::Read with file_path "/src/main.rs"
    // @step And mode is None
    // @step And indentation is None
    assert_eq!(
        result,
        InternalFileParams::Read {
            file_path: "/src/main.rs".to_string(),
            offset: None,
            limit: None,
            mode: None,
            indentation: None,
        }
    );
}

/// Scenario: Codex read_file schema includes mode and indentation properties
#[test]
fn test_codex_read_file_schema_has_mode_and_indentation() {
    // @step Given a CodexReadFileFacade instance
    let facade = CodexReadFileFacade;

    // @step When the tool definition schema is inspected
    let def = facade.definition();

    // @step Then the schema has a "mode" property of type "string"
    assert_eq!(def.parameters["properties"]["mode"]["type"], "string");

    // @step And the schema has an "indentation" property of type "object"
    assert_eq!(
        def.parameters["properties"]["indentation"]["type"],
        "object"
    );

    // @step And the indentation object has an "anchor_line" property of type "integer"
    assert_eq!(
        def.parameters["properties"]["indentation"]["properties"]["anchor_line"]["type"],
        "integer"
    );

    // @step And the indentation object has a "max_levels" property of type "integer"
    assert_eq!(
        def.parameters["properties"]["indentation"]["properties"]["max_levels"]["type"],
        "integer"
    );

    // @step And the indentation object has an "include_siblings" property of type "boolean"
    assert_eq!(
        def.parameters["properties"]["indentation"]["properties"]["include_siblings"]["type"],
        "boolean"
    );

    // @step And the indentation object has an "include_header" property of type "boolean"
    assert_eq!(
        def.parameters["properties"]["indentation"]["properties"]["include_header"]["type"],
        "boolean"
    );

    // @step And the indentation object has a "max_lines" property of type "integer"
    assert_eq!(
        def.parameters["properties"]["indentation"]["properties"]["max_lines"]["type"],
        "integer"
    );

    // @step And the indentation object has additionalProperties false
    assert_eq!(
        def.parameters["properties"]["indentation"]["additionalProperties"],
        false
    );
}

/// Scenario: Other facades provide None for mode and indentation fields
#[test]
fn test_zai_facade_provides_none_mode_and_indentation() {
    use crate::facade::zai::ZAIReadFileFacade;

    // @step Given a ZAIReadFileFacade instance
    let facade = ZAIReadFileFacade;

    // @step When the ZAI model calls read_file with file_path "/src/main.rs"
    let input = json!({
        "file_path": "/src/main.rs"
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalFileParams::Read with mode None and indentation None
    assert_eq!(
        result,
        InternalFileParams::Read {
            file_path: "/src/main.rs".to_string(),
            offset: None,
            limit: None,
            mode: None,
            indentation: None,
        }
    );
}

/// Scenario: CodexReadFileFacade extracts all indentation boolean and integer fields
#[test]
fn test_codex_read_file_extracts_all_indentation_fields() {
    use crate::facade::traits::InternalIndentationParams;

    // @step Given a CodexReadFileFacade instance
    let facade = CodexReadFileFacade;

    // @step When the Codex model calls read_file with file_path "/src/main.rs" mode "indentation" and indentation {include_siblings: true, include_header: true, max_lines: 100}
    let input = json!({
        "file_path": "/src/main.rs",
        "mode": "indentation",
        "indentation": {
            "include_siblings": true,
            "include_header": true,
            "max_lines": 100
        }
    });
    let result = facade.map_params(input).unwrap();

    // @step Then indentation include_siblings is Some(true)
    // @step And indentation include_header is Some(true)
    // @step And indentation max_lines is Some(100)
    // @step And indentation anchor_line is None
    // @step And indentation max_levels is None
    assert_eq!(
        result,
        InternalFileParams::Read {
            file_path: "/src/main.rs".to_string(),
            offset: None,
            limit: None,
            mode: Some("indentation".to_string()),
            indentation: Some(InternalIndentationParams {
                anchor_line: None,
                max_levels: None,
                include_siblings: Some(true),
                include_header: Some(true),
                max_lines: Some(100),
            }),
        }
    );
}
