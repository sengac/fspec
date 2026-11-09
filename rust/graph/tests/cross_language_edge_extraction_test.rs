// Feature: spec/features/cross-language-calls-imports-typeref-edge-extraction-for-dead-code-detection.feature
//
// Integration test for the parent story KGRAPH-041.
// Verifies that the fact engine and resolve pass emit Imports, Calls, and/or
// TypeRef edges for all 12 non-TS languages of one indexed project.
// Each child story (KGRAPH-042..053) has its own detailed per-language integration tests.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_graph::graph_entities::GraphEntity;
use codelet_test_helpers::{TestError, TestResult};
use serde_json::Value;

pub mod code_graph_fixture;
use code_graph_fixture::{edges, entities, index, nodes, project};

/// The slugs of the `node_type` nodes declared in `path` and named `name`.
fn declared(graph: &[GraphEntity], node_type: &str, path: &str, name: &str) -> Vec<String> {
    let prefix = format!("{path}::");
    nodes(graph, node_type)
        .into_iter()
        .filter(|(_, p)| {
            p.get("name").and_then(Value::as_str) == Some(name)
                && p.get("qualifiedName")
                    .and_then(Value::as_str)
                    .is_some_and(|q| q.starts_with(&prefix))
        })
        .map(|(slug, _)| slug)
        .collect()
}

/// The files `path` has an Imports edge to, sorted.
fn imports_of(graph: &[GraphEntity], path: &str) -> Vec<String> {
    let mut found: Vec<String> = edges(graph, "Imports")
        .into_iter()
        .filter(|(from, _, _)| from == path)
        .map(|(_, to, _)| to)
        .collect();
    found.sort();
    found
}

/// Whether an `edge_type` edge leads from `from` (a `from_type` named
/// `from.1` in file `from.0`) to `to` (a `to_type` likewise).
fn linked(
    graph: &[GraphEntity],
    edge_type: &str,
    (from_type, from): (&str, (&str, &str)),
    (to_type, to): (&str, (&str, &str)),
) -> Result<bool, TestError> {
    let sources = declared(graph, from_type, from.0, from.1);
    let targets = declared(graph, to_type, to.0, to.1);
    if sources.len() != 1 || targets.len() != 1 {
        return Err(format!("{from:?}: {sources:?}, {to:?}: {targets:?}").into());
    }
    Ok(edges(graph, edge_type)
        .iter()
        .any(|(f, t, _)| *f == sources[0] && *t == targets[0]))
}

