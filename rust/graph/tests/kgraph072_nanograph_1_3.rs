//! KGRAPH-072: codelet-graph runs on nanograph 1.3.0.
//!
//! Feature: spec/features/codelet-graph-runs-on-nanograph-1-3.feature
//!
//! The legacy graphs under tests/fixtures/kgraph072/ were written by
//! nanograph 1.0.0 (codelet-graph at 7fb2449d5): a code graph holding
//! `legacy_fn` and a learnings graph holding one learning. Every scenario
//! works on copies in temp directories.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use codelet_graph::ast_index::dispatch_ast_index;
use codelet_graph::graph_entities::entities_to_jsonl;
use codelet_graph::registry;
use codelet_test_helpers::{setup_temp_directory, TestError, TestResult};
use serde_json::{json, Value};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/kgraph072");

fn copy_tree(from: &Path, to: &Path) -> TestResult {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Every file under `root` with its bytes, keyed by relative path.
fn tree_bytes(root: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>, TestError> {
    fn walk(root: &Path, at: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) -> TestResult {
        for entry in std::fs::read_dir(at)? {
            let path = entry?.path();
            if path.is_dir() {
                walk(root, &path, out)?;
            } else {
                out.insert(
                    path.strip_prefix(root)?.to_path_buf(),
                    std::fs::read(&path)?,
                );
            }
        }
        Ok(())
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out)?;
    Ok(out)
}

fn project(directory: &Path, name: &str) -> Result<PathBuf, TestError> {
    let root = directory.join(name);
    std::fs::create_dir_all(root.join("src"))?;
    std::fs::write(
        root.join("src/service.py"),
        "def handle_request():\n    return parse_body()\n\n\ndef parse_body():\n    return 1\n",
    )?;
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn start() -> u32 {\n    helper()\n}\n\nfn helper() -> u32 {\n    1\n}\n",
    )?;
    Ok(root.canonicalize()?)
}

fn parsed(output: &str) -> Result<Value, TestError> {
    Ok(serde_json::from_str(output)?)
}

fn legacy_project(directory: &Path) -> Result<PathBuf, TestError> {
    let root = project(directory, "legacy")?;
    copy_tree(
        &Path::new(FIXTURES).join("ast-code.nano"),
        &registry::code_graph_path(&root),
    )?;
    Ok(root)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ast_index_rebuilds_a_code_graph_written_by_nanograph_1_0_0() -> TestResult {
    let directory = setup_temp_directory()?;

    // @step Given a project whose code graph on disk was written by nanograph 1.0.0
    let root = legacy_project(directory.path())?;
    let database = registry::code_graph_path(&root);
    assert!(!database.join("storage.generation.json").exists());

    // @step When ast_index runs for the project
    let output = dispatch_ast_index(&root, false, false).await;

    // @step Then the index succeeds
    assert!(parsed(&output)?.get("error").is_none(), "{output}");

    // @step And the code graph on disk is in nanograph 1.3.0 storage
    assert!(database.join("storage.generation.json").is_file());

    // @step And ast_search lists the project's functions
    let graph = registry::read_code_graph(&root).await?;
    let search = codelet_graph::ast_dispatch::dispatch_ast_search(
        &graph,
        "",
        Some("Function"),
        Some(50),
        None,
        None,
        None,
        None,
    )
    .await;
    let mut names: Vec<String> = parsed(&search)?["results"]
        .as_array()
        .ok_or("no results")?
        .iter()
        .filter_map(|row| row["name"].as_str().map(str::to_owned))
        .collect();
    names.sort();
    assert_eq!(names, ["handle_request", "helper", "parse_body", "start"]);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_query_on_a_code_graph_written_by_nanograph_1_0_0_names_the_rebuild() -> TestResult {
    let directory = setup_temp_directory()?;

    // @step Given a project whose code graph on disk was written by nanograph 1.0.0
    let root = legacy_project(directory.path())?;

    // @step When ast_search runs for the project
    let opened = registry::read_code_graph(&root).await;

    // @step Then it fails with an error naming nanograph 1.3.0 and ast_index
    let error = match opened {
        Ok(_) => return Err("a legacy code graph must not open".into()),
        Err(error) => error,
    };
    assert!(error.contains("nanograph 1.3.0"), "{error}");
    assert!(error.contains("ast_index"), "{error}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_learnings_graph_written_by_nanograph_1_0_0_is_backed_up_and_started_fresh() -> TestResult
{
    let data = setup_temp_directory()?;
    codelet_common::set_data_directory(data.path().to_path_buf())?;
    registry::reset_all_graphs();
    let fixture = Path::new(FIXTURES).join("learnings.nano");

    // @step Given the learnings graph on disk was written by nanograph 1.0.0
    let learnings = registry::learnings_graph_path()?;
    copy_tree(&fixture, &learnings)?;

    // @step When learnings_stats runs
    let graph = registry::read_learnings_graph().await?;
    let stats = codelet_graph::learnings_dispatch::dispatch_learnings_stats(&graph).await;

    // @step Then it reports zero learnings from a fresh learnings graph
    assert_eq!(parsed(&stats)?["nodes"]["Learning"], json!(0), "{stats}");
    assert!(learnings.join("storage.generation.json").is_file());

    // @step And the old learnings graph sits beside it at learnings.nano.v3-backup-<timestamp> with its files intact
    let parent = learnings.parent().ok_or("learnings path has no parent")?;
    let backups: Vec<PathBuf> = std::fs::read_dir(parent)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("learnings.nano.v3-backup-"))
        })
        .collect();
    assert_eq!(backups.len(), 1, "{backups:?}");
    assert_eq!(tree_bytes(&backups[0])?, tree_bytes(&fixture)?);
    drop(graph);
    registry::reset_all_graphs();
    Ok(())
}

fn count(entities: &[codelet_graph::graph_entities::GraphEntity], kind: &str) -> u64 {
    use codelet_graph::graph_entities::GraphEntity;
    entities
        .iter()
        .filter(|entity| match entity {
            GraphEntity::Node { node_type, .. } => node_type == kind,
            GraphEntity::Edge { edge_type, .. } => edge_type == kind,
        })
        .count() as u64
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ast_stats_reports_the_counts_the_index_loaded() -> TestResult {
    let directory = setup_temp_directory()?;

    // @step Given a project with Python and Rust sources has been indexed
    let root = project(directory.path(), "stats")?;
    let output = dispatch_ast_index(&root, false, false).await;
    assert!(parsed(&output)?.get("error").is_none(), "{output}");
    let expected = codelet_graph::ast_pipeline::walk_and_extract(&root, true);

    // @step When ast_stats runs
    let graph = registry::read_code_graph(&root).await?;
    let stats = parsed(&codelet_graph::ast_dispatch::dispatch_ast_stats(&graph).await)?;

    // @step Then the File, Function and Contains counts equal the entities the index loaded
    assert_eq!(
        stats["nodes"]["File"],
        json!(count(&expected, "File")),
        "{stats}"
    );
    assert_eq!(
        stats["nodes"]["Function"],
        json!(count(&expected, "Function")),
        "{stats}"
    );
    assert_eq!(
        stats["edges"]["Contains"],
        json!(count(&expected, "Contains")),
        "{stats}"
    );
    assert_eq!(count(&expected, "Function"), 4);
    Ok(())
}

fn sorted_jsonl(entities: &[codelet_graph::graph_entities::GraphEntity]) -> Vec<String> {
    let mut lines: Vec<String> = entities_to_jsonl(entities)
        .lines()
        .map(str::to_owned)
        .collect();
    lines.sort();
    lines
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_exported_code_graph_imports_into_another_root_unchanged() -> TestResult {
    let directory = setup_temp_directory()?;

    // @step Given a project with Python and Rust sources has been indexed
    let root = project(directory.path(), "source")?;
    let output = dispatch_ast_index(&root, false, false).await;
    assert!(parsed(&output)?.get("error").is_none(), "{output}");

    // @step When its graph is exported with ast_export and imported into another root with ast_import
    let bundle = directory.path().join("graph.astbundle");
    let source = registry::read_code_graph(&root).await?;
    source
        .export_bundle(&bundle, registry::AST_CODE_SCHEMA)
        .await?;
    let other = directory.path().join("other");
    std::fs::create_dir_all(&other)?;
    let mut lease = registry::write_code_graph(&other).await?;
    lease
        .database()
        .await?
        .import_bundle(&bundle, registry::AST_CODE_SCHEMA, "overwrite")
        .await?;
    drop(lease);

    // @step Then the other root's graph exports the same entities
    let imported = registry::read_code_graph(&other).await?;
    let before = sorted_jsonl(&source.export_all_entities().await?);
    let after = sorted_jsonl(&imported.export_all_entities().await?);
    assert!(before.len() > 8, "{before:#?}");
    assert_eq!(before, after);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn queries_over_the_contains_edge_run_on_nanograph_1_3_0() -> TestResult {
    let directory = setup_temp_directory()?;

    // @step Given a project with Python and Rust sources has been indexed
    let root = project(directory.path(), "contains")?;
    let output = dispatch_ast_index(&root, false, false).await;
    assert!(parsed(&output)?.get("error").is_none(), "{output}");

    // @step When the file_functions query runs for an indexed file
    let graph = registry::read_code_graph(&root).await?;
    let rows = graph
        .query_with_source(
            codelet_graph::ast_dispatch::AST_QUERIES,
            "file_functions",
            Some(&json!({ "slug": "src/service.py" })),
        )
        .await?;

    // @step Then it returns the functions of that file
    let mut names: Vec<String> = rows
        .as_array()
        .ok_or("file_functions returned no rows")?
        .iter()
        .filter_map(|row| row["name"].as_str().map(str::to_owned))
        .collect();
    names.sort();
    assert_eq!(names, ["handle_request", "parse_body"]);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_query_over_an_edge_type_with_no_edges_returns_no_rows() -> TestResult {
    let directory = setup_temp_directory()?;

    // @step Given a project whose Python files import nothing has been indexed
    let root = project(directory.path(), "no-imports")?;
    let output = dispatch_ast_index(&root, false, false).await;
    assert_eq!(parsed(&output)?["edges"]["Imports"], json!(0), "{output}");

    // @step When the file_imports query runs for one of its files
    let graph = registry::read_code_graph(&root).await?;
    let rows = graph
        .query_with_source(
            codelet_graph::ast_dispatch::AST_QUERIES,
            "file_imports",
            Some(&json!({ "slug": "src/service.py" })),
        )
        .await;

    // @step Then it returns no rows instead of failing
    assert_eq!(rows?, json!([]));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_learnings_edge_survives_loading_another_edge_type_and_a_reopen() -> TestResult {
    use codelet_graph::database::GraphDatabase;
    use codelet_graph::learnings_dispatch::LEARNINGS_QUERIES;
    let directory = setup_temp_directory()?;
    let path = directory.path().join("learnings.nano");
    let learning = |slug: &str| {
        json!({"type": "Learning", "data": {"slug": slug, "title": slug, "content": slug,
            "category": "pattern", "confidence": "high", "firstSeen": "2026-03-22T00:00:00",
            "lastSeen": "2026-03-22T00:00:00", "mentionCount": 1}})
        .to_string()
    };

    // @step Given a learnings graph with an exploration that discovered two learnings
    let db = GraphDatabase::init(&path, registry::LEARNINGS_SCHEMA).await?;
    let nodes = [
        learning("batch-load"),
        learning("registry-pattern"),
        learning("old-pattern"),
        json!({"type": "Exploration", "data": {"slug": "exp-refactor", "title": "t",
            "strategy": "s", "outcome": "success", "createdAt": "2026-03-22T00:00:00"}})
        .to_string(),
    ];
    db.load_jsonl(&nodes.join("\n")).await?;
    db.load_jsonl(concat!(
        r#"{"edge":"Discovered","from":"exp-refactor","to":"registry-pattern","data":{"extractedAt":"2026-03-22T00:00:00"}}"#,
        "\n",
        r#"{"edge":"Discovered","from":"exp-refactor","to":"batch-load","data":{"extractedAt":"2026-03-22T00:00:00"}}"#,
    ))
    .await?;

    // @step When a Supersedes edge between two learnings is loaded
    db.load_jsonl(r#"{"edge":"Supersedes","from":"registry-pattern","to":"old-pattern","data":{"supersededAt":"2026-03-22T00:00:00","reason":"cleaner"}}"#)
        .await?;

    // @step And the learnings graph is reopened
    drop(db);
    let db = GraphDatabase::open(&path).await?;

    // @step Then exploration_discoveries still returns both discovered learnings
    let rows = db
        .query_with_source(
            LEARNINGS_QUERIES,
            "exploration_discoveries",
            Some(&json!({ "slug": "exp-refactor" })),
        )
        .await?;
    let mut slugs: Vec<String> = rows
        .as_array()
        .ok_or("exploration_discoveries returned no rows")?
        .iter()
        .filter_map(|row| row["slug"].as_str().map(str::to_owned))
        .collect();
    slugs.sort();
    assert_eq!(slugs, ["batch-load", "registry-pattern"]);
    Ok(())
}

/// Every nanograph load call in a source file: method calls on the
/// `Database` field (`self.db.load*(..)`), `Database::load*` paths, and every
/// `LoadMode::<variant>` path.
#[derive(Default)]
struct LoadCalls {
    calls: Vec<String>,
    modes: Vec<String>,
}

impl<'ast> syn::visit::Visit<'ast> for LoadCalls {
    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        let method = call.method.to_string();
        let on_database = matches!(&*call.receiver, syn::Expr::Field(field)
            if matches!(&field.member, syn::Member::Named(name) if name == "db"));
        if on_database && method.starts_with("load") {
            let mode = call.args.iter().nth(1).map(|arg| {
                let syn::Expr::Path(path) = arg else {
                    return "<not a path>".to_string();
                };
                path.path
                    .segments
                    .iter()
                    .map(|segment| segment.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::")
            });
            self.calls
                .push(format!("{method}({})", mode.unwrap_or_default()));
        }
        syn::visit::visit_expr_method_call(self, call);
    }

    fn visit_path(&mut self, path: &'ast syn::Path) {
        let names: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
        if let [.., owner, item] = names.as_slice() {
            if owner == "LoadMode" {
                self.modes.push(format!("LoadMode::{item}"));
            }
            if owner == "Database" && item.starts_with("load") {
                self.calls.push(format!("Database::{item}"));
            }
        }
        syn::visit::visit_path(self, path);
    }
}

fn rust_sources(directory: &Path, found: &mut Vec<PathBuf>) -> TestResult {
    for entry in std::fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            rust_sources(&path, found)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
    Ok(())
}

#[test]
fn codelet_graph_never_uses_a_merge_load() -> TestResult {
    use syn::visit::Visit;

    // @step Given the codelet-graph sources
    let mut sources = Vec::new();
    rust_sources(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut sources,
    )?;
    assert!(sources.len() > 20, "{sources:?}");

    // @step When every nanograph load call in them is collected from the parsed syntax tree
    let mut found = LoadCalls::default();
    for source in &sources {
        let file = syn::parse_file(&std::fs::read_to_string(source)?)?;
        found.visit_file(&file);
    }

    // @step Then the only load call is load_with_mode with LoadMode::Overwrite
    assert_eq!(found.calls, ["load_with_mode(LoadMode::Overwrite)"]);

    // @step And no Database::load call and no LoadMode::Merge or LoadMode::Append appears
    assert!(
        found.modes.iter().all(|mode| mode == "LoadMode::Overwrite"),
        "{:?}",
        found.modes
    );
    Ok(())
}
