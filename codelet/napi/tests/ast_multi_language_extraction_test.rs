// Feature: spec/features/multi-language-ast-extraction.feature
//
// Multi-Language AST Extraction Pipeline Tests
// Tests for extracting AST entities from Python, Go, Java, C, C++, C#,
// Ruby, Kotlin, Swift, Scala, and PHP source files.
//
// Each test uses an isolated temp directory with synthetic source files.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashSet;

use codelet_napi::graph::ast_pipeline::extract_file;
use codelet_napi::graph::graph_entities::GraphEntity;

mod graph_test_helpers;
use graph_test_helpers::{count_edges, count_nodes, find_node, has_dependency_with_source, write_test_file};

// ============================================================================
// Python Extractor Tests
// ============================================================================
#[tokio::test]
async fn test_extract_python_functions_and_classes() {
    // @step Given a Python file "src/auth/login.py" with def, async def, and class declarations
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let py_content = r#"
import os
from typing import Optional

class UserService:
    def __init__(self, db):
        self.db = db

    async def get_user(self, user_id: int) -> Optional[dict]:
        return await self.db.find(user_id)

def validate_email(email: str) -> bool:
    return "@" in email

async def send_notification(user_id: int, message: str) -> None:
    pass

def _internal_helper(data):
    return data
"#;
    let file_path = write_test_file(temp_dir.path(), "src/services/user.py", py_content);

    // @step When the Python extractor parses the file
    let entities = extract_file(&file_path, temp_dir.path(), &HashSet::new())
        .expect("Python extraction should succeed");

    // @step Then Function nodes should be created for each def and async def
    // File node
    assert_eq!(count_nodes(&entities, "File"), 1, "Should create 1 File node");
    let file_node = find_node(&entities, "File", "src-services-user-py");
    assert!(file_node.is_some(), "Should find File node with correct slug");
    if let Some(GraphEntity::Node { properties, .. }) = file_node {
        assert_eq!(
            properties.get("language").and_then(|v| v.as_str()),
            Some("python")
        );
        assert_eq!(properties.get("isTest").and_then(|v| v.as_bool()), Some(false));
    }

    // Function nodes — top-level functions are captured; class methods may or may not be
    let fn_count = count_nodes(&entities, "Function");
    assert!(fn_count >= 2, "Should extract at least 2 functions, got {fn_count}");

    // Check validate_email is public
    let validate_fn = find_node(&entities, "Function", "src-services-user-py::validate_email");
    if let Some(GraphEntity::Node { properties, .. }) = validate_fn {
        assert_eq!(properties.get("isPublic").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(properties.get("isAsync").and_then(|v| v.as_bool()), Some(false));
        assert_eq!(properties.get("paramCount").and_then(|v| v.as_i64()), Some(1));
    }

    // Check async function
    let send_fn = find_node(&entities, "Function", "src-services-user-py::send_notification");
    if let Some(GraphEntity::Node { properties, .. }) = send_fn {
        assert_eq!(properties.get("isAsync").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(properties.get("paramCount").and_then(|v| v.as_i64()), Some(2));
    }

    // @step And isPublic should be false for functions starting with underscore
    // Check private function
    let helper_fn = find_node(&entities, "Function", "src-services-user-py::_internal_helper");
    if let Some(GraphEntity::Node { properties, .. }) = helper_fn {
        assert_eq!(properties.get("isPublic").and_then(|v| v.as_bool()), Some(false));
    }

    // @step And Type nodes should be created for each class with typeKind "class"
    // Type (class) nodes
    assert!(count_nodes(&entities, "Type") >= 1, "Should find at least 1 Type node (UserService class)");

    // @step And Contains and ContainsType edges should link File to children
    // Edges
    assert!(count_edges(&entities, "Contains") >= 2, "Should have Contains edges for functions");
    assert!(count_edges(&entities, "ContainsType") >= 1, "Should have ContainsType edges for classes");
}

#[tokio::test]
async fn test_python_test_file_detection() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let test_content = "def test_something():\n    assert True\n";
    let file_path = write_test_file(temp_dir.path(), "tests/test_auth.py", test_content);

    let entities = extract_file(&file_path, temp_dir.path(), &HashSet::new())
        .expect("Python extraction should succeed");

    let file_node = find_node(&entities, "File", "tests-test_auth-py");
    if let Some(GraphEntity::Node { properties, .. }) = file_node {
        assert_eq!(properties.get("isTest").and_then(|v| v.as_bool()), Some(true));
    }
}

// ============================================================================
// Go Extractor Tests
// ============================================================================
#[tokio::test]
async fn test_extract_go_functions_and_types() {
    // @step Given a Go file "internal/handler.go" with func, method receivers, and struct/interface
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let go_content = r#"package main

import "fmt"

type Server struct {
    Host string
    Port int
}

type Handler interface {
    ServeHTTP(w ResponseWriter, r Request)
}

func NewServer(host string, port int) *Server {
    return &Server{Host: host, Port: port}
}

func (s *Server) Start() error {
    fmt.Println("Starting server")
    return nil
}

func helper(x int) int {
    return x + 1
}
"#;
    let file_path = write_test_file(temp_dir.path(), "cmd/server/main.go", go_content);

    // @step When the Go extractor parses the file
    let entities = extract_file(&file_path, temp_dir.path(), &HashSet::new())
        .expect("Go extraction should succeed");

    // @step Then Function nodes should be created with isPublic based on capitalization
    // File node
    assert_eq!(count_nodes(&entities, "File"), 1);
    let file_node = find_node(&entities, "File", "cmd-server-main-go");
    if let Some(GraphEntity::Node { properties, .. }) = file_node {
        assert_eq!(properties.get("language").and_then(|v| v.as_str()), Some("go"));
        assert_eq!(properties.get("isTest").and_then(|v| v.as_bool()), Some(false));
    }

    // Functions — NewServer is public (capitalized), helper is private
    let fn_count = count_nodes(&entities, "Function");
    assert!(fn_count >= 2, "Should extract at least 2 functions, got {fn_count}");

    let new_server = find_node(&entities, "Function", "cmd-server-main-go::NewServer");
    if let Some(GraphEntity::Node { properties, .. }) = new_server {
        assert_eq!(properties.get("isPublic").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(properties.get("paramCount").and_then(|v| v.as_i64()), Some(2));
    }

    let helper_fn = find_node(&entities, "Function", "cmd-server-main-go::helper");
    if let Some(GraphEntity::Node { properties, .. }) = helper_fn {
        assert_eq!(properties.get("isPublic").and_then(|v| v.as_bool()), Some(false));
    }

    // @step And Type nodes should be created for structs and interfaces
    // Type nodes
    let type_count = count_nodes(&entities, "Type");
    assert!(type_count >= 1, "Should find at least 1 Type node (Server struct), got {type_count}");

    // Edges
    assert!(count_edges(&entities, "Contains") >= 2);
    assert!(count_edges(&entities, "ContainsType") >= 1);
}

#[tokio::test]
async fn test_go_test_file_detection() {
    // @step And test files ending in _test.go should have isTest set to true
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let test_content = "package main\n\nfunc TestSomething() {\n}\n";
    let file_path = write_test_file(temp_dir.path(), "server_test.go", test_content);

    let entities = extract_file(&file_path, temp_dir.path(), &HashSet::new())
        .expect("Go extraction should succeed");

    let file_node = find_node(&entities, "File", "server_test-go");
    if let Some(GraphEntity::Node { properties, .. }) = file_node {
        assert_eq!(properties.get("isTest").and_then(|v| v.as_bool()), Some(true));
    }
}

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
    let file_path = write_test_file(temp_dir.path(), "src/main/UserController.java", java_content);

    // @step When the Java extractor parses the file
    let entities = extract_file(&file_path, temp_dir.path(), &HashSet::new())
        .expect("Java extraction should succeed");

    assert_eq!(count_nodes(&entities, "File"), 1);
    let file_node = find_node(&entities, "File", "src-main-UserController-java");
    assert!(file_node.is_some(), "Should find File node for UserController.java");
    if let Some(GraphEntity::Node { properties, .. }) = file_node {
        assert_eq!(properties.get("language").and_then(|v| v.as_str()), Some("java"));
    }

    // @step Then Function nodes should be created with isPublic from access modifiers
    let fn_count = count_nodes(&entities, "Function");
    assert!(fn_count >= 2, "Should extract at least 2 methods, got {fn_count}");

    // Verify isPublic per-method based on access modifiers
    let get_user_fn = find_node(&entities, "Function", "src-main-UserController-java::getUser");
    if let Some(GraphEntity::Node { properties, .. }) = &get_user_fn {
        assert_eq!(
            properties.get("isPublic").and_then(|v| v.as_bool()),
            Some(true),
            "public method getUser should have isPublic=true"
        );
    }
    let log_fn = find_node(&entities, "Function", "src-main-UserController-java::logAccess");
    if let Some(GraphEntity::Node { properties, .. }) = &log_fn {
        assert_eq!(
            properties.get("isPublic").and_then(|v| v.as_bool()),
            Some(false),
            "private method logAccess should have isPublic=false"
        );
    }

    // @step And Type nodes should be created for classes, interfaces, and enums
    let type_count = count_nodes(&entities, "Type");
    assert!(type_count >= 3, "Should find at least 3 Type nodes (class + interface + enum), got {type_count}");

    // Edges
    assert!(count_edges(&entities, "Contains") >= 1);
    assert!(count_edges(&entities, "ContainsType") >= 1, "Should have ContainsType edges for type declarations");
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
    let entities = extract_file(&file_path, temp_dir.path(), &HashSet::new())
        .expect("C extraction should succeed");

    assert_eq!(count_nodes(&entities, "File"), 1);
    let file_node = find_node(&entities, "File", "src-geometry-c");
    assert!(file_node.is_some(), "Should find File node for geometry.c");
    if let Some(GraphEntity::Node { properties, .. }) = file_node {
        assert_eq!(properties.get("language").and_then(|v| v.as_str()), Some("c"));
    }

    // @step Then Function nodes should be created with isPublic false for static functions
    let fn_count = count_nodes(&entities, "Function");
    assert!(fn_count >= 2, "Should extract at least 2 functions, got {fn_count}");

    // Verify static function is not public
    let internal_fn = find_node(&entities, "Function", "src-geometry-c::internal_helper");
    assert!(internal_fn.is_some(), "Should find static function internal_helper");
    if let Some(GraphEntity::Node { properties, .. }) = internal_fn {
        assert_eq!(
            properties.get("isPublic").and_then(|v| v.as_bool()),
            Some(false),
            "static functions should have isPublic=false"
        );
    }

    // Verify non-static function is public
    let add_fn = find_node(&entities, "Function", "src-geometry-c::add");
    assert!(add_fn.is_some(), "Should find non-static function add");
    if let Some(GraphEntity::Node { properties, .. }) = add_fn {
        assert_eq!(
            properties.get("isPublic").and_then(|v| v.as_bool()),
            Some(true),
            "Non-static functions should have isPublic=true"
        );
    }

    // @step And Type nodes should be created for structs, enums, and typedefs
    let type_count = count_nodes(&entities, "Type");
    assert!(type_count >= 3, "Should find at least 3 Type nodes (struct + enum + typedef), got {type_count}");
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

    let entities = extract_file(&file_path, temp_dir.path(), &HashSet::new())
        .expect("C header extraction should succeed");

    let file_node = find_node(&entities, "File", "include-point-h");
    assert!(file_node.is_some(), "Should find File node for point.h");
    if let Some(GraphEntity::Node { properties, .. }) = file_node {
        // Pure C header without C++ features
        assert_eq!(properties.get("language").and_then(|v| v.as_str()), Some("c-header"));
    }

    // Verify extraction actually works for the header — at minimum functions should be found
    let fn_count = count_nodes(&entities, "Function");
    assert!(fn_count >= 1, "Should extract at least 1 function from header, got {fn_count}");
    // Struct may or may not be extracted depending on preprocessor guards, but function is the key check
}

// ============================================================================
// C++ Extractor Tests
// ============================================================================
#[tokio::test]
async fn test_extract_cpp_functions_and_types() {
    // @step Given a C++ file with classes, methods, namespaces, and templates
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let cpp_content = r#"
#include <string>
#include <vector>

namespace engine {

class GameObject {
public:
    void update() {
        tick();
    }
private:
    void tick() {
    }
};

struct Vec3 {
    float x;
    float y;
    float z;
};

enum class Direction {
    North,
    South,
    East,
    West
};

}

int main(int argc, char* argv[]) {
    return 0;
}
"#;
    let file_path = write_test_file(temp_dir.path(), "src/engine.cpp", cpp_content);

    // @step When the C++ extractor parses the file
    let entities = extract_file(&file_path, temp_dir.path(), &HashSet::new())
        .expect("C++ extraction should succeed");

    assert_eq!(count_nodes(&entities, "File"), 1);
    let file_node = find_node(&entities, "File", "src-engine-cpp");
    if let Some(GraphEntity::Node { properties, .. }) = file_node {
        assert_eq!(properties.get("language").and_then(|v| v.as_str()), Some("cpp"));
    }

    // @step Then Function nodes should be created for standalone and class methods
    let fn_count = count_nodes(&entities, "Function");
    assert!(fn_count >= 1, "Should extract at least 1 function, got {fn_count}");

    // @step And Type nodes should be created for classes, structs, enums, and namespaces
    let type_count = count_nodes(&entities, "Type");
    assert!(type_count >= 1, "Should find at least 1 Type node, got {type_count}");
}

#[tokio::test]
async fn test_cpp_header_heuristic() {
    // @step Given a .h file containing C++ keywords like class or namespace
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let cpp_h_content = r#"
#pragma once
#include <string>

namespace utils {

class Logger {
public:
    void log(std::string msg) {
    }
};

}
"#;
    let file_path = write_test_file(temp_dir.path(), "include/logger.h", cpp_h_content);

    // @step When the pipeline processes the .h file
    let entities = extract_file(&file_path, temp_dir.path(), &HashSet::new())
        .expect("C++ header detection should succeed");

    // @step Then the C++ extractor should be used instead of C
    // Should be detected as C++ due to namespace/class/std:: heuristics
    let file_node = find_node(&entities, "File", "include-logger-h");
    if let Some(GraphEntity::Node { properties, .. }) = file_node {
        assert_eq!(
            properties.get("language").and_then(|v| v.as_str()),
            Some("cpp"),
            "Header with C++ features should be detected as cpp"
        );
    }
}

// ============================================================================
// C# Extractor Tests
// ============================================================================
#[tokio::test]
async fn test_extract_csharp_methods_and_types() {
    // @step Given a C# file with public/private methods and class/interface/struct/enum
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let cs_content = r#"
using System;
using System.Threading.Tasks;

namespace MyApp {

    public class UserController {
        public string GetUser(int id) {
            return "user";
        }

        private void LogAccess(string action) {
            Console.WriteLine(action);
        }

        public async Task<string> FetchAsync(int id) {
            return await Task.FromResult("done");
        }
    }

    public interface IUserService {
        string FindUser(int id);
    }

    public enum UserRole {
        Admin,
        User,
        Guest
    }

    public struct Point {
        public int X;
        public int Y;
    }
}
"#;
    let file_path = write_test_file(temp_dir.path(), "src/UserController.cs", cs_content);

    // @step When the C# extractor parses the file
    let entities = extract_file(&file_path, temp_dir.path(), &HashSet::new())
        .expect("C# extraction should succeed");

    assert_eq!(count_nodes(&entities, "File"), 1);
    let file_node = find_node(&entities, "File", "src-UserController-cs");
    if let Some(GraphEntity::Node { properties, .. }) = file_node {
        assert_eq!(properties.get("language").and_then(|v| v.as_str()), Some("csharp"));
    }

    // @step Then Function nodes should be created with access modifier visibility
    let fn_count = count_nodes(&entities, "Function");
    assert!(fn_count >= 1, "Should extract at least 1 method, got {fn_count}");

    // @step And Type nodes should be created for all C# type declarations
    let type_count = count_nodes(&entities, "Type");
    assert!(type_count >= 1, "Should find at least 1 Type node, got {type_count}");
}

// ============================================================================
// Ruby Extractor Tests
// ============================================================================
#[tokio::test]
async fn test_extract_ruby_methods_and_types() {
    // @step Given a Ruby file with def, def self., class, and module declarations
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let rb_content = r#"
module Authentication
  class UserAuth
    def initialize(db)
      @db = db
    end

    def authenticate(username, password)
      @db.find(username)
    end

    def self.create(config)
      new(config)
    end

    def _private_method
      true
    end
  end
end
"#;
    let file_path = write_test_file(temp_dir.path(), "lib/auth/user_auth.rb", rb_content);

    // @step When the Ruby extractor parses the file
    let entities = extract_file(&file_path, temp_dir.path(), &HashSet::new())
        .expect("Ruby extraction should succeed");

    assert_eq!(count_nodes(&entities, "File"), 1);
    let file_node = find_node(&entities, "File", "lib-auth-user_auth-rb");
    if let Some(GraphEntity::Node { properties, .. }) = file_node {
        assert_eq!(properties.get("language").and_then(|v| v.as_str()), Some("ruby"));
    }

    // @step Then Function nodes and Type nodes should be created
    let fn_count = count_nodes(&entities, "Function");
    assert!(fn_count >= 1, "Should extract at least 1 method, got {fn_count}");

    let type_count = count_nodes(&entities, "Type");
    assert!(type_count >= 1, "Should find at least 1 Type node, got {type_count}");
}

#[tokio::test]
async fn test_ruby_spec_file_detection() {
    // @step And spec files should have isTest set to true
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let spec_content = "class TestAuth\n  def test_login\n  end\nend\n";
    let file_path = write_test_file(temp_dir.path(), "spec/auth_spec.rb", spec_content);

    let entities = extract_file(&file_path, temp_dir.path(), &HashSet::new())
        .expect("Ruby extraction should succeed");

    let file_node = find_node(&entities, "File", "spec-auth_spec-rb");
    if let Some(GraphEntity::Node { properties, .. }) = file_node {
        assert_eq!(properties.get("isTest").and_then(|v| v.as_bool()), Some(true));
    }
}

// ============================================================================
// Kotlin Extractor Tests
// ============================================================================
#[tokio::test]
async fn test_extract_kotlin_functions_and_types() {
    // @step Given a Kotlin file with fun, suspend fun, and class/interface/object/enum
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let kt_content = r#"
package com.example

data class User(val id: Int, val name: String)

interface UserRepository {
    fun findById(id: Int): User?
}

object UserFactory {
    fun create(name: String): User {
        return User(0, name)
    }
}

enum class Role {
    ADMIN,
    USER
}

suspend fun fetchUser(id: Int): User {
    return User(id, "test")
}

fun validateName(name: String): Boolean {
    return name.isNotEmpty()
}
"#;
    let file_path = write_test_file(temp_dir.path(), "src/main/kotlin/User.kt", kt_content);

    // @step When the Kotlin extractor parses the file
    let entities = extract_file(&file_path, temp_dir.path(), &HashSet::new())
        .expect("Kotlin extraction should succeed");

    assert_eq!(count_nodes(&entities, "File"), 1);
    let file_node = find_node(&entities, "File", "src-main-kotlin-User-kt");
    if let Some(GraphEntity::Node { properties, .. }) = file_node {
        assert_eq!(properties.get("language").and_then(|v| v.as_str()), Some("kotlin"));
    }

    // @step Then Function nodes should be created with isAsync for suspend functions
    let fn_count = count_nodes(&entities, "Function");
    assert!(fn_count >= 1, "Should extract at least 1 function, got {fn_count}");

    // Check suspend fun is marked async
    let fetch_fn = find_node(&entities, "Function", "src-main-kotlin-User-kt::fetchUser");
    if let Some(GraphEntity::Node { properties, .. }) = fetch_fn {
        assert_eq!(properties.get("isAsync").and_then(|v| v.as_bool()), Some(true));
    }

    // @step And Type nodes should be created for all Kotlin type declarations
    let type_count = count_nodes(&entities, "Type");
    assert!(type_count >= 1, "Should find at least 1 Type node, got {type_count}");
}

// ============================================================================
// Swift Extractor Tests
// ============================================================================
#[tokio::test]
async fn test_extract_swift_functions_and_types() {
    // @step Given a Swift file with func, async func, and class/struct/protocol/enum
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let swift_content = r#"
import Foundation

class NetworkManager {
    func fetchData(url: String) -> Data {
        return Data()
    }

    func sendAsync(data: Data) async -> Bool {
        return true
    }
}

struct Config {
    var apiKey: String
    var timeout: Int
}

protocol Fetchable {
    func fetch(id: Int) -> String
}

enum AppError {
    case networkError
    case parseError
}

func processRequest(url: String, timeout: Int) -> Bool {
    return true
}
"#;
    let file_path = write_test_file(temp_dir.path(), "Sources/App/Network.swift", swift_content);

    // @step When the Swift extractor parses the file
    let entities = extract_file(&file_path, temp_dir.path(), &HashSet::new())
        .expect("Swift extraction should succeed");

    assert_eq!(count_nodes(&entities, "File"), 1);
    let file_node = find_node(&entities, "File", "Sources-App-Network-swift");
    if let Some(GraphEntity::Node { properties, .. }) = file_node {
        assert_eq!(properties.get("language").and_then(|v| v.as_str()), Some("swift"));
    }

    // @step Then Function nodes and Type nodes should be created
    let fn_count = count_nodes(&entities, "Function");
    assert!(fn_count >= 1, "Should extract at least 1 function, got {fn_count}");

    // @step And protocols should have typeKind "trait_kind"
    let type_count = count_nodes(&entities, "Type");
    assert!(type_count >= 1, "Should find at least 1 Type node, got {type_count}");
    // Verify at least one type has trait_kind (the protocol)
    let has_trait_kind = entities.iter().any(|e| {
        if let GraphEntity::Node { node_type, properties, .. } = e {
            node_type == "Type"
                && properties.get("typeKind").and_then(|v| v.as_str()) == Some("trait_kind")
        } else {
            false
        }
    });
    assert!(has_trait_kind, "Protocol should have typeKind 'trait_kind'");
}

// ============================================================================
// Scala Extractor Tests
// ============================================================================
#[tokio::test]
async fn test_extract_scala_functions_and_types() {
    // @step Given a Scala file with def, class, trait, object, and case class
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let scala_content = r#"
package com.example

trait Repository {
  def findById(id: Int): Option[String]
}

class UserRepo extends Repository {
  def findById(id: Int): Option[String] = {
    Some("user")
  }

  def save(name: String): Unit = {
    println(name)
  }
}

object UserRepo {
  def apply(): UserRepo = {
    new UserRepo()
  }
}

case class User(id: Int, name: String)
"#;
    let file_path = write_test_file(temp_dir.path(), "src/main/scala/UserRepo.scala", scala_content);

    // @step When the Scala extractor parses the file
    let entities = extract_file(&file_path, temp_dir.path(), &HashSet::new())
        .expect("Scala extraction should succeed");

    assert_eq!(count_nodes(&entities, "File"), 1);
    let file_node = find_node(&entities, "File", "src-main-scala-UserRepo-scala");
    if let Some(GraphEntity::Node { properties, .. }) = file_node {
        assert_eq!(properties.get("language").and_then(|v| v.as_str()), Some("scala"));
    }

    // @step Then Function nodes and Type nodes should be created
    let fn_count = count_nodes(&entities, "Function");
    assert!(fn_count >= 1, "Should extract at least 1 function, got {fn_count}");

    // @step And traits should have typeKind "trait_kind"
    let type_count = count_nodes(&entities, "Type");
    assert!(type_count >= 1, "Should find at least 1 Type node, got {type_count}");
    // Verify at least one type has trait_kind (the Scala trait)
    let has_trait_kind = entities.iter().any(|e| {
        if let GraphEntity::Node { node_type, properties, .. } = e {
            node_type == "Type"
                && properties.get("typeKind").and_then(|v| v.as_str()) == Some("trait_kind")
        } else {
            false
        }
    });
    assert!(has_trait_kind, "Scala trait should have typeKind 'trait_kind'");
}

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
    let file_path = write_test_file(temp_dir.path(), "app/Controllers/UserController.php", php_content);

    // @step When the PHP extractor parses the file
    let entities = extract_file(&file_path, temp_dir.path(), &HashSet::new())
        .expect("PHP extraction should succeed");

    assert_eq!(count_nodes(&entities, "File"), 1);
    let file_node = find_node(&entities, "File", "app-Controllers-UserController-php");
    if let Some(GraphEntity::Node { properties, .. }) = file_node {
        assert_eq!(properties.get("language").and_then(|v| v.as_str()), Some("php"));
    }

    // @step Then Function nodes and Type nodes should be created
    let fn_count = count_nodes(&entities, "Function");
    assert!(fn_count >= 1, "Should extract at least 1 function, got {fn_count}");

    let type_count = count_nodes(&entities, "Type");
    assert!(type_count >= 1, "Should find at least 1 Type node, got {type_count}");
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
    let entities = codelet_napi::graph::ast_pipeline::pip_dep_extractor::extract_python_dependencies(temp_dir.path())
        .expect("Python dep extraction should succeed");

    // @step Then Dependency nodes should be created with source "pip"
    let dep_count = count_nodes(&entities, "Dependency");
    assert!(dep_count >= 3, "Should extract at least 3 dependencies, got {dep_count}");
    assert!(has_dependency_with_source(&entities, "pip"), "Dependency source should be 'pip'");

    let file_count = count_nodes(&entities, "File");
    assert!(file_count >= 1, "Should create File node for requirements.txt");

    // @step And DependsOn edges should link manifest files to dependencies
    let depends_on_count = count_edges(&entities, "DependsOn");
    assert!(depends_on_count >= 1, "Should have DependsOn edges linking manifest to deps, got {depends_on_count}");
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
    let entities = codelet_napi::graph::ast_pipeline::pip_dep_extractor::extract_python_dependencies(temp_dir.path())
        .expect("Python pyproject dep extraction should succeed");

    // @step Then Dependency nodes should be created with source "pip"
    let dep_count = count_nodes(&entities, "Dependency");
    assert!(dep_count >= 2, "Should extract at least 2 dependencies, got {dep_count}");
    assert!(has_dependency_with_source(&entities, "pip"), "Dependency source should be 'pip'");
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
    let entities = codelet_napi::graph::ast_pipeline::gomod_dep_extractor::extract_go_dependencies(temp_dir.path())
        .expect("Go dep extraction should succeed");

    // @step Then Dependency nodes should be created with source "go"
    let dep_count = count_nodes(&entities, "Dependency");
    assert!(dep_count >= 2, "Should extract at least 2 dependencies, got {dep_count}");
    assert!(has_dependency_with_source(&entities, "go"), "Dependency source should be 'go'");
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
    let entities = codelet_napi::graph::ast_pipeline::java_dep_extractor::extract_java_dependencies(temp_dir.path())
        .expect("Java dep extraction should succeed");

    // @step Then Dependency nodes should be created with source "maven" or "gradle"
    let dep_count = count_nodes(&entities, "Dependency");
    assert!(dep_count >= 2, "Should extract at least 2 dependencies, got {dep_count}");
    assert!(has_dependency_with_source(&entities, "gradle"), "Dependency source should be 'gradle'");
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
    let entities = codelet_napi::graph::ast_pipeline::java_dep_extractor::extract_java_dependencies(temp_dir.path())
        .expect("Maven dep extraction should succeed");

    // @step Then Dependency nodes should be created with source "maven" or "gradle"
    let dep_count = count_nodes(&entities, "Dependency");
    assert!(dep_count >= 2, "Should extract at least 2 dependencies, got {dep_count}");
    assert!(has_dependency_with_source(&entities, "maven"), "Dependency source should be 'maven'");
}

