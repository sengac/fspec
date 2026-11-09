// Feature: spec/features/calls-imports-typeref-edges-csharp.feature
//
// Integration tests for C# Imports, Calls, and TypeRef edge extraction.
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
// Scenario: Extract Imports edges from C# using statements
// ============================================================================
#[test]
fn test_csharp_extract_imports_from_using_statements() -> TestResult {
    // @step Given a C# file with `using MyApp.Services;`
    let cs_source = r#"
using MyApp.Services;
using System.Collections;

namespace MyApp.Controllers {
    public class Controller {
        public void Handle() {
            var svc = new UserService();
        }
    }
}
"#;

    // @step And the target file `MyApp/Services.cs` exists in the project
    // `using MyApp.Services;` names the namespace a file of the tree
    // declares.
    let target_source = r#"
namespace MyApp.Services {
    public class UserService {}
}
"#;

    // @step When the C# extractor processes the source file
    let (_directory, indexed) = index_project(&[
        ("Controllers/Controller.cs", cs_source),
        ("MyApp/Services.cs", target_source),
    ])?;
    let all = &indexed.entities;

    // @step Then an Imports edge should be emitted from the source file to the target
    assert_eq!(
        imports_of(all, "Controllers/Controller.cs"),
        ["MyApp/Services.cs"],
        "Imports: {:?}",
        edges(all, "Imports")
    );

    // @step And system `using System.Collections` imports should NOT produce edges
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
// Scenario: Extract Calls edges from C# method calls
// ============================================================================
#[test]
fn test_csharp_extract_calls_from_method_calls() -> TestResult {
    let cs_source = r#"
public class Service {
    public void Process() {
        Validate();
    }

    public void Validate() {
        return;
    }
}
"#;

    // @step Given a C# file with method `Process()` that calls `Validate()`
    // @step And `Validate` is defined in the same file
    // @step When the C# extractor processes the source file
    let (_directory, indexed) = index_project(&[("Service.cs", cs_source)])?;
    let all = &indexed.entities;

    // @step Then a Calls edge should be emitted from `Process` to `Validate`
    let process = slug_of(all, "Function", "Service.cs::Service::Process")?;
    assert_eq!(
        callees_of(all, &process),
        ["Service.cs::Service::Validate"],
        "Calls: {:?}",
        edges(all, "Calls")
    );
    Ok(())
}

// ============================================================================
// Scenario: Extract TypeRef edges from C# type annotations
// ============================================================================
#[test]
fn test_csharp_extract_typerefs_from_type_annotations() -> TestResult {
    let cs_source = r#"
public class Request {
    public string Url;
}

public class Response {
    public int Status;
}

public class Handler {
    public Response Handle(Request req) {
        return new Response();
    }
}
"#;

    // @step Given a C# file with `public Response Handle(Request req)`
    // @step And types `Request` and `Response` are defined in the same file
    // @step When the C# extractor processes the source file
    let (_directory, indexed) = index_project(&[("Handler.cs", cs_source)])?;
    let all = &indexed.entities;

    // @step Then TypeRef edges should be emitted from `Handle` to `Request` and `Response`
    let handle = slug_of(all, "Function", "Handler.cs::Handler::Handle")?;
    assert_eq!(
        type_refs_of(all, &handle),
        ["Handler.cs::Request", "Handler.cs::Response"],
        "TypeRef: {:?}",
        edges(all, "TypeRef")
    );
    Ok(())
}
