// Feature: spec/features/dart-dead-code-false-positive-reduction.feature
use super::*;

// ============================================================================
// Scenario: Exclude Dart extension declarations from unreferenced types
// ============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_exclude_extension_declarations_from_unreferenced_types() -> TestResult {
    // @step Given a Dart file with "extension StringExt on String" and "extension UserPatterns on User"
    let dart_source = r#"
class User {
  final String name;
  User(this.name);
}

extension StringExt on String {
  String capitalize() => this[0].toUpperCase() + substring(1);
}

extension UserPatterns on User {
  void map({required void Function(User) onUser}) => onUser(this);
}

class UnusedClass {
  void doNothing() {}
}

void useUser(User u) {
  return;
}
"#;

    // @step And a class "User" is defined and referenced by other functions
    let (_directory, root) = indexed(&[("lib/models.dart", dart_source)]).await?;
    let graph = entities(&root).await?;
    let type_kinds: Vec<(String, String)> = nodes(&graph, "Type")
        .into_iter()
        .map(|(_, p)| {
            let text = |key: &str| p.get(key).and_then(Value::as_str).unwrap_or("").to_string();
            (text("name"), text("typeKind"))
        })
        .collect();
    assert!(
        has_edge(&graph, "TypeRef", "useUser", "User"),
        "useUser references User: {:?}",
        named_edges(&graph, "TypeRef")
    );

    // @step When I run ast_dead_code detection on the indexed project
    let result = dead_code(&root, Some("Type")).await?;
    let unreferenced = items(&result, "Type", "name");

    // Extensions are extracted as Type nodes of typeKind "extension".
    for name in ["StringExt", "UserPatterns"] {
        assert!(
            type_kinds.contains(&(name.to_string(), "extension".to_string())),
            "{name} should be a Type of typeKind 'extension'. Got types: {type_kinds:?}"
        );
    }

    // @step Then extension types should not appear in the unreferenced_types results
    assert!(
        !unreferenced
            .iter()
            .any(|n| n == "StringExt" || n == "UserPatterns"),
        "Extension types should not be reported as unreferenced: {unreferenced:?}"
    );

    // @step And genuinely unreferenced types like an unused class should still appear
    assert!(
        unreferenced.contains(&"UnusedClass".to_string()),
        "UnusedClass should be reported as unreferenced: {unreferenced:?}"
    );
    Ok(())
}

// ============================================================================
// Scenario: Resolve qualified static method calls as Calls edges
// ============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_resolve_qualified_static_calls_as_calls_edges() -> TestResult {
    // @step Given a Dart file with a call "BoardFixtures.connectedInstance()" in a function body
    let test_source = r#"
import 'fixtures.dart';

void testBoard() {
  final board = BoardFixtures.connectedInstance();
  final data = BoardFixtures.boardWithColumns();
}
"#;

    // @step And "BoardFixtures" is a class with static method "connectedInstance" in an imported file
    let fixtures_source = r#"
class BoardFixtures {
  static Map<String, dynamic> connectedInstance() {
    return {'id': '1'};
  }

  static Map<String, dynamic> boardWithColumns() {
    return {'columns': []};
  }
}
"#;

    // @step When I run the Dart AST extractor on both files
    let (_directory, root) = indexed(&[
        ("test/board_test.dart", test_source),
        ("test/fixtures.dart", fixtures_source),
    ])
    .await?;
    let graph = entities(&root).await?;

    // @step Then a Calls edge should exist from the calling function to "connectedInstance"
    assert!(
        has_edge(&graph, "Calls", "testBoard", "connectedInstance"),
        "Qualified static call BoardFixtures.connectedInstance() should produce a Calls edge. Calls: {:?}",
        named_edges(&graph, "Calls")
    );

    // @step And "connectedInstance" should not appear as an uncalled function in dead code
    let uncalled = uncalled_functions(&root).await?;
    assert!(
        !uncalled.contains(&"connectedInstance".to_string()),
        "connectedInstance is called: {uncalled:?}"
    );
    Ok(())
}

// ============================================================================
// Scenario: Recognize constructor invocations as TypeRef edges in function bodies
// ============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_recognize_constructor_invocations_as_typeref_edges() -> TestResult {
    // @step Given a Dart function body containing "final repo = InMemoryConnectionRepository()"
    let setup_source = r#"
import 'repository.dart';

void setupTest() {
  final repo = InMemoryConnectionRepository();
  repo.save('test');
}
"#;

    // @step And "InMemoryConnectionRepository" is a class defined in an imported file
    let repository_source = r#"
class InMemoryConnectionRepository {
  final List<String> items = [];
  void save(String item) {
    items.add(item);
  }
}
"#;

    // @step When I run the Dart AST extractor on both files
    let (_directory, root) = indexed(&[
        ("lib/test_setup.dart", setup_source),
        ("lib/repository.dart", repository_source),
    ])
    .await?;
    let graph = entities(&root).await?;

    // @step Then a TypeRef edge should exist from the calling function to "InMemoryConnectionRepository"
    assert!(
        has_edge(&graph, "TypeRef", "setupTest", "InMemoryConnectionRepository"),
        "Constructor invocation InMemoryConnectionRepository() should produce a TypeRef edge. TypeRef: {:?}, Calls: {:?}",
        named_edges(&graph, "TypeRef"),
        named_edges(&graph, "Calls")
    );

    // @step And "InMemoryConnectionRepository" should not appear as unreferenced in dead code
    let result = dead_code(&root, Some("Type")).await?;
    let unreferenced = items(&result, "Type", "name");
    assert!(
        !unreferenced.contains(&"InMemoryConnectionRepository".to_string()),
        "InMemoryConnectionRepository is referenced: {unreferenced:?}"
    );
    Ok(())
}
