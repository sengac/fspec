//! KGRAPH-075: the global deterministic resolve pass, with exact Rust
//! module resolution.
//!
//! Feature: spec/features/global-deterministic-resolve-pass.feature
//!
//! Every scenario indexes a temp project with `ast_index` and reads the
//! graph back; no edge is hand-written.

use codelet_graph::ast_index::dispatch_ast_index;
use codelet_graph::ast_transitive::dispatch_ast_callers;
use codelet_graph::graph_entities::GraphEntity;
use codelet_graph::registry;
use codelet_test_helpers::TestResult;
use serde_json::Value;

pub mod code_graph_fixture;
use code_graph_fixture::{
    callees_of, edges, entities, index, project, slug_of, unresolved, CRATE_MANIFEST,
};

/// The (from, to) qualifiedName pairs of every `edge_type` edge.
fn named_edges(all: &[GraphEntity], edge_type: &str) -> Vec<(String, String)> {
    let names: std::collections::BTreeMap<String, String> = all
        .iter()
        .filter_map(|e| match e {
            GraphEntity::Node {
                slug, properties, ..
            } => Some((
                slug.clone(),
                properties.get("qualifiedName")?.as_str()?.to_string(),
            )),
            _ => None,
        })
        .collect();
    let mut found: Vec<(String, String)> = edges(all, edge_type)
        .into_iter()
        .filter_map(|(from, to, _)| Some((names.get(&from)?.clone(), names.get(&to)?.clone())))
        .collect();
    found.sort();
    found
}

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect()
}

/// Scenario: A call into another workspace crate resolves through its Cargo manifests
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_cross_crate_call_resolves_through_manifests() -> TestResult {
    // @step Given a Cargo workspace with crates app and util, where app depends on util by a workspace path dependency
    let workspace = "[workspace]\nmembers = [\"app\", \"util\"]\n[workspace.dependencies]\nutil = { path = \"util\" }\n";
    let app = "[package]\nname = \"app\"\n[dependencies]\nutil = { workspace = true }\n";
    let util = "[package]\nname = \"util\"\n";

    // @step And app's fn main calls util::parse()
    let (_directory, root) = project(&[
        ("Cargo.toml", workspace),
        ("app/Cargo.toml", app),
        ("util/Cargo.toml", util),
        ("app/src/main.rs", "fn main() { util::parse(); }\n"),
        ("util/src/lib.rs", "pub fn parse() {}\n"),
    ])?;

    // @step When ast_index runs for the project
    index(&root).await?;

    // @step Then ast_callers of parse lists main
    let all = entities(&root).await?;
    let parse = slug_of(&all, "Function", "util/src/lib.rs::parse")?;
    let graph = registry::read_code_graph(&root).await?;
    let output = dispatch_ast_callers(&graph, &parse, Some(1), None).await;
    let value: Value = serde_json::from_str(&output)?;
    let callers: Vec<&str> = value["results"]
        .as_array()
        .map(|rows| rows.iter().filter_map(|r| r["name"].as_str()).collect())
        .unwrap_or_default();
    assert_eq!(callers, ["main"], "{output}");
    Ok(())
}

/// Scenario: Nested modules resolve through mod declarations and count every call site
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn nested_modules_resolve_and_count_call_sites() -> TestResult {
    // @step Given a crate with src/lib.rs declaring mod a, src/a.rs declaring mod b and src/a/b.rs holding pub fn deep()
    // @step And a function in src/lib.rs that calls a::b::deep() and crate::a::b::deep()
    let (_directory, root) = project(&[
        CRATE_MANIFEST,
        (
            "src/lib.rs",
            "mod a;\npub fn run() { a::b::deep(); crate::a::b::deep(); }\n",
        ),
        ("src/a.rs", "pub mod b;\n"),
        ("src/a/b.rs", "pub fn deep() {}\n"),
    ])?;

    // @step When ast_index runs for the project
    index(&root).await?;

    // @step Then the function has one Calls edge to deep with callCount 2
    let all = entities(&root).await?;
    let run = slug_of(&all, "Function", "src/lib.rs::run")?;
    let deep = slug_of(&all, "Function", "src/a/b.rs::deep")?;
    let calls: Vec<_> = edges(&all, "Calls")
        .into_iter()
        .filter(|(from, _, _)| *from == run)
        .collect();
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert_eq!(calls[0].1, deep);
    assert_eq!(calls[0].2.get("callCount"), Some(&Value::from(2)));
    Ok(())
}

