// Feature: spec/features/multi-language-ast-extraction.feature
use super::*;

// ============================================================================
// Java Extractor Tests
// ============================================================================
#[tokio::test]
async fn test_extract_java_methods_and_types() {
    // @step Given a Java file with public/private methods and class/interface/enum declarations
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let java_content = r#"
package com.example;

public class UserController {

    public String getUser(int id) {
        return "user";
    }

    private void logAccess(String action) {
        System.out.println(action);
    }
}

interface UserService {
    String findUser(int id);
}

enum UserRole {
    ADMIN,
    USER,
    GUEST
}
"#;
    let file_path = write_test_file(
        temp_dir.path(),
        "src/main/UserController.java",
        java_content,
    );

    // @step When the Java extractor parses the file
    let entities = index_file(&file_path, temp_dir.path());

    assert_eq!(count_nodes(&entities, "File"), 1);
    let file_node = find_node(&entities, "File", "src/main/UserController.java");
    assert!(
        file_node.is_some(),
        "Should find File node for UserController.java"
    );
    let Some(GraphEntity::Node { properties, .. }) = file_node else {
        panic!("{} not found", stringify!(file_node));
    };
    assert_eq!(
        properties.get("language").and_then(|v| v.as_str()),
        Some("java")
    );

    // @step Then Function nodes should be created with isPublic from access modifiers
    let fn_count = count_nodes(&entities, "Function");
    assert!(
        fn_count >= 2,
        "Should extract at least 2 methods, got {fn_count}"
    );

    // Verify isPublic per-method based on access modifiers
    let get_user_fn = find_node(
        &entities,
        "Function",
        "src/main/UserController.java::UserController::getUser",
    );
    let Some(GraphEntity::Node { properties, .. }) = &get_user_fn else {
        panic!("{} not found", stringify!(&get_user_fn));
    };
    assert_eq!(
        properties.get("isPublic").and_then(|v| v.as_bool()),
        Some(true),
        "public method getUser should have isPublic=true"
    );
    let log_fn = find_node(
        &entities,
        "Function",
        "src/main/UserController.java::UserController::logAccess",
    );
    let Some(GraphEntity::Node { properties, .. }) = &log_fn else {
        panic!("{} not found", stringify!(&log_fn));
    };
    assert_eq!(
        properties.get("isPublic").and_then(|v| v.as_bool()),
        Some(false),
        "private method logAccess should have isPublic=false"
    );

    // @step And Type nodes should be created for classes, interfaces, and enums
    let type_count = count_nodes(&entities, "Type");
    assert!(
        type_count >= 3,
        "Should find at least 3 Type nodes (class + interface + enum), got {type_count}"
    );

    // Edges
    assert!(count_edges(&entities, "Contains") >= 1);
    assert!(
        count_edges(&entities, "ContainsType") >= 1,
        "Should have ContainsType edges for type declarations"
    );
}

// ============================================================================
// C Extractor Tests
// ============================================================================
#[tokio::test]
async fn test_extract_c_functions_and_types() {
    // @step Given a C file with function definitions, structs, enums, and typedefs
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let c_content = r#"
#include <stdio.h>

typedef int score_t;

struct Point {
    int x;
    int y;
};

enum Color {
    RED,
    GREEN,
    BLUE
};

int add(int a, int b) {
    return a + b;
}

static void internal_helper(int x) {
    printf("%d\n", x);
}

void print_point(struct Point p) {
    printf("(%d, %d)\n", p.x, p.y);
}
"#;
    let file_path = write_test_file(temp_dir.path(), "src/geometry.c", c_content);

    // @step When the C extractor parses the file
    let entities = index_file(&file_path, temp_dir.path());

    assert_eq!(count_nodes(&entities, "File"), 1);
    let file_node = find_node(&entities, "File", "src/geometry.c");
    assert!(file_node.is_some(), "Should find File node for geometry.c");
    let Some(GraphEntity::Node { properties, .. }) = file_node else {
        panic!("{} not found", stringify!(file_node));
    };
    assert_eq!(
        properties.get("language").and_then(|v| v.as_str()),
        Some("c")
    );

    // @step Then Function nodes should be created with isPublic false for static functions
    let fn_count = count_nodes(&entities, "Function");
    assert!(
        fn_count >= 2,
        "Should extract at least 2 functions, got {fn_count}"
    );

    // Verify static function is not public
    let internal_fn = find_node(&entities, "Function", "src/geometry.c::internal_helper");
    assert!(
        internal_fn.is_some(),
        "Should find static function internal_helper"
    );
    let Some(GraphEntity::Node { properties, .. }) = internal_fn else {
        panic!("{} not found", stringify!(internal_fn));
    };
    assert_eq!(
        properties.get("isPublic").and_then(|v| v.as_bool()),
        Some(false),
        "static functions should have isPublic=false"
    );

    // Verify non-static function is public
    let add_fn = find_node(&entities, "Function", "src/geometry.c::add");
    assert!(add_fn.is_some(), "Should find non-static function add");
    let Some(GraphEntity::Node { properties, .. }) = add_fn else {
        panic!("{} not found", stringify!(add_fn));
    };
    assert_eq!(
        properties.get("isPublic").and_then(|v| v.as_bool()),
        Some(true),
        "Non-static functions should have isPublic=true"
    );

    // @step And Type nodes should be created for structs, enums, and typedefs
    let type_count = count_nodes(&entities, "Type");
    assert!(
        type_count >= 3,
        "Should find at least 3 Type nodes (struct + enum + typedef), got {type_count}"
    );
}

#[tokio::test]
async fn test_c_header_detection() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let h_content = r#"
#ifndef POINT_H
#define POINT_H

struct Point {
    int x;
    int y;
};

int add(int a, int b) {
    return a + b;
}

#endif
"#;
    let file_path = write_test_file(temp_dir.path(), "include/point.h", h_content);

    let entities = index_file(&file_path, temp_dir.path());

    let file_node = find_node(&entities, "File", "include/point.h");
    assert!(file_node.is_some(), "Should find File node for point.h");
    let Some(GraphEntity::Node { properties, .. }) = file_node else {
        panic!("{} not found", stringify!(file_node));
    };
    // `.h` is declared by the C++ table (product owner, 2026-11-07): a pure
    // C header is parsed as C++, whose grammar is a superset for headers.
    assert_eq!(
        properties.get("language").and_then(|v| v.as_str()),
        Some("cpp")
    );

    // Verify extraction actually works for the header — at minimum functions should be found
    let fn_count = count_nodes(&entities, "Function");
    assert!(
        fn_count >= 1,
        "Should extract at least 1 function from header, got {fn_count}"
    );
    // Struct may or may not be extracted depending on preprocessor guards, but function is the key check
}
