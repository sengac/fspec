//! Feature: spec/features/codex-list-dir-pagination.feature
//!
//! BUG-110 tests, moved out of codex.rs (COV-075: one test file per feature).

use super::*;

// =========================================================================
// BUG-110: list_dir offset, limit, and depth mapping tests
// Feature: spec/features/codex-list-dir-pagination.feature
// =========================================================================

/// Scenario: CodexListDirFacade maps offset and limit to InternalLsParams
#[test]
fn test_codex_list_dir_maps_offset_and_limit() {
    // @step Given a CodexListDirFacade instance
    let facade = CodexListDirFacade;

    // @step When the Codex model calls list_dir with dir_path "/src" offset 5 and limit 10
    let input = json!({
        "dir_path": "/src",
        "offset": 5,
        "limit": 10
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalLsParams::List with path "/src" offset 5 and limit 10
    assert_eq!(
        result,
        InternalLsParams::List {
            path: Some("/src".to_string()),
            offset: Some(5),
            limit: Some(10),
            depth: None,
        }
    );

    // @step Then depth is None
    let InternalLsParams::List { depth, .. } = &result;
    assert_eq!(*depth, None);
}

/// Scenario: CodexListDirFacade backward compatible without optional params
#[test]
fn test_codex_list_dir_backward_compatible_no_pagination() {
    // @step Given a CodexListDirFacade instance
    let facade = CodexListDirFacade;

    // @step When the Codex model calls list_dir with only dir_path "/src"
    let input = json!({
        "dir_path": "/src"
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalLsParams::List with offset None limit None and depth None
    assert_eq!(
        result,
        InternalLsParams::List {
            path: Some("/src".to_string()),
            offset: None,
            limit: None,
            depth: None,
        }
    );
}

/// Scenario: CodexListDirFacade schema includes offset limit and depth properties
#[test]
fn test_codex_list_dir_schema_has_pagination_params() {
    // @step Given a CodexListDirFacade instance
    let facade = CodexListDirFacade;

    // @step When the tool definition schema is inspected
    let def = facade.definition();

    // @step Then the schema has an "offset" property of type "integer"
    assert_eq!(def.parameters["properties"]["offset"]["type"], "integer");

    // @step Then the schema has a "limit" property of type "integer"
    assert_eq!(def.parameters["properties"]["limit"]["type"], "integer");

    // @step Then the schema has a "depth" property of type "integer"
    assert_eq!(def.parameters["properties"]["depth"]["type"], "integer");

    // @step Then only "dir_path" is in the required array
    assert_eq!(def.parameters["required"], json!(["dir_path"]));
}

/// Scenario: CodexListDirFacade maps depth to InternalLsParams
#[test]
fn test_codex_list_dir_maps_depth() {
    // @step Given a CodexListDirFacade instance
    let facade = CodexListDirFacade;

    // @step When the Codex model calls list_dir with dir_path "/src" and depth 3
    let input = json!({
        "dir_path": "/src",
        "depth": 3
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalLsParams::List with path "/src" and depth 3
    assert_eq!(
        result,
        InternalLsParams::List {
            path: Some("/src".to_string()),
            offset: None,
            limit: None,
            depth: Some(3),
        }
    );

    // @step Then offset is None and limit is None
    let InternalLsParams::List { offset, limit, .. } = &result;
    assert_eq!(*offset, None);
    assert_eq!(*limit, None);
}

/// Scenario: Other facades provide None for offset limit and depth
#[test]
fn test_zai_facade_provides_none_for_pagination_params() {
    use crate::facade::zai::ZAIListDirFacade;

    // @step Given a ZAIListDirFacade instance
    let facade = ZAIListDirFacade;

    // @step When the ZAI model calls list_dir with path "/src"
    let input = json!({
        "path": "/src"
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalLsParams::List with offset None limit None and depth None
    assert_eq!(
        result,
        InternalLsParams::List {
            path: Some("/src".to_string()),
            offset: None,
            limit: None,
            depth: None,
        }
    );
}
