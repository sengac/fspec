// Feature: spec/features/dart-ast-index-support.feature
use super::*;

// ============================================================================
// Scenario: Identify uncalled Dart functions and unimported files as dead code
// ============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_dart_dead_code_detection() -> TestResult {
    use codelet_graph::ast_dead_code::dispatch_ast_dead_code;
    use codelet_graph::registry;

    // @step Given a Dart project with function "unusedHelper" that is never called and file "lib/orphan.dart" that is never imported
    let main_source = r#"
import 'utils.dart';

void main() {
  greet('World');
}
"#;
    let utils_source = r#"
String greet(String name) {
  return 'Hello, $name!';
}

void unusedHelper() {
  return;
}
"#;
    let orphan_source = r#"
void orphanFunction() {
  return;
}
"#;
    let (_directory, root) = project(&[
        ("lib/main.dart", main_source),
        ("lib/utils.dart", utils_source),
        ("lib/orphan.dart", orphan_source),
    ])?;

    // @step When I run ast_dead_code detection on the indexed project
    code_graph_fixture::index(&root).await?;
    let graph = registry::read_code_graph(&root).await?;
    let output: Value =
        serde_json::from_str(&dispatch_ast_dead_code(&graph, None, None, None).await)?;
    let field = |entity_type: &str, field: &str| -> Vec<String> {
        output["results"][entity_type]["items"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|i| i[field].as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    };
    let uncalled_names = field("Function", "name");
    let orphan_paths = field("File", "path");

    // @step Then "unusedHelper" should appear in the dead functions list
    assert!(
        uncalled_names.contains(&"unusedHelper".to_string()),
        "Uncalled functions should include 'unusedHelper', got: {uncalled_names:?}"
    );

    // @step Then "lib/orphan.dart" should appear in the dead files list
    assert!(
        orphan_paths.contains(&"lib/orphan.dart".to_string()),
        "Orphan files should include 'lib/orphan.dart', got: {orphan_paths:?}"
    );
    assert!(
        !orphan_paths.contains(&"lib/utils.dart".to_string()),
        "lib/utils.dart is imported by lib/main.dart, got: {orphan_paths:?}"
    );
    Ok(())
}
