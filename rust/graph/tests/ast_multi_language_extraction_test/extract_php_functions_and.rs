// Feature: spec/features/multi-language-ast-extraction.feature
use super::*;

// ============================================================================
// PHP Extractor Tests
// ============================================================================
#[tokio::test]
async fn test_extract_php_functions_and_types() {
    // @step Given a PHP file with function, class, interface, and trait declarations
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let php_content = r#"<?php

namespace App\Controllers;

class UserController {
    public function index() {
        return "users";
    }

    private function validateInput($data) {
        return true;
    }

    public static function create($name) {
        return new self();
    }
}

interface UserServiceInterface {
    public function findUser($id);
}

trait Cacheable {
    public function cache($key, $value) {
        return true;
    }
}

function globalHelper($msg) {
    echo $msg;
}
"#;
    let file_path = write_test_file(
        temp_dir.path(),
        "app/Controllers/UserController.php",
        php_content,
    );

    // @step When the PHP extractor parses the file
    let entities = index_file(&file_path, temp_dir.path());

    assert_eq!(count_nodes(&entities, "File"), 1);
    let file_node = find_node(&entities, "File", "app/Controllers/UserController.php");
    let Some(GraphEntity::Node { properties, .. }) = file_node else {
        panic!("{} not found", stringify!(file_node));
    };
    assert_eq!(
        properties.get("language").and_then(|v| v.as_str()),
        Some("php")
    );

    // @step Then Function nodes and Type nodes should be created
    let fn_count = count_nodes(&entities, "Function");
    assert!(
        fn_count >= 1,
        "Should extract at least 1 function, got {fn_count}"
    );

    let type_count = count_nodes(&entities, "Type");
    assert!(
        type_count >= 1,
        "Should find at least 1 Type node, got {type_count}"
    );
}

// ============================================================================
// Dependency Extractor Tests
// ============================================================================
#[tokio::test]
async fn test_extract_python_dependencies() {
    // @step Given a project with requirements.txt listing packages with version constraints
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let req_content = "flask>=2.0\nrequests==2.28.0\n# comment\npytest>=7.0\n";
    write_test_file(temp_dir.path(), "requirements.txt", req_content);

    // @step When the pip dependency extractor runs
    let entities = codelet_graph::ast_pipeline::pip_dep_extractor::extract_python_dependencies(
        temp_dir.path(),
    )
    .into_entities()
    .expect("Python dep extraction should succeed");

    // @step Then Dependency nodes should be created with source "pip"
    let dep_count = count_nodes(&entities, "Dependency");
    assert!(
        dep_count >= 3,
        "Should extract at least 3 dependencies, got {dep_count}"
    );
    assert!(
        has_dependency_with_source(&entities, "pip"),
        "Dependency source should be 'pip'"
    );

    let file_count = count_nodes(&entities, "File");
    assert!(
        file_count >= 1,
        "Should create File node for requirements.txt"
    );

    // @step And DependsOn edges should link manifest files to dependencies
    let depends_on_count = count_edges(&entities, "DependsOn");
    assert!(
        depends_on_count >= 1,
        "Should have DependsOn edges linking manifest to deps, got {depends_on_count}"
    );
}

#[tokio::test]
async fn test_extract_python_dependencies_pyproject() {
    // @step And a pyproject.toml with project.dependencies and optional-dependencies
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let pyproject_content = r#"
[project]
name = "myproject"
dependencies = [
    "flask>=2.0",
    "requests",
]

[project.optional-dependencies]
dev = ["pytest>=7.0", "black"]
"#;
    write_test_file(temp_dir.path(), "pyproject.toml", pyproject_content);

    // @step When the pip dependency extractor runs
    let entities = codelet_graph::ast_pipeline::pip_dep_extractor::extract_python_dependencies(
        temp_dir.path(),
    )
    .into_entities()
    .expect("Python pyproject dep extraction should succeed");

    // @step Then Dependency nodes should be created with source "pip"
    let dep_count = count_nodes(&entities, "Dependency");
    assert!(
        dep_count >= 2,
        "Should extract at least 2 dependencies, got {dep_count}"
    );
    assert!(
        has_dependency_with_source(&entities, "pip"),
        "Dependency source should be 'pip'"
    );
}

#[tokio::test]
async fn test_extract_go_dependencies() {
    // @step Given a project with go.mod listing require blocks
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let gomod_content = r#"module github.com/example/myapp

go 1.21

require (
    github.com/gin-gonic/gin v1.9.1
    github.com/stretchr/testify v1.8.4 // indirect
)
"#;
    write_test_file(temp_dir.path(), "go.mod", gomod_content);

    // @step When the go.mod dependency extractor runs
    let entities =
        codelet_graph::ast_pipeline::gomod_dep_extractor::extract_go_dependencies(temp_dir.path())
            .into_entities()
            .expect("Go dep extraction should succeed");

    // @step Then Dependency nodes should be created with source "go"
    let dep_count = count_nodes(&entities, "Dependency");
    assert!(
        dep_count >= 2,
        "Should extract at least 2 dependencies, got {dep_count}"
    );
    assert!(
        has_dependency_with_source(&entities, "go"),
        "Dependency source should be 'go'"
    );
}

#[tokio::test]
async fn test_extract_java_dependencies_gradle() {
    // @step Given a project with pom.xml dependencies and build.gradle implementation blocks
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let gradle_content = r#"
plugins {
    id 'java'
}

dependencies {
    implementation 'org.springframework:spring-core:5.3.0'
    testImplementation 'junit:junit:4.13'
}
"#;
    write_test_file(temp_dir.path(), "build.gradle", gradle_content);

    // @step When the Java dependency extractors run
    let entities =
        codelet_graph::ast_pipeline::java_dep_extractor::extract_java_dependencies(temp_dir.path())
            .into_entities()
            .expect("Java dep extraction should succeed");

    // @step Then Dependency nodes should be created with source "maven" or "gradle"
    let dep_count = count_nodes(&entities, "Dependency");
    assert!(
        dep_count >= 2,
        "Should extract at least 2 dependencies, got {dep_count}"
    );
    assert!(
        has_dependency_with_source(&entities, "gradle"),
        "Dependency source should be 'gradle'"
    );
}

#[tokio::test]
async fn test_extract_java_dependencies_maven() {
    // @step Given a project with pom.xml dependencies and build.gradle implementation blocks
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let pom_content = r#"<?xml version="1.0"?>
<project>
    <dependencies>
        <dependency>
            <groupId>org.springframework</groupId>
            <artifactId>spring-core</artifactId>
            <version>5.3.0</version>
        </dependency>
        <dependency>
            <groupId>junit</groupId>
            <artifactId>junit</artifactId>
            <version>4.13</version>
            <scope>test</scope>
        </dependency>
    </dependencies>
</project>
"#;
    write_test_file(temp_dir.path(), "pom.xml", pom_content);

    // @step When the Java dependency extractors run
    let entities =
        codelet_graph::ast_pipeline::java_dep_extractor::extract_java_dependencies(temp_dir.path())
            .into_entities()
            .expect("Maven dep extraction should succeed");

    // @step Then Dependency nodes should be created with source "maven" or "gradle"
    let dep_count = count_nodes(&entities, "Dependency");
    assert!(
        dep_count >= 2,
        "Should extract at least 2 dependencies, got {dep_count}"
    );
    assert!(
        has_dependency_with_source(&entities, "maven"),
        "Dependency source should be 'maven'"
    );
}
