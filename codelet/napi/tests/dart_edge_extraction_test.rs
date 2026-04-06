// Feature: spec/features/dart-ast-index-support.feature
//
// Integration tests for Dart AST extraction: File, Function, Type nodes,
// plus Imports, Calls, and TypeRef edges. Also tests pubspec.yaml dependency
// extraction and integration with walk_and_extract dispatch.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashSet;

use codelet_napi::graph::ast_pipeline::ast_dart_extractor::extract_dart;
use codelet_napi::graph::ast_pipeline::pubspec_dep_extractor::extract_pubspec_dependencies;
use codelet_napi::graph::graph_entities::GraphEntity;

mod graph_test_helpers;
use graph_test_helpers::{build_known_files, count_edges, count_nodes, find_edges, find_node, get_node_property, write_test_file};

// ============================================================================
// Scenario: Extract File, Function, and Type nodes from Dart source with Contains edges
// ============================================================================
#[test]
fn test_dart_extract_file_function_type_nodes() {
    // @step Given a Dart source file with classes, top-level functions, enums, and mixins
    let dart_source = r#"
import 'dart:math';

void main() {
  greet('World');
}

String greet(String name) {
  return 'Hello, $name!';
}

class Animal {
  String name;
  Animal(this.name);
  void speak() {}
}

enum Color { red, green, blue }

mixin Musical {
  bool canPlayPiano = false;
  void entertainMe() {}
}
"#;
    let known_files = HashSet::new();

    // @step When I run the Dart AST extractor on the file
    let entities = extract_dart(dart_source, "lib/main.dart", &known_files)
        .expect("Dart extraction should succeed");

    // @step Then it should produce a File node with language "dart"
    let file_node = find_node(&entities, "File", "lib-main-dart");
    assert!(file_node.is_some(), "Should have a File node");
    assert_eq!(
        get_node_property(file_node.unwrap(), "language"),
        Some("dart")
    );

    // @step And it should produce Function nodes for each top-level function and class method
    let fn_count = count_nodes(&entities, "Function");
    // Expected: main, greet, Animal (constructor), speak, entertainMe
    assert!(
        fn_count >= 4,
        "Should have at least 4 Function nodes (main, greet, speak, entertainMe), got {fn_count}"
    );

    // @step And it should produce Type nodes for each class, enum, and mixin
    let type_count = count_nodes(&entities, "Type");
    // Expected: Animal, Color, Musical
    assert!(
        type_count >= 3,
        "Should have at least 3 Type nodes (Animal, Color, Musical), got {type_count}"
    );

    // @step And it should produce Contains edges from the File to each Function
    let contains_count = count_edges(&entities, "Contains");
    assert!(
        contains_count >= 4,
        "Should have at least 4 Contains edges, got {contains_count}"
    );

    // @step And it should produce ContainsType edges from the File to each Type
    let contains_type_count = count_edges(&entities, "ContainsType");
    assert!(
        contains_type_count >= 3,
        "Should have at least 3 ContainsType edges, got {contains_type_count}"
    );
}

// ============================================================================
// Scenario: Extract relative imports as Imports edges
// ============================================================================
#[test]
fn test_dart_extract_relative_imports() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a Dart source file with "import '../models/user.dart'"
    let dart_source = r#"
import '../models/user.dart';
import 'dart:math';

void loadUser() {
  return;
}
"#;
    write_test_file(project_dir, "lib/screens/home.dart", dart_source);

    // @step And the target file "models/user.dart" exists in the known files set
    let target_source = r#"
class User {
  String name;
  User(this.name);
}
"#;
    write_test_file(project_dir, "lib/models/user.dart", target_source);

    let known_files = build_known_files(project_dir);

    // @step When I run the Dart AST extractor on the file
    let entities = extract_dart(dart_source, "lib/screens/home.dart", &known_files)
        .expect("Dart extraction should succeed");

    // @step Then it should produce an Imports edge from the source file to "models/user.dart"
    let imports = find_edges(&entities, "Imports", Some("home"), Some("user"));
    assert!(
        !imports.is_empty(),
        "Should have Imports edge to user.dart. All Imports: {:?}",
        find_edges(&entities, "Imports", None, None)
    );

    // @step And it should produce a stub File node for the import target
    let stub_file_nodes: Vec<_> = entities.iter().filter(|e| {
        matches!(e, GraphEntity::Node { node_type, properties, .. }
            if node_type == "File" && properties.get("path").and_then(|v| v.as_str()).is_some_and(|p| p.contains("user.dart"))
        )
    }).collect();
    assert!(!stub_file_nodes.is_empty(), "Should create stub File node for import target");
}