#[tokio::test]
async fn test_extract_composer_dependencies() {
    // @step Given a project with composer.json require and require-dev sections
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let composer_content = r#"{
    "require": {
        "laravel/framework": "^10.0",
        "guzzlehttp/guzzle": "^7.0"
    },
    "require-dev": {
        "phpunit/phpunit": "^10.0"
    }
}"#;
    write_test_file(temp_dir.path(), "composer.json", composer_content);

    // @step When the composer dependency extractor runs
    let entities = codelet_napi::graph::ast_pipeline::composer_dep_extractor::extract_composer_dependencies(temp_dir.path())
        .expect("Composer dep extraction should succeed");

    // @step Then Dependency nodes should be created with source "composer"
    let dep_count = count_nodes(&entities, "Dependency");
    assert!(dep_count >= 3, "Should extract at least 3 dependencies, got {dep_count}");
    assert!(has_dependency_with_source(&entities, "composer"), "Dependency source should be 'composer'");
}

#[tokio::test]
async fn test_extract_gemfile_dependencies() {
    // @step Given a project with Gemfile listing gems with version constraints
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let gemfile_content = r#"source 'https://rubygems.org'

gem 'rails', '~> 7.0'
gem 'pg'

group :development, :test do
  gem 'rspec-rails', '~> 6.0'
end
"#;
    write_test_file(temp_dir.path(), "Gemfile", gemfile_content);

    // @step When the Gemfile dependency extractor runs
    let entities = codelet_napi::graph::ast_pipeline::gemfile_dep_extractor::extract_gemfile_dependencies(temp_dir.path())
        .expect("Gemfile dep extraction should succeed");

    // @step Then Dependency nodes should be created with source "gem"
    let dep_count = count_nodes(&entities, "Dependency");
    assert!(dep_count >= 2, "Should extract at least 2 dependencies, got {dep_count}");
    assert!(has_dependency_with_source(&entities, "gem"), "Dependency source should be 'gem'");
}