/// Scenario: Grouped and renamed use declarations bind names exactly
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn grouped_and_renamed_uses_bind_exactly() -> TestResult {
    // @step Given src/lib.rs with use crate::a::{b::deep, other as renamed}; and a function calling deep() and renamed()
    let (_directory, root) = project(&[
        CRATE_MANIFEST,
        (
            "src/lib.rs",
            "mod a;\nuse crate::a::{b::deep, other as renamed};\npub fn run() { deep(); renamed(); }\n",
        ),
        ("src/a.rs", "pub mod b;\npub fn other() {}\n"),
        ("src/a/b.rs", "pub fn deep() {}\n"),
    ])?;

    // @step When ast_index runs for the project
    index(&root).await?;

    // @step Then the function calls deep in src/a/b.rs and other in src/a.rs
    let all = entities(&root).await?;
    let run = slug_of(&all, "Function", "src/lib.rs::run")?;
    assert_eq!(
        callees_of(&all, &run),
        ["src/a.rs::other", "src/a/b.rs::deep"]
    );
    Ok(())
}

/// Scenario: super and self paths resolve relative to the module
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn super_and_self_paths_resolve_relative_to_the_module() -> TestResult {
    // @step Given src/a/b.rs calling super::sibling() and self::local()
    let (_directory, root) = project(&[
        CRATE_MANIFEST,
        ("src/lib.rs", "mod a;\n"),
        ("src/a.rs", "mod b;\nfn sibling() {}\n"),
        (
            "src/a/b.rs",
            "fn caller() { super::sibling(); self::local(); }\nfn local() {}\n",
        ),
    ])?;

    // @step When ast_index runs for the project
    index(&root).await?;

    // @step Then the caller calls sibling in src/a.rs and local in src/a/b.rs
    let all = entities(&root).await?;
    let caller = slug_of(&all, "Function", "src/a/b.rs::caller")?;
    assert_eq!(
        callees_of(&all, &caller),
        ["src/a.rs::sibling", "src/a/b.rs::local"]
    );
    Ok(())
}

/// Scenario: A pub use re-export is followed to its definition
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_pub_use_reexport_is_followed() -> TestResult {
    // @step Given src/lib.rs with pub use inner::Thing; and impl Thing { pub fn new() } in src/inner.rs
    // @step And a function in another module calling crate::Thing::new()
    let (_directory, root) = project(&[
        CRATE_MANIFEST,
        (
            "src/lib.rs",
            "mod inner;\nmod other;\npub use inner::Thing;\n",
        ),
        (
            "src/inner.rs",
            "pub struct Thing;\nimpl Thing { pub fn new() -> Thing { Thing } }\n",
        ),
        ("src/other.rs", "fn make() { crate::Thing::new(); }\n"),
    ])?;

    // @step When ast_index runs for the project
    index(&root).await?;

    // @step Then the function calls new in src/inner.rs
    let all = entities(&root).await?;
    let make = slug_of(&all, "Function", "src/other.rs::make")?;
    assert_eq!(callees_of(&all, &make), ["src/inner.rs::Thing::new"]);
    Ok(())
}