// ============================================================================
// Scenario: Skip external dart: and package: imports
// ============================================================================
#[test]
fn test_dart_skip_external_imports() {
    // @step Given a Dart source file with "import 'dart:math'" and "import 'package:flutter/material.dart'"
    let dart_source = r#"
import 'dart:math';
import 'dart:async';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

void build() {
  return;
}
"#;
    let known_files = HashSet::new();

    // @step When I run the Dart AST extractor on the file
    let entities = extract_dart(dart_source, "lib/main.dart", &known_files)
        .expect("Dart extraction should succeed");

    // @step Then it should NOT produce any Imports edges for the external imports
    let imports = find_edges(&entities, "Imports", None, None);
    assert!(
        imports.is_empty(),
        "Should NOT have Imports edges for dart: and package: imports. Got: {:?}",
        imports
    );
}

// ============================================================================
// Scenario: Extract Calls edges from function bodies
// ============================================================================
#[test]
fn test_dart_extract_calls_from_function_bodies() {
    // @step Given a Dart source file with a function "processData" that calls local function "validateInput"
    let dart_source = r#"
void processData(String data) {
  validateInput(data);
  transform(data);
}

void validateInput(String input) {
  return;
}

void transform(String data) {
  return;
}
"#;
    let known_files = HashSet::new();

    // @step When I run the Dart AST extractor on the file
    let entities = extract_dart(dart_source, "lib/processor.dart", &known_files)
        .expect("Dart extraction should succeed");

    // @step Then it should produce a Calls edge from "processData" to "validateInput"
    let calls = find_edges(&entities, "Calls", Some("processData"), Some("validateInput"));
    assert!(
        !calls.is_empty(),
        "Should have Calls edge from processData to validateInput. All Calls: {:?}",
        find_edges(&entities, "Calls", None, None)
    );
}

// ============================================================================
// Scenario: Extract TypeRef edges from function signatures excluding builtins
// ============================================================================
#[test]
fn test_dart_extract_typerefs_excluding_builtins() {
    // @step Given a Dart source file with a function "createUser(UserModel model, String name)"
    let dart_source = r#"
class UserModel {
  String name;
  UserModel(this.name);
}

UserModel createUser(UserModel model, String name) {
  return model;
}
"#;
    let known_files = HashSet::new();

    // @step And "UserModel" is defined as a class in the same file
    // (already included in the source above)

    // @step When I run the Dart AST extractor on the file
    let entities = extract_dart(dart_source, "lib/user_service.dart", &known_files)
        .expect("Dart extraction should succeed");

    // @step Then it should produce a TypeRef edge from "createUser" to "UserModel"
    let type_refs = find_edges(&entities, "TypeRef", Some("createUser"), Some("UserModel"));
    assert!(
        !type_refs.is_empty(),
        "Should have TypeRef from createUser to UserModel. All TypeRef: {:?}",
        find_edges(&entities, "TypeRef", None, None)
    );

    // @step And it should NOT produce a TypeRef edge for the builtin type "String"
    let string_refs = find_edges(&entities, "TypeRef", Some("createUser"), Some("String"));
    assert!(
        string_refs.is_empty(),
        "Should NOT have TypeRef to builtin String type"
    );
}

// ============================================================================
// Scenario: Extract dependencies from pubspec.yaml
// ============================================================================
#[test]
fn test_dart_extract_pubspec_dependencies() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a pubspec.yaml file with "provider: ^6.0.0" in dependencies and "build_runner: ^2.4.0" in dev_dependencies
    let pubspec_content = r#"
name: my_app
version: 1.0.0

dependencies:
  flutter:
    sdk: flutter
  provider: ^6.0.0
  http: ^1.1.0

dev_dependencies:
  flutter_test:
    sdk: flutter
  build_runner: ^2.4.0
