// Feature: spec/features/dart-ast-index-support.feature
use super::*;

// ============================================================================
// Scenario: Extract File, Function, and Type nodes from Dart source with Contains edges
// ============================================================================
#[test]
fn test_dart_extract_file_function_type_nodes() -> TestResult {
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

    // @step When I run the Dart AST extractor on the file
    let (_directory, _root, entities) = indexed(&[("lib/main.dart", dart_source)])?;

    // @step Then it should produce a File node with language "dart"
    let files = nodes(&entities, "File");
    assert_eq!(files.len(), 1, "Should have one File node: {files:?}");
    assert_eq!(files[0].0, "lib/main.dart");
    assert_eq!(
        files[0].1.get("language").and_then(Value::as_str),
        Some("dart")
    );

    // @step And it should produce Function nodes for each top-level function and class method
    // main, greet, Animal (constructor), speak, entertainMe
    assert_eq!(
        names(&entities, "Function"),
        vec!["Animal", "entertainMe", "greet", "main", "speak"]
    );

    // @step And it should produce Type nodes for each class, enum, and mixin
    assert_eq!(names(&entities, "Type"), vec!["Animal", "Color", "Musical"]);

    // @step And it should produce Contains edges from the File to each Function
    let contains_count = count_edges(&entities, "Contains");
    assert_eq!(contains_count, 5, "one Contains edge per Function");
    assert!(edges(&entities, "Contains")
        .iter()
        .all(|(from, _, _)| from == "lib/main.dart"));

    // @step And it should produce ContainsType edges from the File to each Type
    let contains_type_count = count_edges(&entities, "ContainsType");
    assert_eq!(contains_type_count, 3, "one ContainsType edge per Type");
    assert!(edges(&entities, "ContainsType")
        .iter()
        .all(|(from, _, _)| from == "lib/main.dart"));
    Ok(())
}

// ============================================================================
// Scenario: Extract relative imports as Imports edges
// ============================================================================
#[test]
fn test_dart_extract_relative_imports() -> TestResult {
    // @step Given a Dart source file with "import '../models/user.dart'"
    let dart_source = r#"
import '../models/user.dart';
import 'dart:math';

void loadUser() {
  return;
}
"#;

    // @step And the target file "models/user.dart" exists in the known files set
    let target_source = r#"
class User {
  String name;
  User(this.name);
}
"#;

    // @step When I run the Dart AST extractor on the file
    let (_directory, _root, entities) = indexed(&[
        ("lib/screens/home.dart", dart_source),
        ("lib/models/user.dart", target_source),
    ])?;

    // @step Then it should produce an Imports edge from the source file to "models/user.dart"
    assert!(
        has_edge(
            &entities,
            "Imports",
            "lib/screens/home.dart",
            "lib/models/user.dart"
        ),
        "Should have Imports edge to user.dart. All Imports: {:?}",
        named_edges(&entities, "Imports")
    );

    // @step And it should produce a stub File node for the import target
    // The import target is the indexed File node of lib/models/user.dart
    // (an Imports edge only leads to a file of the tree; no stub nodes).
    let target: Vec<_> = nodes(&entities, "File")
        .into_iter()
        .filter(|(slug, _)| slug == "lib/models/user.dart")
        .collect();
    assert_eq!(target.len(), 1, "the import target's File node");
    assert_eq!(
        target[0].1.get("language").and_then(Value::as_str),
        Some("dart")
    );
    assert_eq!(count_nodes(&entities, "File"), 2, "no stub File nodes");
    Ok(())
}

// ============================================================================
// Scenario: Skip external dart: and package: imports
// ============================================================================
#[test]
fn test_dart_skip_external_imports() -> TestResult {
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

    // @step When I run the Dart AST extractor on the file
    let (_directory, _root, entities) = indexed(&[("lib/main.dart", dart_source)])?;

    // @step Then it should NOT produce any Imports edges for the external imports
    let imports = named_edges(&entities, "Imports");
    assert!(
        imports.is_empty(),
        "Should NOT have Imports edges for dart: and package: imports. Got: {imports:?}"
    );
    assert_eq!(count_nodes(&entities, "File"), 1, "no stub File nodes");
    Ok(())
}

// ============================================================================
// Scenario: Extract Calls edges from function bodies
// ============================================================================
#[test]
fn test_dart_extract_calls_from_function_bodies() -> TestResult {
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

    // @step When I run the Dart AST extractor on the file
    let (_directory, _root, entities) = indexed(&[("lib/processor.dart", dart_source)])?;

    // @step Then it should produce a Calls edge from "processData" to "validateInput"
    assert!(
        has_edge(&entities, "Calls", "processData", "validateInput"),
        "Should have Calls edge from processData to validateInput. All Calls: {:?}",
        named_edges(&entities, "Calls")
    );
    Ok(())
}

// ============================================================================
// Scenario: Extract TypeRef edges from function signatures excluding builtins
// ============================================================================
#[test]
fn test_dart_extract_typerefs_excluding_builtins() -> TestResult {
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
    // @step And "UserModel" is defined as a class in the same file
    // (already included in the source above)

    // @step When I run the Dart AST extractor on the file
    let (_directory, _root, entities) = indexed(&[("lib/user_service.dart", dart_source)])?;

    // @step Then it should produce a TypeRef edge from "createUser" to "UserModel"
    assert!(
        has_edge(&entities, "TypeRef", "createUser", "UserModel"),
        "Should have TypeRef from createUser to UserModel. All TypeRef: {:?}",
        named_edges(&entities, "TypeRef")
    );

    // @step And it should NOT produce a TypeRef edge for the builtin type "String"
    let refs = named_edges(&entities, "TypeRef");
    assert!(
        refs.iter()
            .all(|(from, to)| from != "createUser" || to == "UserModel"),
        "Should NOT have TypeRef to builtin String type: {refs:?}"
    );
    Ok(())
}
