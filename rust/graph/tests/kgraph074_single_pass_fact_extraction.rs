//! KGRAPH-074: single-pass AST fact extraction, data-driven per language.
//!
//! Feature: spec/features/single-pass-ast-fact-extraction.feature
//!
//! Every graph lives in a temp project. Fact-level scenarios call the fact
//! engine on one source text; graph-level scenarios run `ast_index`.

use std::collections::BTreeMap;

use ast_grep_core::{Doc, Node};
use ast_grep_language::LanguageExt;
use codelet_graph::ast_dispatch::dispatch_ast_search;
use codelet_graph::ast_pipeline::facts::{
    self, extract::extract_source, languages, spec::Grammar, Callee, FileFacts, Receiver,
};
use codelet_graph::registry;
use codelet_test_helpers::{TestError, TestResult};
use serde_json::Value;

pub mod code_graph_fixture;
use code_graph_fixture::{edges, entities, index, nodes, project, slug_of, CRATE_MANIFEST};

fn facts_of(rel: &str, source: &str) -> Result<FileFacts, TestError> {
    let spec = languages::for_path(rel).ok_or_else(|| format!("no language for {rel}"))?;
    Ok(extract_source(spec, rel, source))
}

/// The names of the search results of `node_type` matching `query`.
async fn search(
    root: &std::path::Path,
    query: &str,
    node_type: &str,
) -> Result<Vec<Value>, TestError> {
    let graph = registry::read_code_graph(root).await?;
    let output = dispatch_ast_search(
        &graph,
        query,
        Some(node_type),
        Some(200),
        None,
        None,
        None,
        None,
    )
    .await;
    let value: Value = serde_json::from_str(&output)?;
    Ok(value["results"]
        .as_array()
        .ok_or_else(|| format!("no results in {output}"))?
        .clone())
}

fn names(rows: &[Value]) -> Vec<String> {
    let mut names: Vec<String> = rows
        .iter()
        .filter_map(|r| r["name"].as_str().map(str::to_string))
        .collect();
    names.sort();
    names
}

/// The name of the definition a call site belongs to.
fn caller_name(facts: &FileFacts, caller: usize) -> &str {
    &facts.definitions[caller].name
}

/// Scenario: Every kind of Rust fn is a definition
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_kind_of_rust_fn_is_a_definition() -> TestResult {
    // @step Given a Rust file holding fn generic<T>(), fn with_where<T>() where T: Clone, const fn c(), unsafe fn u(), async fn a() and extern "C" fn e()
    let source = "fn generic<T>() {}\nfn with_where<T>() where T: Clone {}\nconst fn c() {}\nunsafe fn u() {}\nasync fn a() {}\nextern \"C\" fn e() {}\n";
    let (_directory, root) = project(&[CRATE_MANIFEST, ("src/lib.rs", source)])?;

    // @step When ast_index runs for the project
    index(&root).await?;

    // @step And ast_search lists the project's functions
    let rows = search(&root, "", "Function").await?;

    // @step Then the results hold generic, with_where, c, u, a and e
    assert_eq!(names(&rows), ["a", "c", "e", "generic", "u", "with_where"]);
    Ok(())
}

/// Scenario: Same-named methods of different impl blocks are distinct functions
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn same_named_methods_of_different_impls_are_distinct() -> TestResult {
    // @step Given src/lib.rs holding impl A { fn new() } and impl B { fn new() }, where B::new calls fn helper
    let source = "pub struct A;\npub struct B;\nimpl A { fn new() {} }\nimpl B { fn new() { helper(); } }\nfn helper() {}\n";
    let (_directory, root) = project(&[CRATE_MANIFEST, ("src/lib.rs", source)])?;

    // @step When ast_index runs for the project
    index(&root).await?;

    // @step And ast_search lists the functions named new
    let rows = search(&root, "new", "Function").await?;

    // @step Then the results hold two functions with qualifiedNames src/lib.rs::A::new and src/lib.rs::B::new
    let mut qualified: Vec<&str> = rows
        .iter()
        .filter(|r| r["name"] == "new")
        .filter_map(|r| r["qualifiedName"].as_str())
        .collect();
    qualified.sort();
    assert_eq!(qualified, ["src/lib.rs::A::new", "src/lib.rs::B::new"]);

    // @step And ast_callees of B::new is helper and ast_callees of A::new is empty
    let all = entities(&root).await?;
    let graph = registry::read_code_graph(&root).await?;
    for (method, expected) in [
        ("src/lib.rs::B::new", vec!["helper"]),
        ("src/lib.rs::A::new", vec![]),
    ] {
        let slug = slug_of(&all, "Function", method)?;
        let output =
            codelet_graph::ast_transitive::dispatch_ast_callees(&graph, &slug, Some(1), None).await;
        let value: Value = serde_json::from_str(&output)?;
        let rows = value["results"]
            .as_array()
            .ok_or_else(|| format!("no results: {output}"))?;
        assert_eq!(names(rows), expected, "callees of {method}: {output}");
    }
    Ok(())
}

