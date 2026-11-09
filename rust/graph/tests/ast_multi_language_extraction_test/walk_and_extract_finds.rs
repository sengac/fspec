// Feature: spec/features/multi-language-ast-extraction.feature
use super::*;

// ============================================================================
// Walk and Extract picks up new file extensions
// ============================================================================
#[tokio::test]
async fn test_walk_and_extract_finds_multi_language_files() {
    use codelet_graph::ast_pipeline::walk_and_extract;

    // @step Given a project directory with files in Python, Go, Java, C, C++, C#, Ruby, Kotlin, Swift, Scala, and PHP
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    write_test_file(project_dir, "main.py", "def hello():\n    pass\n");
    write_test_file(project_dir, "main.go", "package main\nfunc main() {\n}\n");
    write_test_file(
        project_dir,
        "Main.java",
        "public class Main {\n    public void run() {\n    }\n}\n",
    );
    write_test_file(project_dir, "main.c", "int main() {\n    return 0;\n}\n");
    write_test_file(
        project_dir,
        "main.cpp",
        "int main(int argc, char* argv[]) {\n    return 0;\n}\n",
    );
    write_test_file(
        project_dir,
        "Program.cs",
        "class Program {\n    static void Main() {\n    }\n}\n",
    );
    write_test_file(project_dir, "main.rb", "def main\nend\n");
    write_test_file(project_dir, "Main.kt", "fun main() {\n}\n");
    write_test_file(project_dir, "main.swift", "func main() {\n}\n");
    write_test_file(
        project_dir,
        "Main.scala",
        "object Main {\n  def main(): Unit = {\n  }\n}\n",
    );
    write_test_file(
        project_dir,
        "index.php",
        "<?php\nfunction index() {\n    return true;\n}\n",
    );
    write_test_file(project_dir, "app.ts", "export function app(): void {}\n");
    write_test_file(project_dir, "lib.rs", "pub fn init() {}\n");
    write_test_file(project_dir, ".gitignore", "node_modules/\ntarget/\n");

    // @step When walk_and_extract processes the project
    let entities = walk_and_extract(project_dir, true);

    // Collect all File node languages
    let languages: Vec<String> = entities
        .iter()
        .filter_map(|e| match e {
            GraphEntity::Node {
                node_type,
                properties,
                ..
            } if node_type == "File" => properties
                .get("language")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
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
    assert!(
        languages.contains(&"python".to_string()),
        "Should contain Python files"
    );
    assert!(
        languages.contains(&"go".to_string()),
        "Should contain Go files"
    );
    assert!(
        languages.contains(&"rust".to_string()),
        "Should contain Rust files"
    );
    assert!(
        languages.contains(&"typescript".to_string()),
        "Should contain TS files"
    );
}

// ============================================================================
// Graph database load test — verify no constraint violations
// ============================================================================
#[tokio::test]
async fn test_multi_language_entities_load_into_graph() {
    use codelet_graph::ast_pipeline::walk_and_extract;
    use codelet_graph::database::GraphDatabase;

    // @step Given a project directory with files in Python, Go, Java, C, C++, C#, Ruby, Kotlin, Swift, Scala, and PHP
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    write_test_file(
        project_dir,
        "app.py",
        "class App:\n    def run(self):\n        pass\n",
    );
    write_test_file(project_dir, "server.go", "package main\n\ntype Server struct {\n    Host string\n}\n\nfunc NewServer() *Server {\n    return nil\n}\n");
    write_test_file(project_dir, "main.ts", "export function main(): void {}\n");
    write_test_file(project_dir, ".gitignore", "node_modules/\ntarget/\n");

    // @step When walk_and_extract processes the project
    let entities = walk_and_extract(project_dir, true);

    let schema = include_str!("../../schemas/ast-code.pg");
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