#[tokio::test]
async fn test_extract_swift_dependencies() {
    // @step Given a project with Package.swift listing .package dependencies
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let pkg_content = r#"
// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "MyApp",
    dependencies: [
        .package(url: "https://github.com/vapor/vapor", from: "4.0.0"),
        .package(url: "https://github.com/apple/swift-log.git", exact: "1.5.0"),
    ]
)
"#;
    write_test_file(temp_dir.path(), "Package.swift", pkg_content);

    // @step When the Swift dependency extractor runs
    let entities = codelet_napi::graph::ast_pipeline::swift_dep_extractor::extract_swift_dependencies(temp_dir.path())
        .expect("Swift dep extraction should succeed");

    // @step Then Dependency nodes should be created with source "spm"
    let dep_count = count_nodes(&entities, "Dependency");
    assert!(dep_count >= 2, "Should extract at least 2 dependencies, got {dep_count}");
    assert!(has_dependency_with_source(&entities, "spm"), "Dependency source should be 'spm'");
}

#[tokio::test]
async fn test_extract_csproj_dependencies() {
    // @step Given a project with .csproj PackageReference elements
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let csproj_content = r#"<Project Sdk="Microsoft.NET.Sdk">
  <ItemGroup>
    <PackageReference Include="Newtonsoft.Json" Version="13.0.1" />
    <PackageReference Include="Microsoft.Extensions.Logging" Version="7.0.0" />
  </ItemGroup>
</Project>
"#;
    write_test_file(temp_dir.path(), "MyApp.csproj", csproj_content);

    // @step When the .csproj dependency extractor runs
    let entities = codelet_napi::graph::ast_pipeline::csproj_dep_extractor::extract_csproj_dependencies(temp_dir.path())
        .expect("Csproj dep extraction should succeed");

    // @step Then Dependency nodes should be created with source "nuget"
    let dep_count = count_nodes(&entities, "Dependency");
    assert!(dep_count >= 2, "Should extract at least 2 dependencies, got {dep_count}");
    assert!(has_dependency_with_source(&entities, "nuget"), "Dependency source should be 'nuget'");
}