/// Scenario: Files whose paths differ only in separators and dots stay distinct
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn paths_differing_in_separators_and_dots_stay_distinct() -> TestResult {
    // @step Given files src/a-b.rs, src/a/b.rs and src/a.b.rs that each hold fn f
    let paths = ["src/a-b.rs", "src/a.b.rs", "src/a/b.rs"];
    let files: Vec<(&str, &str)> = paths.iter().map(|p| (*p, "fn f() {}\n")).collect();
    let (_directory, root) = project(&files)?;

    // @step When ast_index runs for the project
    index(&root).await?;

    // @step Then the graph holds three File nodes with those three paths
    let all = entities(&root).await?;
    let mut file_paths: Vec<String> = nodes(&all, "File")
        .into_iter()
        .filter_map(|(_, p)| p.get("path")?.as_str().map(str::to_string))
        .collect();
    file_paths.sort();
    assert_eq!(file_paths, paths);

    // @step And three functions named f, one contained in each file
    let functions: BTreeMap<String, String> = nodes(&all, "Function")
        .into_iter()
        .filter(|(_, p)| p.get("name").and_then(Value::as_str) == Some("f"))
        .map(|(slug, p)| {
            (
                slug,
                p.get("qualifiedName")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            )
        })
        .collect();
    assert_eq!(functions.len(), 3, "{functions:?}");
    let mut containing: Vec<String> = edges(&all, "Contains")
        .into_iter()
        .filter(|(_, to, _)| functions.contains_key(to))
        .map(|(from, _, _)| from)
        .collect();
    containing.sort();
    assert_eq!(containing, paths);
    Ok(())
}

/// Scenario: A call inside a nested function belongs only to the nested function
#[test]
fn a_nested_call_belongs_to_the_nested_function() -> TestResult {
    // @step Given fn outer() { fn inner() { helper() } inner() } and fn helper()
    let source = "fn outer() { fn inner() { helper() } inner() }\nfn helper() {}\n";

    // @step When the facts of the file are extracted
    let facts = facts_of("src/lib.rs", source)?;

    // @step Then the call site helper belongs to inner only
    let calls: Vec<(&str, &Callee)> = facts
        .calls
        .iter()
        .map(|c| (caller_name(&facts, c.caller), &c.callee))
        .collect();
    let helper = Callee::Path(vec!["helper".into()]);
    let inner = Callee::Path(vec!["inner".into()]);
    assert_eq!(
        calls
            .iter()
            .filter(|(_, c)| **c == helper)
            .collect::<Vec<_>>(),
        [&("inner", &helper)]
    );

    // @step And the call site of outer is inner
    assert_eq!(
        calls
            .iter()
            .filter(|(caller, _)| *caller == "outer")
            .collect::<Vec<_>>(),
        [&("outer", &inner)]
    );
    Ok(())
}

/// Scenario: Method calls and scoped calls are call-site facts
#[test]
fn method_and_scoped_calls_are_call_sites() -> TestResult {
    // @step Given a Rust function whose body calls self.run(), Type::new() and module::f()
    let source =
        "struct Type;\nimpl Type { fn go(&self) { self.run(); Type::new(); module::f(); } }\n";

    // @step When the facts of the file are extracted
    let facts = facts_of("src/lib.rs", source)?;

    // @step Then its call sites are the method run on receiver self, the scoped path Type::new and the scoped path module::f
    let callees: Vec<&Callee> = facts.calls.iter().map(|c| &c.callee).collect();
    assert_eq!(
        callees,
        [
            &Callee::Method {
                receiver: Receiver::SelfValue,
                name: "run".into()
            },
            &Callee::Path(vec!["Type".into(), "new".into()]),
            &Callee::Path(vec!["module".into(), "f".into()]),
        ]
    );
    Ok(())
}

