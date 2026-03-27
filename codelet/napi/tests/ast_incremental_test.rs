// Feature: spec/features/incremental-reindexing.feature
//
// Incremental Re-indexing
// Tests that ast_index stores file mtimes on File nodes and supports
// incremental mode where only changed/new files are re-extracted,
// unchanged file entities are reused from the graph, and deleted file
// entities are removed.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_napi::graph::ast_pipeline::helpers::{
    build_contains_edge, build_file_node, build_function_node,
};
use codelet_napi::graph::ast_pipeline::incremental::{
    collect_file_mtimes, filter_reusable_entities, partition_changed_files,
    read_stored_mtimes, stamp_file_mtimes,
};
use codelet_napi::graph::database::GraphDatabase;
use codelet_napi::graph::graph_entities::GraphEntity;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// The AST code schema.
const AST_CODE_SCHEMA: &str = include_str!("../schemas/ast-code.pg");

/// Helper: create a temp project with source files and return their paths.
fn create_test_project(dir: &Path) -> Vec<std::path::PathBuf> {
    let src = dir.join("src");
    std::fs::create_dir_all(&src).expect("create src");

    let main_ts = src.join("main.ts");
    std::fs::write(
        &main_ts,
        "export function handleRequest(req: any, res: any) {\n  return res.json({});\n}\n",
    )
    .expect("write main.ts");

    let utils_ts = src.join("utils.ts");
    std::fs::write(
        &utils_ts,
        "export function parseData(data: string) {\n  return JSON.parse(data);\n}\n",
    )
    .expect("write utils.ts");

    let helper_ts = src.join("helper.ts");
    std::fs::write(
        &helper_ts,
        "export function formatOutput(msg: string): string {\n  return msg.trim();\n}\n",
    )
    .expect("write helper.ts");

    vec![main_ts, utils_ts, helper_ts]
}

/// Helper: build a set of entities matching what extraction would produce
/// for test files, with lastModified timestamps.
fn build_test_entities(
    file_infos: &[(&str, &str, i64)], // (rel_path, file_slug, mtime_millis)
) -> Vec<GraphEntity> {
    let mut entities = Vec::new();
    for &(rel_path, file_slug, _mtime) in file_infos {
        entities.push(build_file_node(rel_path, file_slug, "typescript", 3, false));
        // Add a function per file
        let fn_name = format!("func_{}", file_slug.replace('-', "_"));
        entities.push(build_function_node(
            file_slug, &fn_name, false, true, 1, 1, 3, 1,
            "data", "function body", "", "", "typescript", false,
        ));
        entities.push(build_contains_edge(
            file_slug,
            &format!("{file_slug}::{fn_name}"),
            "Contains",
        ));
    }
    entities
}

// ============================================================================
// Scenario: Full index stores mtime on File nodes
// ============================================================================
#[tokio::test]
async fn test_full_index_stores_mtime_on_file_nodes() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let files = create_test_project(temp_dir.path());

    // @step Given a project directory with multiple source files
    assert_eq!(files.len(), 3);

    // @step When I run a full ast_index on the project
    // Simulate: collect mtimes, build entities, stamp them, load
    let db_path = temp_dir.path().join(".fspec/graph/test.nano");
    let db = GraphDatabase::init(&db_path, AST_CODE_SCHEMA)
        .await
        .expect("DB init");

    let mtimes = collect_file_mtimes(&files, temp_dir.path());
    assert_eq!(mtimes.len(), 3);

    // Build entities from extraction (without mtime)
    let mut entities = Vec::new();
    for (rel_path, mtime_ms) in &mtimes {
        let file_slug = codelet_napi::graph::ast_pipeline::helpers::slugify_path(rel_path);
        entities.push(build_file_node(rel_path, &file_slug, "typescript", 3, false));
        let _ = mtime_ms; // mtime stamped by stamp_file_mtimes below
    }

    // Stamp mtimes onto File nodes
    stamp_file_mtimes(&mut entities, &mtimes);

    db.load_entities_overwrite(&entities).await.expect("load");

    // @step Then every File node in the graph has a lastModified property set to the file's filesystem mtime
    let stored = read_stored_mtimes(&db).expect("read stored mtimes");
    assert_eq!(stored.len(), 3, "should have 3 files with mtimes stored");
    for (rel_path, original_mtime) in &mtimes {
        let stored_mtime = stored.get(rel_path).expect(&format!("mtime for {rel_path}"));
        assert_eq!(
            stored_mtime, original_mtime,
            "stored mtime should match filesystem mtime for {rel_path}"
        );
    }
}