/// Scenario: self method calls resolve to the enclosing impl's type
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn self_method_calls_resolve_to_the_impl_type() -> TestResult {
    // @step Given impl Engine { fn start(&self) { self.run() } fn run(&self) {} }
    let (_directory, root) = project(&[
        CRATE_MANIFEST,
        (
            "src/lib.rs",
            "pub struct Engine;\nimpl Engine { fn start(&self) { self.run() } fn run(&self) {} }\nfn free() {}\n",
        ),
    ])?;

    // @step When ast_index runs for the project
    index(&root).await?;

    // @step Then start calls run
    let all = entities(&root).await?;
    let start = slug_of(&all, "Function", "src/lib.rs::Engine::start")?;
    assert_eq!(callees_of(&all, &start), ["src/lib.rs::Engine::run"]);

    // @step And Engine has HasMethod edges to start and run only
    assert_eq!(
        named_edges(&all, "HasMethod"),
        pairs(&[
            ("src/lib.rs::Engine", "src/lib.rs::Engine::run"),
            ("src/lib.rs::Engine", "src/lib.rs::Engine::start")
        ])
    );
    Ok(())
}

/// Scenario: An ambiguous trait method call is never guessed
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_ambiguous_trait_method_call_is_never_guessed() -> TestResult {
    // @step Given impls of traits P and Q for Engine that both define fn go, and an Engine method calling self.go()
    let (_directory, root) = project(&[
        CRATE_MANIFEST,
        (
            "src/lib.rs",
            "pub trait P { fn go(&self); }\npub trait Q { fn go(&self); }\npub struct Engine;\nimpl P for Engine { fn go(&self) {} }\nimpl Q for Engine { fn go(&self) {} }\nimpl Engine { fn start(&self) { self.go(); } }\n",
        ),
    ])?;

    // @step When ast_index runs for the project
    let result = index(&root).await?;

    // @step Then no Calls edge leads to either go
    let all = entities(&root).await?;
    assert_eq!(named_edges(&all, "Calls"), Vec::<(String, String)>::new());

    // @step And the index result counts one unresolved call with reason ambiguous
    assert_eq!(unresolved(&result, "calls", "ambiguous"), 1, "{result}");
    Ok(())
}

/// Scenario: Calls on unknown receivers and external paths are unresolved with their reason
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unknown_receivers_and_external_paths_are_unresolved() -> TestResult {
    // @step Given a function calling value.len() on a local variable and std::mem::swap(a, b)
    let (_directory, root) = project(&[
        CRATE_MANIFEST,
        (
            "src/lib.rs",
            "pub fn run(mut a: u8, mut b: u8) { let value = [1]; value.len(); std::mem::swap(&mut a, &mut b); }\n",
        ),
    ])?;

    // @step When ast_index runs for the project
    let result = index(&root).await?;

    // @step Then the index result counts one unresolved call with reason receiver_type_unknown and one with reason external
    assert_eq!(
        unresolved(&result, "calls", "receiver_type_unknown"),
        1,
        "{result}"
    );
    assert_eq!(unresolved(&result, "calls", "external"), 1, "{result}");
    Ok(())
}

/// Scenario: Calls inside a macro token tree are unresolved
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn calls_inside_a_macro_token_tree_are_unresolved() -> TestResult {
    // @step Given a test calling assert_eq!(compute(1), 2)
    let (_directory, root) = project(&[
        CRATE_MANIFEST,
        (
            "src/lib.rs",
            "fn compute(x: u8) -> u8 { x }\n#[test]\nfn checks() { assert_eq!(compute(1), 2); }\n",
        ),
    ])?;

    // @step When ast_index runs for the project
    let result = index(&root).await?;

    // @step Then no Calls edge leads to compute
    let all = entities(&root).await?;
    assert_eq!(named_edges(&all, "Calls"), Vec::<(String, String)>::new());

    // @step And the index result counts one unresolved call with reason macro_token_tree
    assert_eq!(
        unresolved(&result, "calls", "macro_token_tree"),
        1,
        "{result}"
    );
    Ok(())
}