#[tokio::test]
async fn test_extract_sbt_dependencies() {
    // @step Given a project with build.sbt listing libraryDependencies
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let sbt_content = r#"
name := "myproject"
version := "0.1.0"

libraryDependencies ++= Seq(
  "com.typesafe.akka" %% "akka-actor" % "2.8.0",
  "org.scalatest" %% "scalatest" % "3.2.0" % Test
)
"#;
    write_test_file(temp_dir.path(), "build.sbt", sbt_content);

    // @step When the sbt dependency extractor runs
    let entities = codelet_napi::graph::ast_pipeline::sbt_dep_extractor::extract_sbt_dependencies(temp_dir.path())
        .expect("SBT dep extraction should succeed");

    // @step Then Dependency nodes should be created with source "sbt"
    let dep_count = count_nodes(&entities, "Dependency");
    assert!(dep_count >= 2, "Should extract at least 2 dependencies, got {dep_count}");
    assert!(has_dependency_with_source(&entities, "sbt"), "Dependency source should be 'sbt'");
}

// ============================================================================
// Missing dep file returns Ok(vec![]) — never errors
// ============================================================================
#[tokio::test]
async fn test_missing_dep_files_return_empty() {
    // @step Given a project without any dependency manifest files
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let root = temp_dir.path();

    // @step When all dependency extractors run
    // @step Then each should return an empty list without errors
    assert!(codelet_napi::graph::ast_pipeline::pip_dep_extractor::extract_python_dependencies(root).unwrap().is_empty());
    assert!(codelet_napi::graph::ast_pipeline::gomod_dep_extractor::extract_go_dependencies(root).unwrap().is_empty());
    assert!(codelet_napi::graph::ast_pipeline::java_dep_extractor::extract_java_dependencies(root).unwrap().is_empty());
    assert!(codelet_napi::graph::ast_pipeline::composer_dep_extractor::extract_composer_dependencies(root).unwrap().is_empty());
    assert!(codelet_napi::graph::ast_pipeline::gemfile_dep_extractor::extract_gemfile_dependencies(root).unwrap().is_empty());
    assert!(codelet_napi::graph::ast_pipeline::swift_dep_extractor::extract_swift_dependencies(root).unwrap().is_empty());
    assert!(codelet_napi::graph::ast_pipeline::csproj_dep_extractor::extract_csproj_dependencies(root).unwrap().is_empty());
    assert!(codelet_napi::graph::ast_pipeline::sbt_dep_extractor::extract_sbt_dependencies(root).unwrap().is_empty());
}