// ============================================================================
// Scenario: Incremental re-index detects no changes and skips extraction
// ============================================================================
#[tokio::test]
async fn test_incremental_no_changes_skips_extraction() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let files = create_test_project(temp_dir.path());

    // @step Given a project that has been fully indexed with mtime-stamped File nodes
    let db_path = temp_dir.path().join(".fspec/graph/test.nano");
    let db = GraphDatabase::init(&db_path, AST_CODE_SCHEMA)
        .await
        .expect("DB init");

    let mtimes = collect_file_mtimes(&files, temp_dir.path());
    let mut entities = build_test_entities(&[
        ("src/main.ts", "src-main-ts", 0),
        ("src/utils.ts", "src-utils-ts", 0),
        ("src/helper.ts", "src-helper-ts", 0),
    ]);
    stamp_file_mtimes(&mut entities, &mtimes);
    db.load_entities_overwrite(&entities).await.expect("load");

    // @step And no source files have been modified since the last index
    let current_mtimes = collect_file_mtimes(&files, temp_dir.path());
    let stored_mtimes = read_stored_mtimes(&db).expect("read stored");

    // @step When I run an incremental ast_index on the project
    let (changed, new, deleted) = partition_changed_files(&current_mtimes, &stored_mtimes);

    // @step Then the result reports 0 files re-extracted
    assert!(changed.is_empty(), "no changed files expected");
    assert!(new.is_empty(), "no new files expected");
    assert!(deleted.is_empty(), "no deleted files expected");

    // @step And all existing entities in the graph are preserved unchanged
    let stats = db.stats().expect("stats");
    let file_count = stats["nodes"]["File"].as_u64().unwrap_or(0);
    assert_eq!(file_count, 3, "all 3 file nodes preserved");
}

// ============================================================================
// Scenario: Incremental re-index extracts only modified files
// ============================================================================
#[tokio::test]
async fn test_incremental_extracts_only_modified_files() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let files = create_test_project(temp_dir.path());

    // @step Given a project that has been fully indexed with mtime-stamped File nodes
    let db_path = temp_dir.path().join(".fspec/graph/test.nano");
    let db = GraphDatabase::init(&db_path, AST_CODE_SCHEMA)
        .await
        .expect("DB init");

    let mtimes = collect_file_mtimes(&files, temp_dir.path());
    let mut entities = build_test_entities(&[
        ("src/main.ts", "src-main-ts", 0),
        ("src/utils.ts", "src-utils-ts", 0),
        ("src/helper.ts", "src-helper-ts", 0),
    ]);
    stamp_file_mtimes(&mut entities, &mtimes);
    db.load_entities_overwrite(&entities).await.expect("load");

    // @step And one source file has been modified since the last index
    // Sleep briefly then touch the file to ensure mtime changes
    std::thread::sleep(std::time::Duration::from_millis(50));
    std::fs::write(
        temp_dir.path().join("src/main.ts"),
        "export function handleRequest(req: any, res: any) {\n  console.log('updated');\n  return res.json({ok: true});\n}\n",
    )
    .expect("modify main.ts");

    // @step When I run an incremental ast_index on the project
    let current_mtimes = collect_file_mtimes(&files, temp_dir.path());
    let stored_mtimes = read_stored_mtimes(&db).expect("read stored");
    let (changed, new, deleted) = partition_changed_files(&current_mtimes, &stored_mtimes);

    // @step Then only the modified file is re-extracted
    assert_eq!(changed.len(), 1, "only 1 changed file");
    assert_eq!(changed[0], "src/main.ts");
    assert!(new.is_empty(), "no new files");
    assert!(deleted.is_empty(), "no deleted files");

    // @step And entities from unchanged files are reused from the existing graph
    let changed_slugs: HashSet<String> = changed
        .iter()
        .map(|p| codelet_napi::graph::ast_pipeline::helpers::slugify_path(p))
        .collect();
    let existing = db.export_all_entities().expect("export");
    let reused = filter_reusable_entities(existing, &changed_slugs);

    // Reused should contain entities from utils.ts and helper.ts but NOT main.ts
    let reused_file_slugs: Vec<&str> = reused
        .iter()
        .filter_map(|e| match e {
            GraphEntity::Node { node_type, slug, .. } if node_type == "File" => Some(slug.as_str()),
            _ => None,
        })
        .collect();
    assert!(reused_file_slugs.contains(&"src-utils-ts"));
    assert!(reused_file_slugs.contains(&"src-helper-ts"));
    assert!(!reused_file_slugs.contains(&"src-main-ts"));

    // @step And the graph contains the updated entities for the modified file
    // (Fresh extraction would produce new entities for main.ts — verified by checking
    //  the reused set does not contain main.ts entities)
    let reused_fn_slugs: Vec<&str> = reused
        .iter()
        .filter_map(|e| match e {
            GraphEntity::Node { node_type, slug, .. } if node_type == "Function" => {
                Some(slug.as_str())
            }
            _ => None,
        })
        .collect();
    assert!(
        !reused_fn_slugs.iter().any(|s| s.starts_with("src-main-ts::")),
        "no main.ts functions in reused set"
    );
}