/// Scenario: Test status comes from test attributes and test path conventions
#[test]
fn test_status_comes_from_attributes_and_path_conventions() -> TestResult {
    // @step Given src/latest.rs holding fn latest() and src/lib.rs holding #[cfg(test)] mod tests { #[test] fn checks() {} }
    let latest_source = "fn latest() {}\n";
    let lib_source =
        "fn lib_fn() {}\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn checks() {}\n}\n";

    // @step When the facts of the files are extracted
    let latest = facts_of("src/latest.rs", latest_source)?;
    let lib = facts_of("src/lib.rs", lib_source)?;

    // @step Then src/latest.rs is not a test file and latest is not a test
    assert!(!latest.is_test);
    assert_eq!(
        latest
            .definitions
            .iter()
            .map(|d| (d.name.as_str(), d.is_test))
            .collect::<Vec<_>>(),
        [("latest", false)]
    );

    // @step And checks is a test
    assert_eq!(
        lib.definitions
            .iter()
            .map(|d| (d.name.as_str(), d.is_test))
            .collect::<Vec<_>>(),
        [("lib_fn", false), ("checks", true)]
    );
    assert!(
        facts_of("tests/it.rs", latest_source)?.is_test,
        "a tests/ directory is a test path"
    );
    Ok(())
}

fn count_kind<D: Doc>(node: &Node<'_, D>, kind: &str, requires_field: Option<&str>) -> usize {
    node.dfs()
        .filter(|n| n.kind() == kind && requires_field.is_none_or(|f| n.field(f).is_some()))
        .count()
}

/// Scenario: Every node of a declared definition kind becomes one definition fact
#[test]
fn every_declared_definition_node_is_one_fact() -> TestResult {
    // @step Given a multi-language fixture with Rust, Python, TypeScript, Go and Java files
    let fixture = [
        ("src/lib.rs", "pub struct S;\nimpl S { fn a(&self) {} fn b() {} }\ntrait T { fn c(&self); fn d(&self) {} }\nenum E { X }\nfn top() { fn nested() {} }\n"),
        ("app/main.py", "class C:\n    def a(self):\n        def inner():\n            pass\n    async def b(self):\n        pass\n\ndef top():\n    pass\n"),
        ("web/app.ts", "interface I { m(): void }\nclass K implements I { m() {} n() {} }\nfunction top() {}\nenum Color { Red }\ntype Alias = string;\n"),
        ("cmd/main.go", "package main\n\ntype S struct{}\n\nfunc (s S) M() {}\n\nfunc top() {}\n"),
        ("src/Main.java", "class Main { Main() {} void a() {} void a(int x) {} interface Inner { void b(); } }\n"),
    ];
    let mut checked = 0;
    for (rel, source) in fixture {
        let spec = languages::for_path(rel).ok_or_else(|| format!("no language for {rel}"))?;

        // @step When the facts of the files are extracted
        let facts = extract_source(spec, rel, source);

        // @step Then for each file and each declared definition node kind, the number of definition facts equals the number of nodes of that kind in the file's syntax tree
        assert!(
            !spec.definitions.is_empty(),
            "{} declares no definitions",
            spec.name
        );
        for def in spec.definitions {
            let in_tree = match spec.grammar {
                Grammar::Builtin(lang) => {
                    count_kind(&lang.ast_grep(source).root(), def.kind, def.requires_field)
                }
                Grammar::Dart => return Err("the fixture has no Dart file".into()),
            };
            let in_facts = facts
                .definitions
                .iter()
                .filter(|d| d.node_kind == def.kind)
                .count();
            assert_eq!(in_facts, in_tree, "{rel}: {} nodes vs facts", def.kind);
            checked += in_tree;
        }
    }
    assert!(
        checked >= 20,
        "the fixture holds {checked} definition nodes"
    );
    Ok(())
}