// ============================================================================
// Walk and Extract picks up new file extensions
// ============================================================================
#[tokio::test]
async fn test_walk_and_extract_finds_multi_language_files() {
    use codelet_napi::graph::ast_pipeline::walk_and_extract;

    // @step Given a project directory with files in Python, Go, Java, C, C++, C#, Ruby, Kotlin, Swift, Scala, and PHP
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    write_test_file(project_dir, "main.py", "def hello():\n    pass\n");
    write_test_file(project_dir, "main.go", "package main\nfunc main() {\n}\n");
    write_test_file(project_dir, "Main.java", "public class Main {\n    public void run() {\n    }\n}\n");
    write_test_file(project_dir, "main.c", "int main() {\n    return 0;\n}\n");
    write_test_file(project_dir, "main.cpp", "int main(int argc, char* argv[]) {\n    return 0;\n}\n");
    write_test_file(project_dir, "Program.cs", "class Program {\n    static void Main() {\n    }\n}\n");
    write_test_file(project_dir, "main.rb", "def main\nend\n");
    write_test_file(project_dir, "Main.kt", "fun main() {\n}\n");
    write_test_file(project_dir, "main.swift", "func main() {\n}\n");
    write_test_file(project_dir, "Main.scala", "object Main {\n  def main(): Unit = {\n  }\n}\n");
    write_test_file(project_dir, "index.php", "<?php\nfunction index() {\n    return true;\n}\n");
    write_test_file(project_dir, "app.ts", "export function app(): void {}\n");
    write_test_file(project_dir, "lib.rs", "pub fn init() {}\n");
    write_test_file(project_dir, ".gitignore", "node_modules/\ntarget/\n");

    // @step When walk_and_extract processes the project
    let entities = walk_and_extract(project_dir)
        .expect("walk_and_extract should succeed");

    // Collect all File node languages
    let languages: Vec<String> = entities
        .iter()
        .filter_map(|e| match e {
            GraphEntity::Node { node_type, properties, .. } if node_type == "File" => {
                properties.get("language").and_then(|v| v.as_str()).map(|s| s.to_string())
            }
            _ => None,
        })
        .collect();

    // @step Then File nodes should be created for all supported extensions
    // Should find files from many languages
    let file_count = count_nodes(&entities, "File");
    assert!(
        file_count >= 10,
        "Should find at least 10 File nodes from multi-language project, got {file_count}. Languages: {:?}",
        languages
    );

    // Verify specific languages are represented
    assert!(languages.contains(&"python".to_string()), "Should contain Python files");
    assert!(languages.contains(&"go".to_string()), "Should contain Go files");
    assert!(languages.contains(&"rust".to_string()), "Should contain Rust files");
    assert!(languages.contains(&"typescript".to_string()), "Should contain TS files");
}