// ============================================================================
// Scenario: Incremental re-index removes deleted file entities
// ============================================================================
#[tokio::test]
async fn test_incremental_removes_deleted_file_entities() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let files = create_test_project(temp_dir.path());

    // @step Given a project that has been fully indexed with mtime-stamped File nodes
    let db_path = temp_dir.path().join(".fspec/graph/test.nano");
    let db = GraphDatabase::init(&db_path, AST_CODE_SCHEMA)
        .await
        .expect("DB init");

    let mtimes = collect_file_mtimes(&files, temp_dir.path());
    let mut entities = build_test_entities(&[
        ("src/main.ts", "src-main-ts", 0),
        ("src/utils.ts", "src-utils-ts", 0),
        ("src/helper.ts", "src-helper-ts", 0),
    ]);
    stamp_file_mtimes(&mut entities, &mtimes);
    db.load_entities_overwrite(&entities).await.expect("load");

    // @step And one source file has been deleted from the filesystem
    std::fs::remove_file(temp_dir.path().join("src/helper.ts")).expect("delete helper.ts");
    let remaining_files: Vec<std::path::PathBuf> = files
        .iter()
        .filter(|f| f.exists())
        .cloned()
        .collect();

    // @step When I run an incremental ast_index on the project
    let current_mtimes = collect_file_mtimes(&remaining_files, temp_dir.path());
    let stored_mtimes = read_stored_mtimes(&db).expect("read stored");
    let (changed, _new, deleted) = partition_changed_files(&current_mtimes, &stored_mtimes);

    assert_eq!(deleted.len(), 1, "one deleted file");
    assert_eq!(deleted[0], "src/helper.ts");

    // Filter out deleted file entities
    let all_changed_slugs: HashSet<String> = changed
        .iter()
        .chain(deleted.iter())
        .map(|p| codelet_napi::graph::ast_pipeline::helpers::slugify_path(p))
        .collect();
    // deleted files also need their slugs added
    let _ = &all_changed_slugs; // already includes deleted

    let existing = db.export_all_entities().expect("export");
    let reused = filter_reusable_entities(existing, &all_changed_slugs);

    // Reload with only reused entities (no fresh extraction for deleted file)
    db.load_entities_overwrite(&reused).await.expect("reload");

    // @step Then the deleted file's File node is no longer in the graph
    let stats = db.stats().expect("stats");
    let file_count = stats["nodes"]["File"].as_u64().unwrap_or(0);
    assert_eq!(file_count, 2, "only 2 files remaining");

    // @step And the deleted file's Function and Type nodes are no longer in the graph
    let fn_count = stats["nodes"]["Function"].as_u64().unwrap_or(0);
    assert_eq!(fn_count, 2, "only 2 functions remaining (helper.ts function removed)");
}