/// Scenario: A file outside every crate's module tree is not resolved
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_file_outside_every_module_tree_is_not_resolved() -> TestResult {
    // @step Given a .rs file under the root that no crate's mod tree reaches, calling fn helper
    let (_directory, root) = project(&[
        CRATE_MANIFEST,
        ("src/lib.rs", "pub fn lib() {}\n"),
        (
            "scripts/loose.rs",
            "fn helper() {}\nfn caller() { helper(); }\n",
        ),
    ])?;

    // @step When ast_index runs for the project
    let result = index(&root).await?;

    // @step Then its call is counted as unresolved with reason not_in_module_tree
    assert_eq!(
        unresolved(&result, "calls", "not_in_module_tree"),
        1,
        "{result}"
    );
    let all = entities(&root).await?;
    assert_eq!(named_edges(&all, "Calls"), Vec::<(String, String)>::new());
    Ok(())
}

/// Scenario: Rust trait implementations and supertraits give Implements and Extends
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn trait_impls_and_supertraits_give_implements_and_extends() -> TestResult {
    // @step Given impl Display for Engine where Display is a trait under the root, impl std::fmt::Debug for Engine, and trait Child: Parent
    let (_directory, root) = project(&[
        CRATE_MANIFEST,
        (
            "src/lib.rs",
            "pub trait Display { fn show(&self); }\npub trait Parent {}\npub trait Child: Parent {}\npub struct Engine;\nimpl Display for Engine { fn show(&self) {} }\nimpl std::fmt::Debug for Engine { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { Ok(()) } }\n",
        ),
    ])?;

    // @step When ast_index runs for the project
    let result = index(&root).await?;

    // @step Then the graph holds Implements Engine -> Display and Extends Child -> Parent
    let all = entities(&root).await?;
    assert_eq!(
        named_edges(&all, "Implements"),
        pairs(&[("src/lib.rs::Engine", "src/lib.rs::Display")])
    );
    assert_eq!(
        named_edges(&all, "Extends"),
        pairs(&[("src/lib.rs::Child", "src/lib.rs::Parent")])
    );

    // @step And no edge leads from Engine to Debug
    assert_eq!(unresolved(&result, "bases", "external"), 1, "{result}");
    Ok(())
}

/// Scenario: Class bases in Python and TypeScript give Extends and Implements
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn class_bases_in_python_and_typescript() -> TestResult {
    // @step Given Python class Dog(Animal) importing Animal by from .animals import Animal
    // @step And TypeScript class A extends B implements I with B and I imported from ./base
    let (_directory, root) = project(&[
        ("pets/__init__.py", ""),
        ("pets/animals.py", "class Animal:\n    pass\n"),
        (
            "pets/dog.py",
            "from .animals import Animal\n\nclass Dog(Animal):\n    pass\n",
        ),
        ("web/base.ts", "export class B {}\nexport interface I {}\n"),
        (
            "web/a.ts",
            "import { B, I } from './base';\nexport class A extends B implements I {}\n",
        ),
    ])?;

    // @step When ast_index runs for the project
    index(&root).await?;

    // @step Then the graph holds Extends Dog -> Animal, Extends A -> B and Implements A -> I
    let all = entities(&root).await?;
    assert_eq!(
        named_edges(&all, "Extends"),
        pairs(&[
            ("pets/dog.py::Dog", "pets/animals.py::Animal"),
            ("web/a.ts::A", "web/base.ts::B")
        ])
    );
    assert_eq!(
        named_edges(&all, "Implements"),
        pairs(&[("web/a.ts::A", "web/base.ts::I")])
    );
    Ok(())
}

/// Scenario: Imported names resolve calls in Python and TypeScript
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn imported_names_resolve_calls_in_python_and_typescript() -> TestResult {
    // @step Given Python from .helpers import tidy followed by tidy()
    // @step And TypeScript import { tidy } from './helpers' followed by tidy()
    let (_directory, root) = project(&[
        ("pkg/__init__.py", ""),
        ("pkg/helpers.py", "def tidy():\n    pass\n"),
        (
            "pkg/main.py",
            "from .helpers import tidy\n\ndef run():\n    tidy()\n",
        ),
        ("web/helpers.ts", "export function tidy() {}\n"),
        (
            "web/main.ts",
            "import { tidy } from './helpers';\nexport function run() { tidy(); }\n",
        ),
    ])?;

    // @step When ast_index runs for the project
    index(&root).await?;

    // @step Then each caller calls tidy in its own language's helpers file
    let all = entities(&root).await?;
    assert_eq!(
        named_edges(&all, "Calls"),
        pairs(&[
            ("pkg/main.py::run", "pkg/helpers.py::tidy"),
            ("web/main.ts::run", "web/helpers.ts::tidy")
        ])
    );
    Ok(())
}

