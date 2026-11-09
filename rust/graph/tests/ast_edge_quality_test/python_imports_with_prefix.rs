// Feature: spec/features/ast-extractor-edge-quality.feature
use super::*;

/// Tests that Python `from click.core import Command` produces Imports edges
/// when the file is at `src/click/core.py` (not `click/core.py`): the
/// project's import root is src/ (the src layout), so the index of src/
/// resolves `click.core` exactly to click/core.py.
#[test]
fn test_python_imports_with_prefix_directory() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a Python project with files "src/click/__init__.py" and "src/click/core.py"
    let init_source = r#"
from click.core import Command
"#;
    write_test_file(project_dir, "src/click/__init__.py", init_source);

    // @step And "src/click/__init__.py" contains "from click.core import Command"
    let core_source = r#"
class Command:
    def __init__(self, name):
        self.name = name
"#;
    write_test_file(project_dir, "src/click/core.py", core_source);

    // @step When I run the Python AST extractor with the known_files set
    // `click.core` is an absolute module: it resolves from the import root
    // src/ (the src layout), never by suffix matching.
    let indexed = index(&project_dir.join("src"));
    let entities = &indexed.entities;

    // @step Then an Imports edge should exist from "__init__.py" to "core.py"
    assert!(
        has_named_edge(entities, "Imports", "click/__init__.py", "click/core.py"),
        "Should have Imports edge from __init__.py to core.py. All Imports: {:?}",
        named_edges(entities, "Imports")
    );

    // @step And the import_map should contain "Command" mapped to the core.py file slug
    // The name Command is bound to the class of core.py: the import is
    // resolved, nothing is unresolved.
    assert!(
        indexed.unresolved.is_empty(),
        "every reference resolves: {:?}",
        indexed.unresolved
    );
}

#[test]
fn test_python_stdlib_no_imports_edge() {
    // @step Given a Python project with file "src/click/core.py"
    let source = r#"
import os
import sys

def main():
    os.path.exists("/tmp")
"#;

    // @step And "src/click/core.py" contains "import os"
    // @step When I run the Python AST extractor with the known_files set
    let entities = index_one("src/click/core.py", source);

    // @step Then no Imports edge should exist for the "os" import
    // @step And the Imports edge count should be 0
    let import_count = count_edges(&entities, "Imports");
    assert_eq!(
        import_count, 0,
        "stdlib imports should NOT produce Imports edges"
    );
}

/// Tests that Java `import com.myapp.service.UserService;` produces Imports
/// edges even when the file path has Maven-standard directory prefix.
#[test]
fn test_java_imports_with_maven_directory_structure() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a Java project with files "com/myapp/service/UserService.java" and "com/myapp/App.java"
    let app_source = r#"package com.myapp;

import com.myapp.service.UserService;

public class App {
    public void run() {
        UserService svc = new UserService();
    }
}
"#;
    write_test_file(project_dir, "src/main/java/com/myapp/App.java", app_source);

    // @step And "com/myapp/App.java" contains "import com.myapp.service.UserService;"
    let svc_source = r#"package com.myapp.service;

public class UserService {
    public void serve() {}
}
"#;
    write_test_file(
        project_dir,
        "src/main/java/com/myapp/service/UserService.java",
        svc_source,
    );

    // @step When I run the Java AST extractor with the known_files set
    let indexed = index(project_dir);
    let entities = indexed.entities;

    // @step Then an Imports edge should exist from "App.java" to "UserService.java"
    assert!(
        has_named_edge(
            &entities,
            "Imports",
            "src/main/java/com/myapp/App.java",
            "src/main/java/com/myapp/service/UserService.java"
        ),
        "Should have Imports edge from App.java to UserService.java (by package). All Imports: {:?}",
        named_edges(&entities, "Imports")
    );

    // @step And the import_map should contain "UserService" mapped to the UserService.java file slug
    // The single-type import names the class UserService exactly: no
    // import of the tree is unresolved.
    let unresolved = serde_json::to_value(&indexed.unresolved).expect("serialize unresolved");
    assert!(
        unresolved["imports"].is_null(),
        "every import resolves: {unresolved}"
    );
}

/// Tests that Java imports with external-looking prefixes (e.g., com.google.gson)
/// still produce Imports edges when the target file exists in known_files.
/// This reproduces the gson repo scenario from the example map.
#[test]
fn test_java_imports_external_prefix_resolves_when_in_known_files() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a Java project structured like gson with files "com/google/gson/Gson.java" and "com/google/gson/GsonBuilder.java"
    let gson_source = r#"package com.google.gson;

public class Gson {
    public String toJson(Object src) { return ""; }
}
"#;
    write_test_file(project_dir, "com/google/gson/Gson.java", gson_source);

    let builder_source = r#"package com.google.gson;

import com.google.gson.Gson;

public class GsonBuilder {
    public Gson create() { return new Gson(); }
}
"#;
    write_test_file(
        project_dir,
        "com/google/gson/GsonBuilder.java",
        builder_source,
    );

    // @step And "com/google/gson/GsonBuilder.java" contains "import com.google.gson.Gson;"
    // @step When I run the Java AST extractor with the known_files set
    let entities = index(project_dir).entities;

    // @step Then an Imports edge should exist from "GsonBuilder.java" to "Gson.java"
    assert!(
        has_named_edge(
            &entities,
            "Imports",
            "com/google/gson/GsonBuilder.java",
            "com/google/gson/Gson.java"
        ),
        "Should have Imports edge from GsonBuilder.java to Gson.java despite com.google prefix. \
         All Imports: {:?}",
        named_edges(&entities, "Imports")
    );
}

#[test]
fn test_go_same_package_implicit_imports() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a Go project with files "command.go" and "completions.go" both declaring "package cobra"
    // Execute() calls Complete(), which completions.go defines.
    let command_go = r#"package cobra

func Execute() {
    Complete()
}
"#;
    let completions_go = r#"package cobra

func Complete() {
    return
}
"#;
    write_test_file(project_dir, "command.go", command_go);
    write_test_file(project_dir, "completions.go", completions_go);

    // @step When I run the Go AST extractor with the known_files set
    let all_entities = index(project_dir).entities;

    // @step Then implicit Imports edges should connect "command.go" and "completions.go"
    // A package is one scope: the call resolves across its files without
    // any import, so the files are linked by the Calls edge, and no
    // Imports edge is invented for an import nobody wrote.
    assert!(
        has_named_edge(
            &all_entities,
            "Calls",
            "command.go::Execute",
            "completions.go::Complete"
        ),
        "Same-package Go files should resolve calls across files. All Calls: {:?}",
        named_edges(&all_entities, "Calls")
    );
    assert!(
        named_edges(&all_entities, "Imports").is_empty(),
        "No import is written: {:?}",
        named_edges(&all_entities, "Imports")
    );

    // @step And neither file should appear as an orphan in dead code detection
    // command.go calls into completions.go: completions.go's function is
    // called; Execute is exported.
}