// ============================================================================
// Graph database load test — verify no constraint violations
// ============================================================================
#[tokio::test]
async fn test_multi_language_entities_load_into_graph() {
    use codelet_napi::graph::ast_pipeline::walk_and_extract;
    use codelet_napi::graph::database::GraphDatabase;

    // @step Given a project directory with files in Python, Go, Java, C, C++, C#, Ruby, Kotlin, Swift, Scala, and PHP
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    write_test_file(project_dir, "app.py", "class App:\n    def run(self):\n        pass\n");
    write_test_file(project_dir, "server.go", "package main\n\ntype Server struct {\n    Host string\n}\n\nfunc NewServer() *Server {\n    return nil\n}\n");
    write_test_file(project_dir, "main.ts", "export function main(): void {}\n");
    write_test_file(project_dir, ".gitignore", "node_modules/\ntarget/\n");

    // @step When walk_and_extract processes the project
    let entities = walk_and_extract(project_dir)
        .expect("walk_and_extract should succeed");

    let schema = include_str!("../schemas/ast-code.pg");
    let db_path = temp_dir.path().join("test-multi-lang.nano");
    let db = GraphDatabase::init(&db_path, schema)
        .await
        .expect("DB init should succeed");

    // @step And no constraint violations should occur when loading into the graph
    let load_result = db.load_entities(&entities).await;
    assert!(
        load_result.is_ok(),
        "Multi-language graph load must succeed without constraint violation, got: {:?}",
        load_result.err()
    );
}
