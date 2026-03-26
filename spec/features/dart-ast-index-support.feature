@KGRAPH-057
Feature: Add Dart language support to GraphSearch AST index and dead code detection

  """
  The Dart extractor must use DartLang (from codelet/tools) for KindMatcher, not pattern matching, since Dart splits function_signature and function_body as sibling nodes at top level
  tree-sitter-dart must be added as a dependency to codelet/napi/Cargo.toml (it's already in codelet/tools from KGRAPH-056)
  The DartLang struct from codelet/tools/src/dart_lang.rs can be reused via codelet-tools dependency — or duplicated locally in codelet/napi as a simple struct implementing Language+LanguageExt. Since codelet-tools is already a dependency, reuse via pub import is preferred.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. ast_dart_extractor.rs must follow the established extractor pattern: extract File, Function, Type nodes and Contains, ContainsType, Imports, Calls, TypeRef edges
  #   2. Function extraction must cover: function_signature (top-level), method_signature (class methods), constructor_signature, constant_constructor_signature, factory_constructor_signature, getter_signature, setter_signature, operator_signature
  #   3. Type extraction must cover: class_declaration, enum_declaration, mixin_declaration, extension_declaration, extension_type_declaration, type_alias, mixin_application_class
  #   4. Import extraction must handle relative imports (../foo.dart, ./bar.dart, baz.dart) as local Imports edges, skip dart: and package: imports as external
  #   5. Calls edges must be extracted from function bodies using the shared extract_call_names_from_body helper
  #   6. TypeRef edges must be extracted from function signatures via colon-based type annotations (param: Type, : ReturnType)
  #   7. pubspec_dep_extractor.rs must parse dependencies and dev_dependencies from pubspec.yaml and emit Dependency nodes + DependsOn edges
  #   8. The .dart extension must be added to SUPPORTED_EXTENSIONS and the extract_file dispatch in mod.rs
  #   9. pubspec.yaml dependency extraction must be wired into ast_dispatch.rs dispatch_ast_index alongside other dep extractors
  #   10. Dart uses $ for string interpolation but $ IS valid in Dart identifiers, so DartLang from codelet/tools does NOT use expando_char — the AST extractor must use DartLang directly rather than SupportLang (since Dart is not in ast-grep's SupportLang enum)
  #
  # EXAMPLES:
  #   1. Indexing a Dart project with classes, functions, enums, and mixins produces correct File, Function, Type nodes with proper Contains/ContainsType edges
  #   2. A Dart file with relative imports (import '../models/user.dart') produces Imports edges to the target file when the target exists in known_files
  #   3. A Dart file with dart: and package: imports does NOT produce Imports edges (external imports are skipped)
  #   4. A function calling another local function produces a Calls edge; calling an imported function from a relative import produces a cross-file Calls edge
  #   5. A function with typed parameters (String name, int age) produces TypeRef edges to locally-defined types but NOT to Dart builtins
  #   6. pubspec.yaml with dependencies: and dev_dependencies: sections produces Dependency nodes with correct isDev flag and version constraints
  #   7. Named constructors (User.fromJson), factory constructors, and const constructors are extracted as Function nodes
  #   8. ast_dead_code correctly identifies uncalled Dart functions and files without importers
  #
  # ========================================

  Background: User Story
    As a developer
    I want to index Dart/Flutter projects with ast_index
    So that I get code navigation, dead code detection, and dependency tracking for Dart codebases

  Scenario: Extract File, Function, and Type nodes from Dart source with Contains edges
    Given a Dart source file with classes, top-level functions, enums, and mixins
    When I run the Dart AST extractor on the file
    Then it should produce a File node with language "dart"
    And it should produce Function nodes for each top-level function and class method
    And it should produce Type nodes for each class, enum, and mixin
    And it should produce Contains edges from the File to each Function
    And it should produce ContainsType edges from the File to each Type

  Scenario: Extract relative imports as Imports edges
    Given a Dart source file with "import '../models/user.dart'"
    And the target file "models/user.dart" exists in the known files set
    When I run the Dart AST extractor on the file
    Then it should produce an Imports edge from the source file to "models/user.dart"
    And it should produce a stub File node for the import target

  Scenario: Skip external dart: and package: imports
    Given a Dart source file with "import 'dart:math'" and "import 'package:flutter/material.dart'"
    When I run the Dart AST extractor on the file
    Then it should NOT produce any Imports edges for the external imports

  Scenario: Extract Calls edges from function bodies
    Given a Dart source file with a function "processData" that calls local function "validateInput"
    When I run the Dart AST extractor on the file
    Then it should produce a Calls edge from "processData" to "validateInput"

  Scenario: Extract TypeRef edges from function signatures excluding builtins
    Given a Dart source file with a function "createUser(UserModel model, String name)"
    And "UserModel" is defined as a class in the same file
    When I run the Dart AST extractor on the file
    Then it should produce a TypeRef edge from "createUser" to "UserModel"
    And it should NOT produce a TypeRef edge for the builtin type "String"

  Scenario: Extract dependencies from pubspec.yaml
    Given a pubspec.yaml file with "provider: ^6.0.0" in dependencies and "build_runner: ^2.4.0" in dev_dependencies
    When I run the pubspec dependency extractor
    Then it should produce a Dependency node for "provider" with isDev false and version "^6.0.0"
    And it should produce a Dependency node for "build_runner" with isDev true and version "^2.4.0"
    And it should produce DependsOn edges from the pubspec.yaml File to each Dependency

  Scenario: Extract named constructors and factory constructors as Function nodes
    Given a Dart class with constructor "User(this.name)", named constructor "User.fromJson", and factory constructor "factory User.create"
    When I run the Dart AST extractor on the file
    Then it should produce Function nodes for "User", "fromJson", and "create"

  Scenario: Dart files are included in SUPPORTED_EXTENSIONS and dispatched correctly
    Given a project directory containing ".dart" files
    When I run walk_and_extract on the project directory
    Then the ".dart" files should be discovered and extracted using the Dart extractor

  Scenario: Identify uncalled Dart functions and unimported files as dead code
    Given a Dart project with function "unusedHelper" that is never called and file "lib/orphan.dart" that is never imported
    When I run ast_dead_code detection on the indexed project
    Then "unusedHelper" should appear in the dead functions list
    Then "lib/orphan.dart" should appear in the dead files list

