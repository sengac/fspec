// Feature: spec/features/ast-index-class-import-crash.feature
//
// Tests for KGRAPH-055: Python and Java ast_index crashes on real repos.
// Root causes:
// 1. resolve_calls first_char.is_uppercase() heuristic fails for underscore-prefixed class names
// 2. extract_name_after_keyword picks up keyword from comments before actual declaration
// 3. deduplicate_entities edge pruning doesn't check node type matches edge schema

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashSet;

use codelet_napi::graph::ast_pipeline::ast_java_extractor::extract_java;
use codelet_napi::graph::ast_pipeline::ast_python_extractor::extract_python;
use codelet_napi::graph::ast_pipeline::deduplicate_entities;
use codelet_napi::graph::graph_entities::GraphEntity;

mod graph_test_helpers;
use graph_test_helpers::{build_known_files, find_edges, write_test_file};

/// Find nodes by type and name substring.
fn find_nodes_by_name<'a>(
    entities: &'a [GraphEntity],
    node_type: &str,
    name_contains: &str,
) -> Vec<&'a GraphEntity> {
    entities
        .iter()
        .filter(|e| match e {
            GraphEntity::Node {
                node_type: nt,
                properties,
                ..
            } => {
                nt == node_type
                    && properties
                        .get("name")
                        .and_then(|v| v.as_str())
                        .is_some_and(|n| n.contains(name_contains))
            }
            _ => false,
        })
        .collect()
}

/// Build a Calls edge entity for testing dedup.
fn make_calls_edge(from_slug: &str, to_slug: &str) -> GraphEntity {
    GraphEntity::Edge {
        edge_type: "Calls".to_string(),
        from_slug: from_slug.to_string(),
        to_slug: to_slug.to_string(),
        properties: serde_json::Map::new(),
    }
}

/// Build a Function node entity for testing dedup.
fn make_function_node(slug: &str) -> GraphEntity {
    let mut props = serde_json::Map::new();
    props.insert("slug".to_string(), serde_json::Value::String(slug.to_string()));
    props.insert("name".to_string(), serde_json::Value::String("test".to_string()));
    props.insert("isAsync".to_string(), serde_json::Value::Bool(false));
    props.insert("isPublic".to_string(), serde_json::Value::Bool(true));
    props.insert("paramCount".to_string(), serde_json::Value::Number(0.into()));
    props.insert("lineStart".to_string(), serde_json::Value::Number(1.into()));
    props.insert("lineEnd".to_string(), serde_json::Value::Number(5.into()));
    GraphEntity::Node {
        node_type: "Function".to_string(),
        slug: slug.to_string(),
        properties: props,
    }
}

/// Build a Type node entity for testing dedup.
fn make_type_node(slug: &str) -> GraphEntity {
    let mut props = serde_json::Map::new();
    props.insert("slug".to_string(), serde_json::Value::String(slug.to_string()));
    props.insert("name".to_string(), serde_json::Value::String("TestClass".to_string()));
    props.insert("typeKind".to_string(), serde_json::Value::String("class".to_string()));
    props.insert("isPublic".to_string(), serde_json::Value::Bool(true));
    GraphEntity::Node {
        node_type: "Type".to_string(),
        slug: slug.to_string(),
        properties: props,
    }
}

// ============================================================================
// Scenario: Python underscore-prefixed class imported and called as constructor
//           produces TypeRef
// ============================================================================

#[test]
fn test_python_underscore_class_import_produces_typeref() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a Python file "src/app/parser.py" defining class "_OptionParser" and function "split_args"
    let parser_source = r#"
class _OptionParser:
    def __init__(self, ctx):
        self.ctx = ctx

def split_args(text):
    return text.split()
"#;
    write_test_file(project_dir, "src/app/parser.py", parser_source);

    // @step And a Python file "src/app/core.py" with "from app.parser import _OptionParser, split_args"
    let core_source = r#"
from app.parser import _OptionParser, split_args

def make_parser(ctx):
    parser = _OptionParser(ctx)
    args = split_args("hello world")
    return parser
"#;
    write_test_file(project_dir, "src/app/core.py", core_source);

    // @step And "src/app/core.py" has function "make_parser" that calls "_OptionParser(ctx)" and "split_args()"
    let known_files = build_known_files(project_dir);

    // @step When I extract entities from both files with known_files containing both paths
    let entities = extract_python(core_source, "src/app/core.py", &known_files)
        .expect("Python extraction should not crash");

    // @step Then a TypeRef edge should exist from "make_parser" to "_OptionParser"
    let typeref_edges = find_edges(&entities, "TypeRef", Some("make_parser"), Some("_OptionParser"));
    assert!(
        !typeref_edges.is_empty(),
        "Should have TypeRef edge from make_parser to _OptionParser (underscore-prefixed class). \
         All TypeRef: {:?}, All Calls: {:?}",
        find_edges(&entities, "TypeRef", None, None),
        find_edges(&entities, "Calls", None, None)
    );

    // @step And a Calls edge should exist from "make_parser" to "split_args"
    let calls_edges = find_edges(&entities, "Calls", Some("make_parser"), Some("split_args"));
    assert!(
        !calls_edges.is_empty(),
        "Should have Calls edge from make_parser to split_args. All Calls: {:?}",
        find_edges(&entities, "Calls", None, None)
    );

    // @step And no Calls edge should target "_OptionParser"
    let bad_calls = find_edges(&entities, "Calls", None, Some("_OptionParser"));
    assert!(
        bad_calls.is_empty(),
        "Should NOT have Calls edge targeting _OptionParser (it's a Type, not Function). \
         Bad Calls: {:?}",
        bad_calls
    );
}

