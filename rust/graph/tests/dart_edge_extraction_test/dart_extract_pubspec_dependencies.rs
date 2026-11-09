// Feature: spec/features/dart-ast-index-support.feature
use super::*;

// ============================================================================
// Scenario: Extract dependencies from pubspec.yaml
// ============================================================================
#[test]
fn test_dart_extract_pubspec_dependencies() -> TestResult {
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
    let (_directory, root) = project(&[("pubspec.yaml", pubspec_content)])?;

    // @step When I run the pubspec dependency extractor
    let entities = extract_pubspec_dependencies(&root).into_entities()?;

    // @step Then it should produce a Dependency node for "provider" with isDev false and version "^6.0.0"
    let provider_dep = entities.iter().find(|e| {
        if let GraphEntity::Node {
            node_type,
            properties,
            ..
        } = e
        {
            node_type == "Dependency"
                && properties.get("name").and_then(|v| v.as_str()) == Some("provider")
        } else {
            false
        }
    });
    assert!(
        provider_dep.is_some(),
        "Should have Dependency node for 'provider'"
    );
    let Some(GraphEntity::Node { properties, .. }) = provider_dep else {
        panic!("no provider Dependency node");
    };
    assert_eq!(
        properties.get("isDev").and_then(|v| v.as_bool()),
        Some(false)
    );
    assert_eq!(
        properties.get("version").and_then(|v| v.as_str()),
        Some("^6.0.0")
    );

    // @step And it should produce a Dependency node for "build_runner" with isDev true and version "^2.4.0"
    let build_runner_dep = entities.iter().find(|e| {
        if let GraphEntity::Node {
            node_type,
            properties,
            ..
        } = e
        {
            node_type == "Dependency"
                && properties.get("name").and_then(|v| v.as_str()) == Some("build_runner")
        } else {
            false
        }
    });
    assert!(
        build_runner_dep.is_some(),
        "Should have Dependency node for 'build_runner'"
    );
    let Some(GraphEntity::Node { properties, .. }) = build_runner_dep else {
        panic!("no build_runner Dependency node");
    };
    assert_eq!(
        properties.get("isDev").and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        properties.get("version").and_then(|v| v.as_str()),
        Some("^2.4.0")
    );

    // @step And it should produce DependsOn edges from the pubspec.yaml File to each Dependency
    let depends_on = edges(&entities, "DependsOn");
    assert!(
        depends_on.len() >= 2,
        "Should have at least 2 DependsOn edges (provider, build_runner), got {depends_on:?}"
    );
    // A manifest's File node slug is its path.
    assert!(depends_on.iter().all(|(from, _, _)| from == "pubspec.yaml"));
    Ok(())
}

// ============================================================================
// Scenario: Extract named constructors and factory constructors as Function nodes
// ============================================================================
#[test]
fn test_dart_extract_constructors_as_functions() -> TestResult {
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

    // @step When I run the Dart AST extractor on the file
    let (_directory, _root, entities) = indexed(&[("lib/models/user.dart", dart_source)])?;

    // @step Then it should produce Function nodes for "User", "fromJson", and "create"
    let fn_nodes: Vec<_> = entities
        .iter()
        .filter_map(|e| {
            if let GraphEntity::Node {
                node_type,
                properties,
                ..
            } = e
            {
                if node_type == "Function" {
                    return properties.get("name").and_then(|v| v.as_str());
                }
            }
            None
        })
        .collect();

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
    Ok(())
}

// ============================================================================
// Scenario: Dart files are included in SUPPORTED_EXTENSIONS and dispatched correctly
// ============================================================================
#[test]
fn test_dart_files_dispatched_in_walk_and_extract() -> TestResult {
    // @step Given a project directory containing ".dart" files
    let dart_source = r#"
void main() {
  print('Hello');
}
"#;
    let (_directory, root) = project(&[("lib/main.dart", dart_source)])?;

    // @step When I run walk_and_extract on the project directory
    let entities = codelet_graph::ast_pipeline::walk_and_extract(&root, false);

    // @step Then the ".dart" files should be discovered and extracted using the Dart extractor
    let dart_file = entities.iter().find(|e| {
        if let GraphEntity::Node {
            node_type,
            properties,
            ..
        } = e
        {
            node_type == "File"
                && properties.get("language").and_then(|v| v.as_str()) == Some("dart")
        } else {
            false
        }
    });
    assert!(
        dart_file.is_some(),
        "Should discover and extract .dart files. Entities: {:?}",
        entities
            .iter()
            .filter(|e| matches!(e, GraphEntity::Node { node_type, .. } if node_type == "File"))
            .collect::<Vec<_>>()
    );
    Ok(())
}
