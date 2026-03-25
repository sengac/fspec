// Feature: spec/features/calls-imports-edges-ruby.feature
//
// Integration tests for Ruby Imports and Calls edge extraction.
// Uses real Ruby source code fixtures and verifies the extracted graph entities.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashSet;

use codelet_napi::graph::ast_pipeline::ast_ruby_extractor::extract_ruby;

mod graph_test_helpers;
use graph_test_helpers::{build_known_files, find_edges, write_test_file};

// ============================================================================
// Scenario: Extract Imports edges from Ruby require_relative statements
// ============================================================================
#[test]
fn test_ruby_extract_imports_from_require_relative() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a Ruby file with `require_relative 'helpers/validator'`
    let ruby_source = r#"
require_relative 'helpers/validator'
require 'json'

def process(data)
  validate(data)
end
"#;
    write_test_file(project_dir, "lib/app.rb", ruby_source);

    // @step And the target file `lib/helpers/validator.rb` exists in the project
    let target_source = r#"
def validate(data)
  data.is_a?(Hash)
end
"#;
    write_test_file(project_dir, "lib/helpers/validator.rb", target_source);

    let known_files = build_known_files(project_dir);

    // @step When the Ruby extractor processes the source file
    let entities = extract_ruby(ruby_source, "lib/app.rb", &known_files)
        .expect("Ruby extraction should succeed");

    // @step Then an Imports edge should be emitted from the source file to the resolved target
    let imports = find_edges(&entities, "Imports", Some("lib-app-rb"), Some("validator"));
    assert!(
        !imports.is_empty(),
        "Should have Imports edge to helpers/validator.rb. All Imports: {:?}",
        find_edges(&entities, "Imports", None, None)
    );

    // @step And external `require 'json'` should NOT produce an edge
    let json_imports = find_edges(&entities, "Imports", None, Some("json"));
    assert!(
        json_imports.is_empty(),
        "External 'json' require should NOT produce edges"
    );
}

// ============================================================================
// Scenario: Extract Calls edges from Ruby method calls
// ============================================================================
#[test]
fn test_ruby_extract_calls_from_method_calls() {
    let ruby_source = r#"
def process(data)
  validate(data)
end

def validate(data)
  data.is_a?(Hash)
end
"#;
    let known_files = HashSet::new();

    // @step Given a Ruby file with `def process(data)` that calls `validate(data)`
    // @step And `validate` is defined in the same file
    // @step When the Ruby extractor processes the source file
    let entities = extract_ruby(ruby_source, "app.rb", &known_files)
        .expect("Ruby extraction should succeed");

    // @step Then a Calls edge should be emitted from `process` to `validate`
    let calls = find_edges(&entities, "Calls", Some("process"), Some("validate"));
    assert!(
        !calls.is_empty(),
        "Should have Calls edge from process to validate. All Calls: {:?}",
        find_edges(&entities, "Calls", None, None)
    );
}