"#;
    write_test_file(project_dir, "pubspec.yaml", pubspec_content);

    // @step When I run the pubspec dependency extractor
    let entities = extract_pubspec_dependencies(project_dir)
        .expect("Pubspec extraction should succeed");

    // @step Then it should produce a Dependency node for "provider" with isDev false and version "^6.0.0"
    let provider_dep = entities.iter().find(|e| {
        if let GraphEntity::Node { node_type, properties, .. } = e {
            node_type == "Dependency"
                && properties.get("name").and_then(|v| v.as_str()) == Some("provider")
        } else {
            false
        }
    });
    assert!(provider_dep.is_some(), "Should have Dependency node for 'provider'");
    if let Some(GraphEntity::Node { properties, .. }) = provider_dep {
        assert_eq!(properties.get("isDev").and_then(|v| v.as_bool()), Some(false));
        assert_eq!(properties.get("version").and_then(|v| v.as_str()), Some("^6.0.0"));
    }

    // @step And it should produce a Dependency node for "build_runner" with isDev true and version "^2.4.0"
    let build_runner_dep = entities.iter().find(|e| {
        if let GraphEntity::Node { node_type, properties, .. } = e {
            node_type == "Dependency"
                && properties.get("name").and_then(|v| v.as_str()) == Some("build_runner")
        } else {
            false
        }
    });
    assert!(build_runner_dep.is_some(), "Should have Dependency node for 'build_runner'");
    if let Some(GraphEntity::Node { properties, .. }) = build_runner_dep {
        assert_eq!(properties.get("isDev").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(properties.get("version").and_then(|v| v.as_str()), Some("^2.4.0"));
    }

    // @step And it should produce DependsOn edges from the pubspec.yaml File to each Dependency
    let depends_on_count = count_edges(&entities, "DependsOn");
    assert!(
        depends_on_count >= 2,
        "Should have at least 2 DependsOn edges (provider, build_runner), got {depends_on_count}"
    );
}

// ============================================================================
// Scenario: Extract named constructors and factory constructors as Function nodes
// ============================================================================
#[test]
fn test_dart_extract_constructors_as_functions() {
    // @step Given a Dart class with constructor "User(this.name)", named constructor "User.fromJson", and factory constructor "factory User.create"
    let dart_source = r#"
class User {
  String name;
  int age;

  User(this.name, this.age);

  User.fromJson(Map<String, dynamic> json)
      : name = json['name'] as String,
        age = json['age'] as int;

  factory User.create(String name) {
    return User(name, 0);
  }
}
"#;
    let known_files = HashSet::new();

    // @step When I run the Dart AST extractor on the file
    let entities = extract_dart(dart_source, "lib/models/user.dart", &known_files)
        .expect("Dart extraction should succeed");

    // @step Then it should produce Function nodes for "User", "fromJson", and "create"
    let fn_nodes: Vec<_> = entities
        .iter()
        .filter_map(|e| {
            if let GraphEntity::Node { node_type, properties, .. } = e {
                if node_type == "Function" {
                    return properties.get("name").and_then(|v| v.as_str());
                }
            }
            None
        })
        .collect();

    println!("Function names found: {:?}", fn_nodes);
    assert!(
        fn_nodes.contains(&"User"),
        "Should have Function node for constructor 'User'. Got: {:?}",
        fn_nodes
    );
    assert!(
        fn_nodes.contains(&"fromJson"),
        "Should have Function node for named constructor 'fromJson'. Got: {:?}",
        fn_nodes
    );
    assert!(
        fn_nodes.contains(&"create"),
        "Should have Function node for factory constructor 'create'. Got: {:?}",
        fn_nodes
    );
}

// ============================================================================
// Scenario: Dart files are included in SUPPORTED_EXTENSIONS and dispatched correctly
// ============================================================================
#[test]
fn test_dart_files_dispatched_in_walk_and_extract() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a project directory containing ".dart" files
    let dart_source = r#"
