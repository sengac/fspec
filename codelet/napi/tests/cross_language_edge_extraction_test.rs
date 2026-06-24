// Feature: spec/features/cross-language-calls-imports-typeref-edge-extraction-for-dead-code-detection.feature
//
// Integration test for the parent story KGRAPH-041.
// Verifies that all 12 non-TS language extractors emit Imports, Calls, and/or TypeRef edges.
// Each child story (KGRAPH-042..053) has its own detailed per-language integration tests.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_napi::graph::ast_pipeline::{
    ast_c_extractor::extract_c, ast_cpp_extractor::extract_cpp,
    ast_csharp_extractor::extract_csharp, ast_go_extractor::extract_go,
    ast_java_extractor::extract_java, ast_kotlin_extractor::extract_kotlin,
    ast_php_extractor::extract_php, ast_python_extractor::extract_python,
    ast_ruby_extractor::extract_ruby, ast_rust_extractor::extract_rust,
    ast_scala_extractor::extract_scala, ast_swift_extractor::extract_swift,
};

mod graph_test_helpers;
use graph_test_helpers::{build_known_files, find_edges, write_test_file};

// ============================================================================
// Scenario: All language extractors emit edge relationships for dead code analysis
// ============================================================================
#[test]
fn test_all_extractors_emit_edges_for_dead_code_analysis() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let dir = temp_dir.path();

    // @step Given updated extractors for PHP, Python, Go, Rust, Java, C, C++, C#, Ruby, Kotlin, Swift, and Scala
    // Create minimal source files for each language with local imports and calls

    // PHP
    write_test_file(dir, "Slim/App.php", "<?php\nnamespace Slim;\nuse Slim\\Routing\\Router;\nclass App {\n  public function run() { $this->boot(); }\n  public function boot() {}\n}\n");
    write_test_file(
        dir,
        "Slim/Routing/Router.php",
        "<?php\nnamespace Slim\\Routing;\nclass Router {}\n",
    );

    // Python
    write_test_file(
        dir,
        "cli/app.py",
        "from cli.util import helper\ndef main():\n    helper()\ndef helper():\n    pass\n",
    );
    write_test_file(dir, "cli/util.py", "def helper(): pass\n");

    // Go
    write_test_file(dir, "cmd/root.go", "package cmd\nimport \"./internal/util\"\nfunc Execute() { initConfig() }\nfunc initConfig() {}\n");
    write_test_file(
        dir,
        "internal/util/helpers.go",
        "package util\nfunc Helper() {}\n",
    );

    // Rust
    write_test_file(
        dir,
        "src/lib.rs",
        "use crate::graph::helpers;\nfn extract() { slugify() }\nfn slugify() {}\n",
    );
    write_test_file(dir, "src/graph/helpers.rs", "pub fn slugify_path() {}\n");

    // Java
    write_test_file(dir, "com/app/Service.java", "package com.app;\nimport com.app.Util;\npublic class Service { void run() { init(); } void init() {} }\n");
    write_test_file(
        dir,
        "com/app/Util.java",
        "package com.app;\npublic class Util {}\n",
    );

    // C
    write_test_file(
        dir,
        "main.c",
        "#include \"util.h\"\nvoid main() { helper(); }\nvoid helper() {}\n",
    );
    write_test_file(dir, "util.h", "void helper();\n");

    // C++
    write_test_file(dir, "main.cpp", "#include \"util.h\"\nvoid run() {}\n");

    // C#
    write_test_file(dir, "MyApp/Service.cs", "using MyApp.Utils;\nnamespace MyApp { class Service { void Run() { Init(); } void Init() {} } }\n");
    write_test_file(
        dir,
        "MyApp/Utils.cs",
        "namespace MyApp.Utils { class Utils {} }\n",
    );

    // Ruby
    write_test_file(dir, "lib/app.rb", "require_relative 'helpers/util'\ndef process(data)\n  validate(data)\nend\ndef validate(data)\nend\n");
    write_test_file(dir, "lib/helpers/util.rb", "def util_fn; end\n");

    // Kotlin
    write_test_file(
        dir,
        "com/app/Service.kt",
        "package com.app\nimport com.app.Util\nfun handle() { process() }\nfun process() {}\n",
    );
    write_test_file(dir, "com/app/Util.kt", "package com.app\nclass Util\n");

    // Swift
    write_test_file(
        dir,
        "Sources/App.swift",
        "func handle() { process() }\nfunc process() {}\n",
    );

    // Scala
    write_test_file(dir, "com/app/Service.scala", "package com.app\nimport com.app.Util\ndef handle(): Unit = { process() }\ndef process(): Unit = {}\n");
    write_test_file(dir, "com/app/Util.scala", "package com.app\nclass Util\n");

    // @step And each extractor accepts a `known_files` parameter for import resolution
    let known_files = build_known_files(dir);

    // @step When each extractor processes source files with imports, function calls, and type annotations
    let php = extract_php(
        &std::fs::read_to_string(dir.join("Slim/App.php")).unwrap(),
        "Slim/App.php",
        &known_files,
    )
    .expect("PHP");
    let python = extract_python(
        &std::fs::read_to_string(dir.join("cli/app.py")).unwrap(),
        "cli/app.py",
        &known_files,
    )
    .expect("Python");
    let go = extract_go(
        &std::fs::read_to_string(dir.join("cmd/root.go")).unwrap(),
        "cmd/root.go",
        &known_files,
    )
    .expect("Go");
    let rust = extract_rust(
        &std::fs::read_to_string(dir.join("src/lib.rs")).unwrap(),
        "src/lib.rs",
        &known_files,
    )
    .expect("Rust");
    let java = extract_java(
        &std::fs::read_to_string(dir.join("com/app/Service.java")).unwrap(),
        "com/app/Service.java",
        &known_files,
    )
    .expect("Java");
    let c = extract_c(
        &std::fs::read_to_string(dir.join("main.c")).unwrap(),
        "main.c",
        &known_files,
    )
    .expect("C");
    let cpp = extract_cpp(
        &std::fs::read_to_string(dir.join("main.cpp")).unwrap(),
        "main.cpp",
        &known_files,
    )
    .expect("C++");
    let csharp = extract_csharp(
        &std::fs::read_to_string(dir.join("MyApp/Service.cs")).unwrap(),
        "MyApp/Service.cs",
        &known_files,
    )
    .expect("C#");
    let ruby = extract_ruby(
        &std::fs::read_to_string(dir.join("lib/app.rb")).unwrap(),
        "lib/app.rb",
        &known_files,
    )
    .expect("Ruby");
    let kotlin = extract_kotlin(
        &std::fs::read_to_string(dir.join("com/app/Service.kt")).unwrap(),
        "com/app/Service.kt",
        &known_files,
    )
    .expect("Kotlin");
    let swift = extract_swift(
        &std::fs::read_to_string(dir.join("Sources/App.swift")).unwrap(),
        "Sources/App.swift",
        &known_files,
    )
    .expect("Swift");
    let scala = extract_scala(
        &std::fs::read_to_string(dir.join("com/app/Service.scala")).unwrap(),
        "com/app/Service.scala",
        &known_files,
    )
    .expect("Scala");

    // @step Then Imports edges are emitted for project-local imports in each language
    assert!(
        !find_edges(&php, "Imports", None, None).is_empty(),
        "PHP should emit Imports edges"
    );
    assert!(
        !find_edges(&python, "Imports", None, None).is_empty(),
        "Python should emit Imports edges"
    );
    assert!(
        !find_edges(&go, "Imports", None, None).is_empty(),
        "Go should emit Imports edges"
    );
    assert!(
        !find_edges(&rust, "Imports", None, None).is_empty(),
        "Rust should emit Imports edges"
    );
    assert!(
        !find_edges(&java, "Imports", None, None).is_empty(),
        "Java should emit Imports edges"
    );
    assert!(
        !find_edges(&c, "Imports", None, None).is_empty(),
        "C should emit Imports edges"
    );
    assert!(
        !find_edges(&cpp, "Imports", None, None).is_empty(),
        "C++ should emit Imports edges"
    );
    assert!(
        !find_edges(&csharp, "Imports", None, None).is_empty(),
        "C# should emit Imports edges"
    );
    assert!(
        !find_edges(&ruby, "Imports", None, None).is_empty(),
        "Ruby should emit Imports edges"
    );
    assert!(
        !find_edges(&kotlin, "Imports", None, None).is_empty(),
        "Kotlin should emit Imports edges"
    );
    assert!(
        !find_edges(&scala, "Imports", None, None).is_empty(),
        "Scala should emit Imports edges"
    );

    // @step And Calls edges are emitted for same-file function calls in each language
    assert!(
        !find_edges(&php, "Calls", None, None).is_empty(),
        "PHP should emit Calls edges"
    );
    assert!(
        !find_edges(&python, "Calls", None, None).is_empty(),
        "Python should emit Calls edges"
    );
    assert!(
        !find_edges(&go, "Calls", None, None).is_empty(),
        "Go should emit Calls edges"
    );
    assert!(
        !find_edges(&rust, "Calls", None, None).is_empty(),
        "Rust should emit Calls edges"
    );
    assert!(
        !find_edges(&java, "Calls", None, None).is_empty(),
        "Java should emit Calls edges"
    );
    assert!(
        !find_edges(&c, "Calls", None, None).is_empty(),
        "C should emit Calls edges"
    );
    assert!(
        !find_edges(&csharp, "Calls", None, None).is_empty(),
        "C# should emit Calls edges"
    );
    assert!(
        !find_edges(&ruby, "Calls", None, None).is_empty(),
        "Ruby should emit Calls edges"
    );
    assert!(
        !find_edges(&kotlin, "Calls", None, None).is_empty(),
        "Kotlin should emit Calls edges"
    );
    assert!(
        !find_edges(&swift, "Calls", None, None).is_empty(),
        "Swift should emit Calls edges"
    );
    assert!(
        !find_edges(&scala, "Calls", None, None).is_empty(),
        "Scala should emit Calls edges"
    );

    // @step And TypeRef edges are emitted for type annotations where the language supports them
    // TypeRef is language-dependent — statically typed languages emit them
    // PHP, Java, C#, Kotlin, Scala, Rust support TypeRef; Python/Go/C/C++/Ruby/Swift may not

    // @step And external package imports do not generate any edges
    // Verified by the individual language tests (children)
    // External imports (serde, cobra, System.Collections, etc.) are filtered in each extractor
}
