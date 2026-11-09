// Feature: spec/features/dart-dead-code-false-positive-reduction.feature
use super::*;

// ============================================================================
// Scenario: Exclude test file functions from dead code results
// ============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_exclude_test_file_functions_from_dead_code() -> TestResult {
    let (_directory, root) = project(&[])?;
    let project_dir = root.as_path();

    // @step Given a Dart project with test files containing main() functions and helper functions
    let test_source = r#"
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('loads board', (tester) async {
    await tester.pumpWidget(MyApp());
  });
}

void helperSetup() {
  // test helper
}
"#;
    write_test_file(project_dir, "test/board_test.dart", test_source);

    let lib_source = r#"
void main() {
  runApp();
}

void runApp() {
  return;
}

void unusedFunction() {
  return;
}
"#;
    write_test_file(project_dir, "lib/main.dart", lib_source);
    write_test_file(project_dir, ".gitignore", "build/\n.dart_tool/\n");

    // @step And the test files are marked with isTest=true during extraction
    index(&root).await?;
    let graph = entities(&root).await?;

    // Verify test file is marked as test (the Dart table's test paths:
    // a `test` directory, a `_test` file stem), and so are its functions.
    let test_file: Vec<_> = nodes(&graph, "File")
        .into_iter()
        .filter(|(slug, _)| slug == "test/board_test.dart")
        .collect();
    assert_eq!(test_file.len(), 1, "Should find test file");
    assert_eq!(
        test_file[0].1.get("isTest").and_then(Value::as_bool),
        Some(true)
    );
    let test_functions: Vec<_> = nodes(&graph, "Function")
        .into_iter()
        .filter(|(slug, _)| slug.starts_with("test/board_test.dart::"))
        .collect();
    assert_eq!(
        test_functions.len(),
        2,
        "main and helperSetup: {test_functions:?}"
    );
    assert!(test_functions
        .iter()
        .all(|(_, p)| p.get("isTest").and_then(Value::as_bool) == Some(true)));
    let raw_uncalled = uncalled_functions(&root).await?;
    assert!(raw_uncalled.contains(&"helperSetup".to_string()));

    // @step When I run ast_dead_code detection on the indexed project
    let parsed = dead_code(&root, None).await?;
    let uncalled_names = items(&parsed, "Function", "name");
    let uncalled_slugs = items(&parsed, "Function", "slug");

    // @step Then functions in test files should not appear in the uncalled_functions results
    assert!(
        !uncalled_slugs.iter().any(|s| s.starts_with("test/")),
        "Functions from test files should NOT appear in uncalled_functions (dispatch must filter). Got: {:?}",
        uncalled_slugs
    );

    // @step And functions in non-test files that are genuinely uncalled should still appear
    assert!(
        uncalled_names.contains(&"unusedFunction".to_string()),
        "Genuinely uncalled function in non-test file should still appear. Got: {:?}",
        uncalled_names
    );
    Ok(())
}

// ============================================================================
// Scenario: Exclude generated .g.dart and .freezed.dart files from dead code
// ============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_exclude_generated_dart_files_from_dead_code() -> TestResult {
    let (_directory, root) = project(&[])?;
    let project_dir = root.as_path();

    // @step Given a Dart project with generated files ending in ".g.dart" and ".freezed.dart"
    let gen_freezed = r#"
class UserPatterns {
  void map() {}
}

class _$UserImpl implements User {
  final String name;
  const _$UserImpl({required this.name});
}
"#;
    write_test_file(project_dir, "lib/models/user.freezed.dart", gen_freezed);

    let gen_g = r#"
Map<String, dynamic> _$UserToJson(User instance) {
  return {'name': instance.name};
}

User _$UserFromJson(Map<String, dynamic> json) {
  return User(name: json['name'] as String);
}
"#;
    write_test_file(project_dir, "lib/models/user.g.dart", gen_g);

    // @step And those generated files contain types and functions from code generation
    let regular_source = r#"
import 'models/user.freezed.dart';

class User {
  final String name;
  User({required this.name});
}

void unusedRegularFunction() {
  return;
}
"#;
    write_test_file(project_dir, "lib/main.dart", regular_source);
    write_test_file(project_dir, ".gitignore", "build/\n.dart_tool/\n");

    index(&root).await?;
    // The generated files' entities are in the graph (and dead before the
    // generated-file filter).
    let raw_uncalled = uncalled_functions(&root).await?;
    assert!(
        raw_uncalled.contains(&"_$UserToJson".to_string()),
        "{raw_uncalled:?}"
    );
    let raw_unreferenced = unreferenced_types(&root).await?;
    assert!(
        raw_unreferenced.contains(&"UserPatterns".to_string()),
        "{raw_unreferenced:?}"
    );

    // @step When I run ast_dead_code detection on the indexed project
    let parsed = dead_code(&root, None).await?;

    // Check uncalled functions
    let uncalled_slugs = items(&parsed, "Function", "slug");

    // Check unreferenced types
    let unreferenced_slugs = items(&parsed, "Type", "slug");

    // @step Then no entities from ".g.dart" files should appear in dead code results
    let g_dart_uncalled: Vec<&String> = uncalled_slugs
        .iter()
        .filter(|s| s.starts_with("lib/models/user.g.dart::"))
        .collect();
    let g_dart_unreferenced: Vec<&String> = unreferenced_slugs
        .iter()
        .filter(|s| s.starts_with("lib/models/user.g.dart::"))
        .collect();
    assert!(
        g_dart_uncalled.is_empty(),
        "No uncalled functions from .g.dart should appear. Got: {:?}",
        g_dart_uncalled
    );
    assert!(
        g_dart_unreferenced.is_empty(),
        "No unreferenced types from .g.dart should appear. Got: {:?}",
        g_dart_unreferenced
    );

    // @step And no entities from ".freezed.dart" files should appear in dead code results
    let freezed_uncalled: Vec<&String> = uncalled_slugs
        .iter()
        .filter(|s| s.starts_with("lib/models/user.freezed.dart::"))
        .collect();
    let freezed_unreferenced: Vec<&String> = unreferenced_slugs
        .iter()
        .filter(|s| s.starts_with("lib/models/user.freezed.dart::"))
        .collect();
    assert!(
        freezed_uncalled.is_empty(),
        "No uncalled functions from .freezed.dart should appear. Got: {:?}",
        freezed_uncalled
    );
    assert!(
        freezed_unreferenced.is_empty(),
        "No unreferenced types from .freezed.dart should appear. Got: {:?}",
        freezed_unreferenced
    );

    // @step And entities from regular ".dart" files that are genuinely dead should still appear
    let regular_uncalled: Vec<&String> = uncalled_slugs
        .iter()
        .filter(|s| !s.contains(".g.") && !s.contains(".freezed."))
        .collect();
    // unusedRegularFunction should still be in regular results
    assert!(
        regular_uncalled
            .iter()
            .any(|s| s.contains("unusedRegularFunction")),
        "Genuinely dead regular function should appear. Uncalled slugs: {:?}",
        uncalled_slugs
    );
    Ok(())
}