// ============================================================================
// Scenario: All language extractors emit edge relationships for dead code analysis
// ============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_all_extractors_emit_edges_for_dead_code_analysis() -> TestResult {
    // @step Given updated extractors for PHP, Python, Go, Rust, Java, C, C++, C#, Ruby, Kotlin, Swift, and Scala
    // Minimal source files for each language with local imports and calls.
    let files: &[(&str, &str)] = &[
        // PHP
        ("Slim/App.php", "<?php\nnamespace Slim;\nuse Slim\\Routing\\Router;\nclass App {\n  public function run() { $this->boot(); }\n  public function boot() {}\n}\n"),
        ("Slim/Routing/Router.php", "<?php\nnamespace Slim\\Routing;\nclass Router {}\n"),
        // Python (`os` is an external module)
        ("cli/app.py", "import os\nfrom cli.util import helper\ndef main():\n    helper()\n    run()\ndef run():\n    pass\n"),
        ("cli/util.py", "def helper(): pass\n"),
        // Go: a module whose go.mod names the import path
        ("go.mod", "module example.com/app\n\ngo 1.21\n"),
        ("cmd/root.go", "package cmd\nimport \"example.com/app/internal/util\"\nfunc Execute() { initConfig(); util.Helper() }\nfunc initConfig() {}\n"),
        ("internal/util/helpers.go", "package util\nfunc Helper() {}\n"),
        // Rust: a crate whose module tree holds graph::helpers
        ("Cargo.toml", "[package]\nname = \"fixture\"\n"),
        ("src/lib.rs", "mod graph;\nuse crate::graph::helpers;\nfn extract() { slugify() }\nfn slugify() {}\n"),
        ("src/graph/mod.rs", "pub mod helpers;\n"),
        ("src/graph/helpers.rs", "pub fn slugify_path() {}\n"),
        // Java
        ("com/app/Service.java", "package com.app;\nimport com.app.Util;\npublic class Service { void run() { init(); } void init() {} void take(Util u) {} }\n"),
        ("com/app/Util.java", "package com.app;\npublic class Util {}\n"),
        // C
        ("main.c", "#include \"util.h\"\nvoid main() { helper(); }\nvoid helper() {}\n"),
        ("util.h", "void helper();\n"),
        // C++
        ("main.cpp", "#include \"util.h\"\nvoid run() {}\n"),
        // C#
        ("MyApp/Service.cs", "using MyApp.Utils;\nnamespace MyApp { class Service { void Run() { Init(); } void Init() {} } }\n"),
        ("MyApp/Utils.cs", "namespace MyApp.Utils { class Utils {} }\n"),
        // Ruby
        ("lib/app.rb", "require_relative 'helpers/util'\ndef process(data)\n  validate(data)\nend\ndef validate(data)\nend\n"),
        ("lib/helpers/util.rb", "def util_fn; end\n"),
        // Kotlin (its own package: Java, Kotlin and Scala share the JVM's
        // packages, so one `com.app.Util` per language would be ambiguous)
        ("com/kapp/Service.kt", "package com.kapp\nimport com.kapp.Util\nfun handle() { process() }\nfun process() {}\n"),
        ("com/kapp/Util.kt", "package com.kapp\nclass Util\n"),
        // Swift
        ("Sources/App.swift", "func handle() { process() }\nfunc process() {}\n"),
        // Scala (its own package, as Kotlin)
        ("com/sapp/Service.scala", "package com.sapp\nimport com.sapp.Util\ndef handle(): Unit = { process() }\ndef process(): Unit = {}\n"),
        ("com/sapp/Util.scala", "package com.sapp\nclass Util\n"),
    ];
    let (_directory, root) = project(files)?;

    // @step And each extractor accepts a `known_files` parameter for import resolution
    // (the resolve pass sees every file of the tree)

    // @step When each extractor processes source files with imports, function calls, and type annotations
    index(&root).await?;
    let graph = entities(&root).await?;

    // @step Then Imports edges are emitted for project-local imports in each language
    let expected_imports: &[(&str, &str)] = &[
        ("Slim/App.php", "Slim/Routing/Router.php"),
        ("cli/app.py", "cli/util.py"),
        ("cmd/root.go", "internal/util/helpers.go"),
        ("src/lib.rs", "src/graph/helpers.rs"),
        ("com/app/Service.java", "com/app/Util.java"),
        ("main.c", "util.h"),
        ("main.cpp", "util.h"),
        ("MyApp/Service.cs", "MyApp/Utils.cs"),
        ("lib/app.rb", "lib/helpers/util.rb"),
        ("com/kapp/Service.kt", "com/kapp/Util.kt"),
        ("com/sapp/Service.scala", "com/sapp/Util.scala"),
    ];
    for (from, to) in expected_imports {
        assert!(
            imports_of(&graph, from).contains(&to.to_string()),
            "{from} should import {to}, got {:?}",
            imports_of(&graph, from)
        );
    }

    // @step And Calls edges are emitted for same-file function calls in each language
    let expected_calls: &[(&str, &str, &str)] = &[
        ("Slim/App.php", "run", "boot"),
        ("cli/app.py", "main", "run"),
        ("cmd/root.go", "Execute", "initConfig"),
        ("src/lib.rs", "extract", "slugify"),
        ("com/app/Service.java", "run", "init"),
        ("main.c", "main", "helper"),
        ("MyApp/Service.cs", "Run", "Init"),
        ("lib/app.rb", "process", "validate"),
        ("com/kapp/Service.kt", "handle", "process"),
        ("Sources/App.swift", "handle", "process"),
        ("com/sapp/Service.scala", "handle", "process"),
    ];
    let mut missing = Vec::new();
    for (path, caller, callee) in expected_calls {
        if !linked(
            &graph,
            "Calls",
            ("Function", (path, caller)),
            ("Function", (path, callee)),
        )? {
            missing.push(format!("{path}: {caller} -> {callee}"));
        }
    }
    assert!(missing.is_empty(), "missing Calls edges: {missing:?}");
    // Calls through an import reach the imported file's definition.
    assert!(linked(
        &graph,
        "Calls",
        ("Function", ("cli/app.py", "main")),
        ("Function", ("cli/util.py", "helper")),
    )?);
    assert!(linked(
        &graph,
        "Calls",
        ("Function", ("cmd/root.go", "Execute")),
        ("Function", ("internal/util/helpers.go", "Helper")),
    )?);

    // @step And TypeRef edges are emitted for type annotations where the language supports them
    assert!(linked(
        &graph,
        "TypeRef",
        ("Function", ("com/app/Service.java", "take")),
        ("Type", ("com/app/Util.java", "Util")),
    )?);

    // @step And external package imports do not generate any edges
    assert_eq!(imports_of(&graph, "cli/app.py"), vec!["cli/util.py"]);
    Ok(())
}
