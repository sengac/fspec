@done
@KGRAPH-058
Feature: Dart AST extractor produces excessive false positives in dead code detection for Flutter projects
  """
  Primary fix locations: ast_dispatch.rs (dead code filters), ast_dart_extractor.rs (Dart-specific edge extraction), edge_helpers.rs (qualified call resolution). Most P0 fixes are in the dispatch filter layer, not the extractor itself.
  The generated file exclusion (.g.dart, .freezed.dart) should be a Dart-specific filter in dispatch, not hardcoded in the query — other languages have their own code generation patterns (e.g., .pb.go for protobuf in Go).
  Qualified call resolution (ClassName.method()) is a cross-language issue — TypeScript/Java/Kotlin/C# all have static method calls. Any fix should be in the shared edge_helpers, not Dart-specific.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Functions in test files (isTest=true) must be excluded from uncalled_functions dead code results
  #   2. Generated Dart files (.g.dart and .freezed.dart) must be excluded from dead code analysis entirely
  #   3. Flutter platform directories (ios/, android/, macos/, linux/, windows/) must be excluded from dead code for projects with flutter dependency
  #   4. Dart main.dart entry point files must not be flagged as orphan files
  #   5. Dart extension declarations must either be linked to their target type or excluded from unreferenced_types
  #   6. Qualified/static method calls (ClassName.method()) must produce Calls edges to the method in the target file
  #   7. Constructor invocations (PascalCase identifiers followed by parentheses in function bodies) must produce TypeRef edges
  #   8. Flutter StatefulWidget _*State private classes must be recognized as used by the parent widget
  #
  # EXAMPLES:
  #   1. 14 test main() functions in fspec-mobile flagged dead but are runner entry points — after fix, 0 test functions in dead code
  #   2. 40 Freezed-generated types in .freezed.dart files flagged dead — after fix, 0 generated types in dead code
  #   3. 30 platform runner files (ios/AppDelegate.swift, android/MainActivity.kt, etc.) flagged dead — after fix, 0 platform files in dead code
  #   4. BoardFixtures.connectedInstance() called from 10 test files but flagged as dead function — after fix, correctly shows as called
  #   5. Genuinely dead WebSocketFixtures.pongMessage() still correctly reported as dead after all fixes
  #   6. 14 Freezed *Patterns extension types flagged dead — after fix, extension declarations linked to target type or excluded
  #
  # ========================================
  Background: User Story
    As a developer
    I want to get accurate dead code analysis results for Flutter/Dart projects
    So that I can trust the dead code report and clean up only genuinely unused code

  # Rule: Functions in test files must be excluded from uncalled_functions dead code results
  @critical
  Scenario: Exclude test file functions from dead code results
    Given a Dart project with test files containing main() functions and helper functions
    And the test files are marked with isTest=true during extraction
    When I run ast_dead_code detection on the indexed project
    Then functions in test files should not appear in the uncalled_functions results
    And functions in non-test files that are genuinely uncalled should still appear

  @critical
  Scenario: Exclude generated .g.dart and .freezed.dart files from dead code
  # Rule: Generated Dart files must be excluded from dead code analysis entirely
    Given a Dart project with generated files ending in ".g.dart" and ".freezed.dart"
    And those generated files contain types and functions from code generation
    When I run ast_dead_code detection on the indexed project
    Then no entities from ".g.dart" files should appear in dead code results
    And no entities from ".freezed.dart" files should appear in dead code results
    And entities from regular ".dart" files that are genuinely dead should still appear

  @critical
  Scenario: Exclude Flutter platform directories from dead code for Flutter projects
  # Rule: Flutter platform directories must be excluded from dead code
    Given a Flutter project with platform directories "ios/", "android/", "macos/", "linux/", and "windows/"
    And the project has a pubspec.yaml with a "flutter" dependency
    When I run ast_dead_code detection on the indexed project
    Then no files from platform directories should appear in the orphan files results
    And non-platform orphan files should still be detected

  @high
  Scenario: Do not flag main.dart entry point as orphan file
  # Rule: Dart main.dart entry point must not be flagged as orphan file
    Given a Dart project with "lib/main.dart" containing a top-level main() function
    And no other file imports main.dart
    When I run ast_dead_code detection on the indexed project
    Then "lib/main.dart" should not appear in the orphan files results
    And other files that are genuinely orphaned should still appear

  @high
  Scenario: Exclude Dart extension declarations from unreferenced types
  # Rule: Extension declarations must be linked to target type or excluded from unreferenced_types
    Given a Dart file with "extension StringExt on String" and "extension UserPatterns on User"
    And a class "User" is defined and referenced by other functions
    When I run ast_dead_code detection on the indexed project
    Then extension types should not appear in the unreferenced_types results
    And genuinely unreferenced types like an unused class should still appear

  @high
  Scenario: Resolve qualified static method calls as Calls edges
  # Rule: Qualified/static method calls must produce Calls edges
    Given a Dart file with a call "BoardFixtures.connectedInstance()" in a function body
    And "BoardFixtures" is a class with static method "connectedInstance" in an imported file
    When I run the Dart AST extractor on both files
    Then a Calls edge should exist from the calling function to "connectedInstance"
    And "connectedInstance" should not appear as an uncalled function in dead code

  @medium
  Scenario: Recognize constructor invocations as TypeRef edges in function bodies
  # Rule: Constructor invocations must produce TypeRef edges
    Given a Dart function body containing "final repo = InMemoryConnectionRepository()"
    And "InMemoryConnectionRepository" is a class defined in an imported file
    When I run the Dart AST extractor on both files
    Then a TypeRef edge should exist from the calling function to "InMemoryConnectionRepository"
    And "InMemoryConnectionRepository" should not appear as unreferenced in dead code

  @medium
  Scenario: Recognize StatefulWidget State classes as used by parent widget
  # Rule: Flutter StatefulWidget State classes must be recognized as used
    Given a Dart file with "class MyScreen extends StatefulWidget" and "class _MyScreenState extends State<MyScreen>"
    And "_MyScreenState" is only referenced in the createState() method
    When I run ast_dead_code detection on the indexed project
    Then "_MyScreenState" should not appear in the unreferenced_types results

  # Validation: Genuinely dead code is still detected after all fixes
  Scenario: Genuinely dead code is still reported after false positive fixes
    Given a Dart project with a genuinely unused function "pongMessage" in a test fixtures file
    And a genuinely unused type "FakeWebSocketChannel" in the same fixtures file
    And all false positive reduction filters are active
    When I run ast_dead_code detection on the indexed project
    Then "pongMessage" should still appear in the uncalled functions results
    And "FakeWebSocketChannel" should still appear in the unreferenced types results