void main() {
  print('Hello');
}
"#;
    write_test_file(project_dir, "lib/main.dart", dart_source);

    // @step When I run walk_and_extract on the project directory
    let entities = codelet_napi::graph::ast_pipeline::walk_and_extract(project_dir, false)
        .expect("walk_and_extract should succeed");

    // @step Then the ".dart" files should be discovered and extracted using the Dart extractor
    let dart_file = entities.iter().find(|e| {
        if let GraphEntity::Node { node_type, properties, .. } = e {
            node_type == "File"
                && properties.get("language").and_then(|v| v.as_str()) == Some("dart")
        } else {
            false
        }
    });
    assert!(
        dart_file.is_some(),
        "Should discover and extract .dart files. Entities: {:?}",
        entities.iter().filter(|e| matches!(e, GraphEntity::Node { node_type, .. } if node_type == "File")).collect::<Vec<_>>()
    );
}

// ============================================================================
// Scenario: Identify uncalled Dart functions and unimported files as dead code
// ============================================================================
#[tokio::test]
async fn test_dart_dead_code_detection() {
    use codelet_napi::graph::ast_pipeline::walk_and_extract;
    use codelet_napi::graph::database::GraphDatabase;

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    /// The AST code schema for loading extracted entities.
    const AST_CODE_SCHEMA: &str = include_str!("../schemas/ast-code.pg");

    // @step Given a Dart project with function "unusedHelper" that is never called and file "lib/orphan.dart" that is never imported
    let main_source = r#"
import 'utils.dart';

void main() {
  greet('World');
}
"#;
    write_test_file(project_dir, "lib/main.dart", main_source);

    let utils_source = r#"
String greet(String name) {
  return 'Hello, $name!';
}

void unusedHelper() {
  return;
}
"#;
    write_test_file(project_dir, "lib/utils.dart", utils_source);

    let orphan_source = r#"
void orphanFunction() {
  return;
}
"#;
    write_test_file(project_dir, "lib/orphan.dart", orphan_source);
    write_test_file(project_dir, ".gitignore", "build/\n.dart_tool/\n");

    // @step When I run ast_dead_code detection on the indexed project
    let entities = walk_and_extract(project_dir, false)
        .expect("walk_and_extract should succeed");

    let db_path = temp_dir.path().join("test-dart-dead-code.nano");
    let db = GraphDatabase::init(&db_path, AST_CODE_SCHEMA)
        .await
        .expect("DB init should succeed");
    db.load_entities(&entities)
        .await
        .expect("load should succeed");

    // Query for uncalled functions
    let uncalled_query = r#"
query uncalled_functions() {
    match {
        $fn: Function
        not { $caller calls $fn }
    }
    return { $fn.slug, $fn.name, $fn.isPublic }
}
"#;
    let db_fn = db.clone().with_query_source(uncalled_query);
    let result = db_fn
        .query("uncalled_functions", None)
        .await
        .expect("query should succeed");
    let uncalled = result.as_array().expect("should be array");
    let uncalled_names: Vec<&str> = uncalled
        .iter()
        .filter_map(|f| f.get("name").and_then(|v| v.as_str()))
        .collect();

    // @step Then "unusedHelper" should appear in the dead functions list
    assert!(
        uncalled_names.contains(&"unusedHelper"),
        "Uncalled functions should include 'unusedHelper', got: {:?}",
        uncalled_names
    );

    // Query for orphan files
    let orphan_query = r#"
query orphan_files() {
    match {
        $f: File
        not { $other imports $f }
    }
    return { $f.slug, $f.path, $f.language, $f.isTest }
}
"#;
    let db_file = db.with_query_source(orphan_query);
    let result = db_file
        .query("orphan_files", None)
        .await
        .expect("query should succeed");
    let orphans = result.as_array().expect("should be array");
    let orphan_paths: Vec<&str> = orphans
        .iter()
        .filter_map(|o| {
            let is_test = o.get("isTest").and_then(|v| v.as_bool()).unwrap_or(false);
            let has_language = o.get("language").and_then(|v| v.as_str()).is_some();
            if !is_test && has_language {
                o.get("path").and_then(|v| v.as_str())
            } else {
                None
            }
        })
        .collect();

    // @step And "lib/orphan.dart" should appear in the dead files list
    assert!(
        orphan_paths.iter().any(|p| p.contains("orphan.dart")),
        "Orphan files should include 'lib/orphan.dart', got: {:?}",
        orphan_paths
    );
}