/// Scenario: A call inside a decision node is conditional
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_call_inside_a_decision_node_is_conditional() -> TestResult {
    // @step Given a caller whose only call to go is inside if ready { go() } and a call to stay() outside any decision node
    let (_directory, root) = project(&[
        CRATE_MANIFEST,
        (
            "src/lib.rs",
            "fn go() {}\nfn stay() {}\npub fn caller(ready: bool) { if ready { go() } stay(); }\n",
        ),
    ])?;

    // @step When ast_index runs for the project
    index(&root).await?;

    // @step Then the Calls edge to go has isConditional true and the Calls edge to stay has isConditional false
    let all = entities(&root).await?;
    let go = slug_of(&all, "Function", "src/lib.rs::go")?;
    let stay = slug_of(&all, "Function", "src/lib.rs::stay")?;
    let conditional = |target: &str| {
        edges(&all, "Calls")
            .into_iter()
            .find(|(_, to, _)| to == target)
            .and_then(|(_, _, p)| p.get("isConditional").cloned())
    };
    assert_eq!(conditional(&go), Some(Value::Bool(true)));
    assert_eq!(conditional(&stay), Some(Value::Bool(false)));
    Ok(())
}

/// Scenario: The same tree always gives the same graph
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_same_tree_always_gives_the_same_graph() -> TestResult {
    // @step Given a multi-language fixture with a Cargo workspace
    let (_directory, root) = project(&[
        ("Cargo.toml", "[workspace]\nmembers = [\"app\", \"util\"]\n"),
        (
            "app/Cargo.toml",
            "[package]\nname = \"app\"\n[dependencies]\nutil = { path = \"../util\" }\n",
        ),
        ("util/Cargo.toml", "[package]\nname = \"util\"\n"),
        (
            "app/src/main.rs",
            "use util::Engine;\nfn main() { util::parse(); Engine::start(); }\n",
        ),
        (
            "util/src/lib.rs",
            "mod engine;\npub use engine::Engine;\npub fn parse() { helper(); }\nfn helper() {}\n",
        ),
        (
            "util/src/engine.rs",
            "pub struct Engine;\nimpl Engine { pub fn start() {} }\n",
        ),
        (
            "py/main.py",
            "from .helpers import tidy\n\ndef run():\n    tidy()\n",
        ),
        ("py/helpers.py", "def tidy():\n    pass\n"),
        (
            "web/main.ts",
            "import { tidy } from './helpers';\nexport function run() { tidy(); }\n",
        ),
        ("web/helpers.ts", "export function tidy() {}\n"),
    ])?;

    // @step When it is indexed twice, and once more after its graph is deleted
    let mut runs = Vec::new();
    for reset in [false, false, true] {
        let result: Value = serde_json::from_str(&dispatch_ast_index(&root, reset, false).await)?;
        assert!(result.get("error").is_none(), "{result}");
        let mut all: Vec<String> =
            codelet_graph::graph_entities::entities_to_jsonl(&entities(&root).await?)
                .lines()
                .map(str::to_string)
                .collect();
        all.sort();
        runs.push((all, result["unresolved"].clone()));
    }

    // @step Then the three entity lists are byte-identical
    assert!(runs[0].0.len() > 20, "{} entities", runs[0].0.len());
    assert_eq!(runs[0].0, runs[1].0);
    assert_eq!(runs[0].0, runs[2].0);

    // @step And the three unresolved counts are identical
    assert_eq!(runs[0].1, runs[1].1);
    assert_eq!(runs[0].1, runs[2].1);
    Ok(())
}