// ============================================================================
// Scenario: Java class with comment containing keyword before declaration
//           extracts correct name
// ============================================================================

#[test]
fn test_java_class_with_comment_keyword_extracts_correct_name() {
    // @step Given a Java file "com/myapp/MyException.java" with content:
    let java_source = r#"package com.myapp;
// This is a class for custom exceptions
@SuppressWarnings("MemberName") // class name is part of the public API
public final class MyException extends RuntimeException {
    public MyException(String msg) { super(msg); }
}
"#;

    let known_files = HashSet::new();

    // @step When I extract entities from the Java file
    let entities = extract_java(java_source, "com/myapp/MyException.java", &known_files)
        .expect("Java extraction should not crash");

    // @step Then a Type node should exist with name "MyException"
    let exception_types = find_nodes_by_name(&entities, "Type", "MyException");
    assert!(
        !exception_types.is_empty(),
        "Should have Type node for MyException. All Types: {:?}",
        entities
            .iter()
            .filter(|e| matches!(e, GraphEntity::Node { node_type, .. } if node_type == "Type"))
            .collect::<Vec<_>>()
    );

    // @step And no Type node should exist with name "name"
    let bad_name_types = find_nodes_by_name(&entities, "Type", "name");
    // Filter to exact match — "name" not just substring
    let exact_bad = bad_name_types
        .iter()
        .filter(|e| {
            if let GraphEntity::Node { properties, .. } = e {
                properties.get("name").and_then(|v| v.as_str()) == Some("name")
            } else {
                false
            }
        })
        .count();
    assert_eq!(
        exact_bad, 0,
        "Should NOT have Type node with name 'name' (extracted from comment). Types: {:?}",
        entities
            .iter()
            .filter(|e| matches!(e, GraphEntity::Node { node_type, .. } if node_type == "Type"))
            .collect::<Vec<_>>()
    );

    // @step And no Type node should exist with name "MemberName"
    let member_types = find_nodes_by_name(&entities, "Type", "MemberName");
    assert!(
        member_types.is_empty(),
        "Should NOT have Type node with name 'MemberName' (from annotation string)"
    );
}

// ============================================================================
// Scenario: Java imported class used in constructor produces TypeRef not crash
// ============================================================================

#[test]
fn test_java_imported_class_constructor_produces_typeref() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a Java file "com/myapp/MyException.java" defining class "MyException"
    let exception_source = r#"package com.myapp;
public class MyException extends RuntimeException {
    public MyException(String msg) { super(msg); }
}
"#;
    write_test_file(project_dir, "com/myapp/MyException.java", exception_source);

    // @step And a Java file "com/myapp/Service.java" with "import com.myapp.MyException;"
    let service_source = r#"package com.myapp;
import com.myapp.MyException;

public class Service {
    public void doWork() {
        throw new MyException("failed");
    }
}
"#;
    write_test_file(project_dir, "com/myapp/Service.java", service_source);

    let known_files = build_known_files(project_dir);

    // @step And "com/myapp/Service.java" has method "doWork" that calls "new MyException(msg)"
    // @step When I extract entities from both files with known_files containing both paths
    let entities = extract_java(service_source, "com/myapp/Service.java", &known_files)
        .expect("Java extraction should not crash");

    // @step Then a TypeRef edge should exist from "doWork" to "MyException"
    let typeref_edges = find_edges(&entities, "TypeRef", Some("doWork"), Some("MyException"));
    assert!(
        !typeref_edges.is_empty(),
        "Should have TypeRef edge from doWork to MyException. All TypeRef: {:?}, All Calls: {:?}",
        find_edges(&entities, "TypeRef", None, None),
        find_edges(&entities, "Calls", None, None)
    );

    // @step And no Calls edge should target "MyException" as a Function
    let bad_calls = find_edges(&entities, "Calls", None, Some("MyException"));
    assert!(
        bad_calls.is_empty(),
        "Should NOT have Calls edge targeting MyException. Bad Calls: {:?}",
        bad_calls
    );

    // @step And indexing should complete without errors
    // (verified by the expect() above not panicking)
}