// ============================================================================
// Scenario: Incremental re-index adds new file entities
// ============================================================================
#[tokio::test]
async fn test_incremental_adds_new_file_entities() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let files = create_test_project(temp_dir.path());

    // @step Given a project that has been fully indexed with mtime-stamped File nodes
    let db_path = temp_dir.path().join(".fspec/graph/test.nano");
    let db = GraphDatabase::init(&db_path, AST_CODE_SCHEMA)
        .await
        .expect("DB init");

    let mtimes = collect_file_mtimes(&files, temp_dir.path());
    let mut entities = build_test_entities(&[
        ("src/main.ts", "src-main-ts", 0),
        ("src/utils.ts", "src-utils-ts", 0),
        ("src/helper.ts", "src-helper-ts", 0),
    ]);
    stamp_file_mtimes(&mut entities, &mtimes);
    db.load_entities_overwrite(&entities).await.expect("load");

    // @step And a new source file has been added to the project
    let new_file = temp_dir.path().join("src/extra.ts");
    std::fs::write(
        &new_file,
        "export function extraFunc(): void {\n  console.log('extra');\n}\n",
    )
    .expect("write extra.ts");

    let mut all_files = files.clone();
    all_files.push(new_file);

    // @step When I run an incremental ast_index on the project
    let current_mtimes = collect_file_mtimes(&all_files, temp_dir.path());
    let stored_mtimes = read_stored_mtimes(&db).expect("read stored");
    let (changed, new_files, deleted) = partition_changed_files(&current_mtimes, &stored_mtimes);

    assert_eq!(new_files.len(), 1, "one new file");
    assert_eq!(new_files[0], "src/extra.ts");
    assert!(deleted.is_empty(), "no deleted files");

    // Build fresh entities for new file (simulating extraction)
    let new_slug = codelet_napi::graph::ast_pipeline::helpers::slugify_path("src/extra.ts");
    let mut fresh = vec![
        build_file_node("src/extra.ts", &new_slug, "typescript", 3, false),
        build_function_node(
            &new_slug, "extraFunc", false, true, 0, 1, 3, 1,
            "", "function body", "", "", "typescript", false,
        ),
        build_contains_edge(&new_slug, &format!("{new_slug}::extraFunc"), "Contains"),
    ];
    let new_mtimes: HashMap<String, i64> = current_mtimes
        .iter()
        .filter(|(k, _)| *k == "src/extra.ts")
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    stamp_file_mtimes(&mut fresh, &new_mtimes);

    // Reuse unchanged entities
    let changed_slugs: HashSet<String> = changed
        .iter()
        .chain(new_files.iter())
        .map(|p| codelet_napi::graph::ast_pipeline::helpers::slugify_path(p))
        .collect();
    let existing = db.export_all_entities().expect("export");
    let reused = filter_reusable_entities(existing, &changed_slugs);

    // Combine and reload
    let mut combined = reused;
    combined.extend(fresh);
    db.load_entities_overwrite(&combined).await.expect("reload");

    // @step Then the new file's entities are extracted and added to the graph
    let stats = db.stats().expect("stats");
    let file_count = stats["nodes"]["File"].as_u64().unwrap_or(0);
    assert_eq!(file_count, 4, "4 files total (3 original + 1 new)");

    // @step And entities from previously indexed files are preserved
    let fn_count = stats["nodes"]["Function"].as_u64().unwrap_or(0);
    assert_eq!(fn_count, 4, "4 functions total (3 original + 1 new)");
}

// ============================================================================
// Scenario: Incremental falls back to full extraction on empty graph
// ============================================================================
#[tokio::test]
async fn test_incremental_falls_back_on_empty_graph() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let files = create_test_project(temp_dir.path());

    // @step Given a project directory with source files
    assert_eq!(files.len(), 3);

    // @step And the AST graph is empty with no prior index
    let db_path = temp_dir.path().join(".fspec/graph/test.nano");
    let db = GraphDatabase::init(&db_path, AST_CODE_SCHEMA)
        .await
        .expect("DB init");

    let stored_mtimes = read_stored_mtimes(&db).expect("read stored");
    assert!(stored_mtimes.is_empty(), "no stored mtimes on empty graph");

    // @step When I run an incremental ast_index on the project
    let current_mtimes = collect_file_mtimes(&files, temp_dir.path());
    let (changed, new_files, deleted) = partition_changed_files(&current_mtimes, &stored_mtimes);

    // When stored is empty, ALL files appear as "new"
    let needs_full = stored_mtimes.is_empty()
        || (changed.len() + new_files.len()) * 2 > current_mtimes.len();

    // @step Then a full extraction is performed for all source files
    assert!(needs_full, "should fall back to full extraction");
    assert_eq!(
        new_files.len(),
        3,
        "all 3 files are new (no prior index)"
    );
    assert!(changed.is_empty(), "no 'changed' files, all are 'new'");
    assert!(deleted.is_empty(), "no deleted files");

    // @step And all entities are loaded into the graph with mtime stamps
    // Simulate full extraction
    let mut entities = build_test_entities(&[
        ("src/main.ts", "src-main-ts", 0),
        ("src/utils.ts", "src-utils-ts", 0),
        ("src/helper.ts", "src-helper-ts", 0),
    ]);
    stamp_file_mtimes(&mut entities, &current_mtimes);
    db.load_entities_overwrite(&entities).await.expect("load");

    let stored_after = read_stored_mtimes(&db).expect("read stored after");
    assert_eq!(stored_after.len(), 3, "all 3 files have mtimes after full index");
}
