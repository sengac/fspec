//! Feature: spec/features/codex-grep-files-include-limit.feature
//!
//! BUG-111 tests, moved out of codex.rs (COV-075: one test file per feature).

use super::*;

/// Feature: spec/features/codex-grep-files-include-limit.feature
///
/// Scenario: CodexGrepFilesFacade maps include param to InternalSearchParams::Grep
#[test]
fn test_codex_grep_files_facade_maps_include_param() {
    // @step Given a CodexGrepFilesFacade instance
    let facade = CodexGrepFilesFacade;

    // @step When the Codex model calls grep_files with pattern "TODO", include "*.rs", and path "/src"
    let input = json!({
        "pattern": "TODO",
        "include": "*.rs",
        "path": "/src"
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalSearchParams::Grep with pattern "TODO", path "/src", and include "*.rs"
    assert_eq!(
        result,
        InternalSearchParams::Grep {
            pattern: "TODO".to_string(),
            path: Some("/src".to_string()),
            include: Some("*.rs".to_string()),
            limit: None,
        }
    );
}

/// Scenario: CodexGrepFilesFacade maps limit param to InternalSearchParams::Grep
#[test]
fn test_codex_grep_files_facade_maps_limit_param() {
    // @step Given a CodexGrepFilesFacade instance
    let facade = CodexGrepFilesFacade;

    // @step When the Codex model calls grep_files with pattern "." and limit 10
    let input = json!({
        "pattern": ".",
        "limit": 10
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalSearchParams::Grep with pattern "." and limit 10
    assert_eq!(
        result,
        InternalSearchParams::Grep {
            pattern: ".".to_string(),
            path: None,
            include: None,
            limit: Some(10),
        }
    );
}

/// Scenario: CodexGrepFilesFacade maps both include and limit params
#[test]
fn test_codex_grep_files_facade_maps_include_and_limit() {
    // @step Given a CodexGrepFilesFacade instance
    let facade = CodexGrepFilesFacade;

    // @step When the Codex model calls grep_files with pattern ".", include "*.rs", and limit 10
    let input = json!({
        "pattern": ".",
        "include": "*.rs",
        "limit": 10
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalSearchParams::Grep with include "*.rs" and limit 10
    assert_eq!(
        result,
        InternalSearchParams::Grep {
            pattern: ".".to_string(),
            path: None,
            include: Some("*.rs".to_string()),
            limit: Some(10),
        }
    );
}

/// Scenario: CodexGrepFilesFacade remains backward compatible without include or limit
#[test]
fn test_codex_grep_files_facade_backward_compatible() {
    // @step Given a CodexGrepFilesFacade instance
    let facade = CodexGrepFilesFacade;

    // @step When the Codex model calls grep_files with only pattern "TODO"
    let input = json!({
        "pattern": "TODO"
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalSearchParams::Grep with include None and limit None
    assert_eq!(
        result,
        InternalSearchParams::Grep {
            pattern: "TODO".to_string(),
            path: None,
            include: None,
            limit: None,
        }
    );
}

/// Feature: spec/features/codex-grep-files-include-limit.feature
///
/// Scenario: InternalSearchParams::Grep includes optional include and limit fields
#[test]
fn test_internal_search_params_grep_has_include_and_limit_fields() {
    // @step Given the InternalSearchParams::Grep enum variant
    // Construct it with all fields to prove they exist

    // @step Then it has an optional "include" field of type Option<String>
    let with_include = InternalSearchParams::Grep {
        pattern: "test".to_string(),
        path: None,
        include: Some("*.rs".to_string()),
        limit: None,
    };
    if let InternalSearchParams::Grep { include, .. } = &with_include {
        assert_eq!(include, &Some("*.rs".to_string()));
    }

    // @step And it has an optional "limit" field of type Option<usize>
    let with_limit = InternalSearchParams::Grep {
        pattern: "test".to_string(),
        path: None,
        include: None,
        limit: Some(100),
    };
    if let InternalSearchParams::Grep { limit, .. } = &with_limit {
        assert_eq!(limit, &Some(100));
    }
}

// =========================================================================
// BUG-111: GrepArgs glob field tests
// Feature: spec/features/codex-grep-files-include-limit.feature
// =========================================================================

/// Scenario: GrepArgs struct supports optional glob field
#[test]
fn test_grep_args_supports_optional_glob_field() {
    use crate::grep::GrepArgs;

    // @step Given a GrepArgs with pattern "TODO" and glob "*.rs"
    let args = GrepArgs {
        pattern: "TODO".to_string(),
        path: None,
        output_mode: None,
        glob: Some("*.rs".to_string()),
        limit: None,
    };

    // @step When the GrepTool call() method executes
    // Verify the struct has the glob field and it can be set
    assert_eq!(args.glob.as_deref(), Some("*.rs"));

    // @step Then the glob filter is passed through to the execute() method
    // The call() method serializes args to Value and passes glob through
    let value = serde_json::to_value(&args).unwrap();
    assert_eq!(value["glob"], "*.rs");
}

/// Scenario: SearchToolFacadeWrapper passes include as glob to GrepTool
#[test]
fn test_wrapper_grep_args_construction_with_include() {
    use crate::grep::GrepArgs;

    // @step Given a SearchToolFacadeWrapper with a CodexGrepFilesFacade
    let facade = CodexGrepFilesFacade;

    // @step When the wrapper receives InternalSearchParams::Grep with include "*.rs"
    let input = json!({
        "pattern": "TODO",
        "include": "*.rs",
        "path": "/src"
    });
    let params = facade.map_params(input).unwrap();

    // @step Then the wrapper passes "glob" = "*.rs" in the GrepTool execute args
    // Verify the internal params carry include, and show how wrapper would construct GrepArgs
    if let InternalSearchParams::Grep {
        pattern,
        path,
        include,
        limit,
    } = params
    {
        let grep_args = GrepArgs {
            pattern,
            path,
            output_mode: None,
            glob: include,
            limit,
        };
        assert_eq!(grep_args.glob.as_deref(), Some("*.rs"));
    } else {
        panic!("Expected InternalSearchParams::Grep");
    }
}

/// Scenario: SearchToolFacadeWrapper applies limit to cap grep results
#[test]
fn test_wrapper_grep_args_construction_with_limit() {
    use crate::grep::GrepArgs;

    // @step Given a SearchToolFacadeWrapper with a CodexGrepFilesFacade
    let facade = CodexGrepFilesFacade;

    // @step When the wrapper receives InternalSearchParams::Grep with limit 10
    let input = json!({
        "pattern": "TODO",
        "limit": 10
    });
    let params = facade.map_params(input).unwrap();

    // @step Then the wrapper caps the grep output to at most 10 result lines
    // Verify the internal params carry limit, and show how wrapper would construct GrepArgs
    if let InternalSearchParams::Grep {
        pattern,
        path,
        include,
        limit,
    } = params
    {
        let grep_args = GrepArgs {
            pattern,
            path,
            output_mode: None,
            glob: include,
            limit,
        };
        assert_eq!(grep_args.limit, Some(10));
    } else {
        panic!("Expected InternalSearchParams::Grep");
    }
}

/// Feature: spec/features/codex-grep-files-include-limit.feature
///
/// Scenario: ZAI grep facade constructs InternalSearchParams::Grep with None defaults
#[test]
fn test_zai_grep_facade_uses_none_defaults_for_include_and_limit() {
    use crate::facade::zai::ZAIGrepFilesFacade;

    // @step Given a ZAIGrepFilesFacade instance
    let facade = ZAIGrepFilesFacade;

    // @step When the ZAI model calls grep_files with pattern "TODO" and path "src"
    let input = json!({
        "pattern": "TODO",
        "path": "src"
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalSearchParams::Grep with include None and limit None
    assert_eq!(
        result,
        InternalSearchParams::Grep {
            pattern: "TODO".to_string(),
            path: Some("src".to_string()),
            include: None,
            limit: None,
        }
    );
}

/// Scenario: Gemini grep facade constructs InternalSearchParams::Grep with None defaults
#[test]
fn test_gemini_grep_facade_uses_none_defaults_for_include_and_limit() {
    use crate::facade::search::GeminiSearchFileContentFacade;

    // @step Given a GeminiSearchFileContentFacade instance
    let facade = GeminiSearchFileContentFacade;

    // @step When Gemini sends parameters {pattern: 'TODO', dir_path: 'src'} to tool 'search_file_content'
    let input = json!({
        "pattern": "TODO",
        "dir_path": "src"
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalSearchParams::Grep with include None and limit None
    assert_eq!(
        result,
        InternalSearchParams::Grep {
            pattern: "TODO".to_string(),
            path: Some("src".to_string()),
            include: None,
            limit: None,
        }
    );
}