// ============================================================================
// Scenario: Python mixed imports — function gets Calls edge, class gets TypeRef edge
// ============================================================================

#[test]
fn test_python_mixed_imports_function_calls_class_typeref() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a Python file "src/app/utils.py" defining function "parse_args" and class "Config"
    let utils_source = r#"
def parse_args(argv):
    return argv

class Config:
    def __init__(self):
        self.debug = False
"#;
    write_test_file(project_dir, "src/app/utils.py", utils_source);

    // @step And a Python file "src/app/main.py" with "from app.utils import parse_args, Config"
    let main_source = r#"
from app.utils import parse_args, Config

def run():
    args = parse_args(["--debug"])
    cfg = Config()
    return cfg
"#;
    write_test_file(project_dir, "src/app/main.py", main_source);

    let known_files = build_known_files(project_dir);

    // @step And "src/app/main.py" has function "run" that calls "parse_args()" and "Config()"
    // @step When I extract entities from both files with known_files containing both paths
    let entities = extract_python(main_source, "src/app/main.py", &known_files)
        .expect("Python extraction should not crash");

    // @step Then a Calls edge should exist from "run" to "parse_args"
    let calls_edges = find_edges(&entities, "Calls", Some("run"), Some("parse_args"));
    assert!(
        !calls_edges.is_empty(),
        "Should have Calls edge from run to parse_args. All Calls: {:?}",
        find_edges(&entities, "Calls", None, None)
    );

    // @step And a TypeRef edge should exist from "run" to "Config"
    let typeref_edges = find_edges(&entities, "TypeRef", Some("run"), Some("Config"));
    assert!(
        !typeref_edges.is_empty(),
        "Should have TypeRef edge from run to Config. All TypeRef: {:?}",
        find_edges(&entities, "TypeRef", None, None)
    );
}

// ============================================================================
// Scenario: Deduplicate prunes Calls edge targeting a Type-only slug
// ============================================================================

#[test]
fn test_deduplicate_prunes_calls_edge_to_type_only_slug() {
    // @step Given extracted entities containing a Type node with slug "file-a::MyClass"
    let type_node = make_type_node("file-a::MyClass");

    // @step And no Function node exists with slug "file-a::MyClass"
    // (we intentionally don't create one)

    // @step And a Calls edge from "file-b::caller" to "file-a::MyClass"
    let caller_fn = make_function_node("file-b::caller");
    let bad_calls_edge = make_calls_edge("file-b::caller", "file-a::MyClass");

    // Also need a File node for containment so the entities are valid
    let file_a = GraphEntity::Node {
        node_type: "File".to_string(),
        slug: "file-a".to_string(),
        properties: {
            let mut p = serde_json::Map::new();
            p.insert("slug".to_string(), serde_json::Value::String("file-a".to_string()));
            p.insert("path".to_string(), serde_json::Value::String("file-a.py".to_string()));
            p
        },
    };
    let file_b = GraphEntity::Node {
        node_type: "File".to_string(),
        slug: "file-b".to_string(),
        properties: {
            let mut p = serde_json::Map::new();
            p.insert("slug".to_string(), serde_json::Value::String("file-b".to_string()));
            p.insert("path".to_string(), serde_json::Value::String("file-b.py".to_string()));
            p
        },
    };

    let entities = vec![file_a, file_b, type_node, caller_fn, bad_calls_edge];

    // @step When I run deduplicate_entities on the entity list
    let deduped = deduplicate_entities(entities);

    // @step Then the Calls edge from "file-b::caller" to "file-a::MyClass" should be pruned
    let remaining_calls = find_edges(&deduped, "Calls", Some("file-b::caller"), Some("file-a::MyClass"));

    // @step And no Calls edge should remain targeting "file-a::MyClass"
    assert!(
        remaining_calls.is_empty(),
        "Calls edge targeting a Type-only slug should be pruned by deduplicate_entities. \
         Remaining Calls: {:?}",
        remaining_calls
    );
}

// ============================================================================
// Scenario: Python stdlib import does not create edges or crash
// ============================================================================

#[test]
fn test_python_stdlib_import_no_edges() {
    // @step Given a Python file "src/app/core.py" with "from os import path"
    let source = r#"
from os import path

def resolve(a, b):
    return path.join(a, b)
"#;

    // @step And "src/app/core.py" has function "resolve" that calls "path.join(a, b)"
    // @step When I extract entities with known_files NOT containing any "os" path
    let known_files = HashSet::new();
    let entities = extract_python(source, "src/app/core.py", &known_files)
        .expect("Python extraction should not crash on stdlib imports");

    // @step Then no Imports edge should exist for the "os" import
    let import_edges = find_edges(&entities, "Imports", None, None);
    assert!(
        import_edges.is_empty(),
        "stdlib imports (os) should NOT produce Imports edges. Found: {:?}",
        import_edges
    );

    // @step And indexing should complete without errors
    // (verified by the expect() above not panicking)
}