/// Scenario: Facts are the same whatever the number of threads
#[test]
fn facts_are_the_same_whatever_the_number_of_threads() -> TestResult {
    // @step Given a multi-language fixture
    let mut files: Vec<(String, String)> = Vec::new();
    for i in 0..24 {
        files.push((
            format!("src/m{i:02}.rs"),
            format!("fn f{i}() {{ g{i}(); }}\nfn g{i}() {{}}\n"),
        ));
        files.push((
            format!("py/m{i:02}.py"),
            format!("def f{i}():\n    g{i}()\n\ndef g{i}():\n    pass\n"),
        ));
        files.push((
            format!("ts/m{i:02}.ts"),
            format!("function f{i}() {{ g{i}(); }}\nfunction g{i}() {{}}\n"),
        ));
    }
    let refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(p, t)| (p.as_str(), t.as_str()))
        .collect();
    let (_directory, root) = project(&refs)?;
    let mut paths: Vec<std::path::PathBuf> = refs.iter().map(|(p, _)| root.join(p)).collect();
    paths.reverse();

    // @step When its facts are extracted with 1 thread and with 8 threads
    let one = facts::extract_tree(&root, &paths, 1);
    let eight = facts::extract_tree(&root, &paths, 8);

    // @step Then the two serialized fact lists are byte-identical
    assert!(one.failed.is_empty(), "{:?}", one.failed);
    assert_eq!(one.files.len(), files.len());
    assert_eq!(serde_json::to_string(&one)?, serde_json::to_string(&eight)?);

    // @step And the facts are sorted by path, then start byte
    let file_paths: Vec<&str> = one.files.iter().map(|f| f.path.as_str()).collect();
    let mut sorted = file_paths.clone();
    sorted.sort();
    assert_eq!(file_paths, sorted);
    for file in &one.files {
        let starts: Vec<usize> = file.definitions.iter().map(|d| d.start_byte).collect();
        assert!(
            starts.windows(2).all(|w| w[0] < w[1]),
            "{}: {starts:?}",
            file.path
        );
        let calls: Vec<usize> = file.calls.iter().map(|c| c.start_byte).collect();
        assert!(
            calls.windows(2).all(|w| w[0] < w[1]),
            "{}: {calls:?}",
            file.path
        );
    }
    Ok(())
}

/// Scenario: A .h header is parsed with the C++ grammar
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_header_is_parsed_with_the_cpp_grammar() -> TestResult {
    // @step Given src/shape.h holding class Shape { public: virtual double area(); };
    let (_directory, root) = project(&[(
        "src/shape.h",
        "class Shape { public: virtual double area(); };\n",
    )])?;

    // @step When ast_index runs for the project
    index(&root).await?;

    // @step Then ast_search lists the class Shape and its method area
    let types = search(&root, "Shape", "Type").await?;
    assert_eq!(names(&types), ["Shape"]);
    assert_eq!(types[0]["typeKind"], "class", "{types:?}");
    let functions = search(&root, "area", "Function").await?;
    assert_eq!(names(&functions), ["area"]);
    assert_eq!(functions[0]["qualifiedName"], "src/shape.h::Shape::area");
    Ok(())
}

/// A Rust file of exactly `bytes` bytes holding `fn name() {}`.
fn sized_rust_file(name: &str, bytes: usize) -> String {
    let mut text = format!("fn {name}() {{}}\n//");
    text.push_str(&"x".repeat(bytes - text.len() - 1));
    text.push('\n');
    text
}

/// Scenario: A file larger than 1 MiB is skipped and reported
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_file_larger_than_the_cap_is_skipped() -> TestResult {
    // @step Given a .rs file of exactly 1,048,576 bytes and a .rs file of 1,048,577 bytes
    let at_cap = sized_rust_file("at_cap", 1_048_576);
    let over_cap = sized_rust_file("over_cap", 1_048_577);
    assert_eq!((at_cap.len(), over_cap.len()), (1_048_576, 1_048_577));
    let (_directory, root) =
        project(&[("src/at_cap.rs", &at_cap), ("src/over_cap.rs", &over_cap)])?;

    // @step When ast_index runs for the project
    let result = index(&root).await?;

    // @step Then the first file is parsed
    assert_eq!(result["files_parsed"], 1, "{result}");
    assert_eq!(names(&search(&root, "", "Function").await?), ["at_cap"]);

    // @step And files_skipped lists the second file with reason too_large
    assert_eq!(
        result["files_skipped"],
        serde_json::json!([{ "path": "src/over_cap.rs", "reason": "too_large" }])
    );
    Ok(())
}

/// Scenario: A symbolic link is not followed and is reported
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_symlink_is_not_followed_and_is_reported() -> TestResult {
    // @step Given src/real.rs and a symbolic link src/link.rs to it
    let (_directory, root) = project(&[("src/real.rs", "fn real() {}\n")])?;
    std::os::unix::fs::symlink(root.join("src/real.rs"), root.join("src/link.rs"))?;

    // @step When ast_index runs for the project
    let result = index(&root).await?;

    // @step Then only src/real.rs is parsed
    assert_eq!(result["files_parsed"], 1, "{result}");
    let all = entities(&root).await?;
    let file_paths: Vec<String> = nodes(&all, "File")
        .into_iter()
        .filter_map(|(_, p)| p.get("path")?.as_str().map(str::to_string))
        .collect();
    assert_eq!(file_paths, ["src/real.rs"]);

    // @step And files_skipped lists src/link.rs with reason symlink
    assert_eq!(
        result["files_skipped"],
        serde_json::json!([{ "path": "src/link.rs", "reason": "symlink" }])
    );
    Ok(())
}
