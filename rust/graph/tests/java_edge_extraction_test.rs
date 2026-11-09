// Feature: spec/features/calls-imports-typeref-edges-java.feature
//
// Integration tests for Java Imports, Calls, and TypeRef edge extraction.
// Every scenario indexes a temp project with the real pipeline (walk, fact
// extraction, global resolve pass); no edge is hand-written.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_graph::ast_pipeline::resolve::{Reason, RefKindCount};
use codelet_graph::ast_pipeline::{index_tree, Indexed};
use codelet_graph::graph_entities::GraphEntity;
use codelet_test_helpers::{TestError, TestResult};

pub mod code_graph_fixture;
use code_graph_fixture::{callees_of, edges, nodes, project, slug_of};

/// Index a temp project holding `files` with the real pipeline; keep the
/// returned directory alive.
fn index_project(files: &[(&str, &str)]) -> Result<(tempfile::TempDir, Indexed), TestError> {
    let (directory, root) = project(files)?;
    let indexed = index_tree(&root, false, 1);
    Ok((directory, indexed))
}

/// The files `from` has Imports edges to, sorted.
fn imports_of(all: &[GraphEntity], from: &str) -> Vec<String> {
    let mut found: Vec<String> = edges(all, "Imports")
        .into_iter()
        .filter(|(f, _, _)| f == from)
        .map(|(_, to, _)| to)
        .collect();
    found.sort();
    found
}

/// The count of unresolved imports with `reason`.
fn unresolved_imports(indexed: &Indexed, reason: Reason) -> u64 {
    indexed
        .unresolved
        .get(&RefKindCount::Imports)
        .and_then(|reasons| reasons.get(&reason))
        .copied()
        .unwrap_or(0)
}

/// The qualifiedNames the TypeRef edges from `function` lead to, sorted.
fn type_refs_of(all: &[GraphEntity], function: &str) -> Vec<String> {
    let names: std::collections::BTreeMap<String, String> = nodes(all, "Type")
        .into_iter()
        .filter_map(|(slug, p)| Some((slug, p.get("qualifiedName")?.as_str()?.to_string())))
        .collect();
    let mut found: Vec<String> = edges(all, "TypeRef")
        .into_iter()
        .filter(|(from, _, _)| from == function)
        .filter_map(|(_, to, _)| names.get(&to).cloned())
        .collect();
    found.sort();
    found.dedup();
    found
}
// ============================================================================
// Scenario: Extract Imports edges from Java import statements
// ============================================================================
#[test]
fn test_java_extract_imports_from_import_statements() -> TestResult {
    // @step Given a Java file with `import com.myapp.service.UserService;`
    let java_source = r#"
package com.myapp.web;

import com.myapp.service.UserService;
import java.util.List;

public class Controller {
    public void handle() {
        UserService svc = new UserService();
    }
}
"#;

    // @step And the target file `com/myapp/service/UserService.java` exists in the project
    let target_source = r#"
package com.myapp.service;

public class UserService {
    public void process() {}
}
"#;

    // @step When the Java extractor processes the source file
    let (_directory, indexed) = index_project(&[
        ("com/myapp/web/Controller.java", java_source),
        ("com/myapp/service/UserService.java", target_source),
    ])?;
    let all = &indexed.entities;

    // @step Then an Imports edge should be emitted from the source file to the target file
    assert_eq!(
        imports_of(all, "com/myapp/web/Controller.java"),
        ["com/myapp/service/UserService.java"],
        "Imports: {:?}",
        edges(all, "Imports")
    );

    // @step And external `import java.util.List` imports should NOT produce edges
    assert_eq!(
        edges(all, "Imports").len(),
        1,
        "Imports: {:?}",
        edges(all, "Imports")
    );
    assert_eq!(
        unresolved_imports(&indexed, Reason::External),
        1,
        "{:?}",
        indexed.unresolved
    );
    Ok(())
}

// ============================================================================
// Scenario: Extract Calls edges from Java method calls
// ============================================================================
#[test]
fn test_java_extract_calls_from_method_calls() -> TestResult {
    let java_source = r#"
public class Service {
    public void processRequest() {
        validate();
    }

    public void validate() {
        return;
    }
}
"#;

    // @step Given a Java file with method `processRequest()` that calls `validate()`
    // @step And `validate` is defined in the same file
    // @step When the Java extractor processes the source file
    let (_directory, indexed) = index_project(&[("Service.java", java_source)])?;
    let all = &indexed.entities;

    // @step Then a Calls edge should be emitted from `processRequest` to `validate`
    let process = slug_of(all, "Function", "Service.java::Service::processRequest")?;
    assert_eq!(
        callees_of(all, &process),
        ["Service.java::Service::validate"],
        "Calls: {:?}",
        edges(all, "Calls")
    );
    Ok(())
}

// ============================================================================
// Scenario: Extract TypeRef edges from Java type annotations
// ============================================================================
#[test]
fn test_java_extract_typerefs_from_type_annotations() -> TestResult {
    // One public top-level class per file: Request and Response are
    // package-private.
    let java_source = r#"
class Request {
    public String url;
}

class Response {
    public int status;
}

public class Handler {
    public Response handle(Request req) {
        return new Response();
    }
}
"#;

    // @step Given a Java file with `public Response handle(Request req)`
    // @step And types `Request` and `Response` are defined in the same file
    // @step When the Java extractor processes the source file
    let (_directory, indexed) = index_project(&[("Handler.java", java_source)])?;
    let all = &indexed.entities;

    // @step Then TypeRef edges should be emitted from `handle` to `Request` and `Response`
    let handle = slug_of(all, "Function", "Handler.java::Handler::handle")?;
    assert_eq!(
        type_refs_of(all, &handle),
        ["Handler.java::Request", "Handler.java::Response"],
        "TypeRef: {:?}",
        edges(all, "TypeRef")
    );
    Ok(())
}
