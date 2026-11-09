// Feature: spec/features/multi-language-ast-extraction.feature
use super::*;

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
    let entities = index_file(&file_path, temp_dir.path());

    // @step Then Function nodes should be created for each def and async def
    // File node
    assert_eq!(
        count_nodes(&entities, "File"),
        1,
        "Should create 1 File node"
    );
    let file_node = find_node(&entities, "File", "src/services/user.py");
    assert!(
        file_node.is_some(),
        "Should find File node with correct slug"
    );
    let Some(GraphEntity::Node { properties, .. }) = file_node else {
        panic!("{} not found", stringify!(file_node));
    };
    assert_eq!(
        properties.get("language").and_then(|v| v.as_str()),
        Some("python")
    );
    assert_eq!(
        properties.get("isTest").and_then(|v| v.as_bool()),
        Some(false)
    );

    // Function nodes — top-level functions are captured; class methods may or may not be
    let fn_count = count_nodes(&entities, "Function");
    assert!(
        fn_count >= 2,
        "Should extract at least 2 functions, got {fn_count}"
    );

    // Check validate_email is public
    let validate_fn = find_node(
        &entities,
        "Function",
        "src/services/user.py::validate_email",
    );
    let Some(GraphEntity::Node { properties, .. }) = validate_fn else {
        panic!("{} not found", stringify!(validate_fn));
    };
    assert_eq!(
        properties.get("isPublic").and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        properties.get("isAsync").and_then(|v| v.as_bool()),
        Some(false)
    );
    assert_eq!(
        properties.get("paramCount").and_then(|v| v.as_i64()),
        Some(1)
    );

    // Check async function
    let send_fn = find_node(
        &entities,
        "Function",
        "src/services/user.py::send_notification",
    );
    let Some(GraphEntity::Node { properties, .. }) = send_fn else {
        panic!("{} not found", stringify!(send_fn));
    };
    assert_eq!(
        properties.get("isAsync").and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        properties.get("paramCount").and_then(|v| v.as_i64()),
        Some(2)
    );

    // @step And isPublic should be false for functions starting with underscore
    // Check private function
    let helper_fn = find_node(
        &entities,
        "Function",
        "src/services/user.py::_internal_helper",
    );
    let Some(GraphEntity::Node { properties, .. }) = helper_fn else {
        panic!("{} not found", stringify!(helper_fn));
    };
    assert_eq!(
        properties.get("isPublic").and_then(|v| v.as_bool()),
        Some(false)
    );

    // @step And Type nodes should be created for each class with typeKind "class"
    // Type (class) nodes
    assert!(
        count_nodes(&entities, "Type") >= 1,
        "Should find at least 1 Type node (UserService class)"
    );

    // @step And Contains and ContainsType edges should link File to children
    // Edges
    assert!(
        count_edges(&entities, "Contains") >= 2,
        "Should have Contains edges for functions"
    );
    assert!(
        count_edges(&entities, "ContainsType") >= 1,
        "Should have ContainsType edges for classes"
    );
}

#[tokio::test]
async fn test_python_test_file_detection() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let test_content = "def test_something():\n    assert True\n";
    let file_path = write_test_file(temp_dir.path(), "tests/test_auth.py", test_content);

    let entities = index_file(&file_path, temp_dir.path());

    let file_node = find_node(&entities, "File", "tests/test_auth.py");
    let Some(GraphEntity::Node { properties, .. }) = file_node else {
        panic!("{} not found", stringify!(file_node));
    };
    assert_eq!(
        properties.get("isTest").and_then(|v| v.as_bool()),
        Some(true)
    );
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
    let entities = index_file(&file_path, temp_dir.path());

    // @step Then Function nodes should be created with isPublic based on capitalization
    // File node
    assert_eq!(count_nodes(&entities, "File"), 1);
    let file_node = find_node(&entities, "File", "cmd/server/main.go");
    let Some(GraphEntity::Node { properties, .. }) = file_node else {
        panic!("{} not found", stringify!(file_node));
    };
    assert_eq!(
        properties.get("language").and_then(|v| v.as_str()),
        Some("go")
    );
    assert_eq!(
        properties.get("isTest").and_then(|v| v.as_bool()),
        Some(false)
    );

    // Functions — NewServer is public (capitalized), helper is private
    let fn_count = count_nodes(&entities, "Function");
    assert!(
        fn_count >= 2,
        "Should extract at least 2 functions, got {fn_count}"
    );

    let new_server = find_node(&entities, "Function", "cmd/server/main.go::NewServer");
    let Some(GraphEntity::Node { properties, .. }) = new_server else {
        panic!("{} not found", stringify!(new_server));
    };
    assert_eq!(
        properties.get("isPublic").and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        properties.get("paramCount").and_then(|v| v.as_i64()),
        Some(2)
    );

    let helper_fn = find_node(&entities, "Function", "cmd/server/main.go::helper");
    let Some(GraphEntity::Node { properties, .. }) = helper_fn else {
        panic!("{} not found", stringify!(helper_fn));
    };
    assert_eq!(
        properties.get("isPublic").and_then(|v| v.as_bool()),
        Some(false)
    );

    // @step And Type nodes should be created for structs and interfaces
    // Type nodes
    let type_count = count_nodes(&entities, "Type");
    assert!(
        type_count >= 1,
        "Should find at least 1 Type node (Server struct), got {type_count}"
    );

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

    let entities = index_file(&file_path, temp_dir.path());

    let file_node = find_node(&entities, "File", "server_test.go");
    let Some(GraphEntity::Node { properties, .. }) = file_node else {
        panic!("{} not found", stringify!(file_node));
    };
    assert_eq!(
        properties.get("isTest").and_then(|v| v.as_bool()),
        Some(true)
    );
}
