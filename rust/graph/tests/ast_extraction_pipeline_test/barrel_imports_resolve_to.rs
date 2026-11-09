// Feature: spec/features/ast-extraction-pipeline.feature
// Feature: spec/features/ast-entity-deduplication.feature
use super::*;

// ============================================================================
// Scenario: Barrel imports resolve to directory/index.ts during project walk
// ============================================================================
#[tokio::test]
async fn test_barrel_imports_resolve_to_index_ts() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a project with a barrel module "src/types/index.ts"
    let types_content = r#"
export interface WorkUnit {
    id: string;
    title: string;
}

export interface PrefixesData {
    prefixes: string[];
}
"#;
    write_test_file(project_dir, "src/types/index.ts", types_content);

    // @step And a command file "src/commands/list.ts" that imports from "../types"
    let list_content = r#"
import type { WorkUnit } from '../types';

export function listWorkUnits(): void {
    return;
}
"#;
    write_test_file(project_dir, "src/commands/list.ts", list_content);

    // @step And another command file "src/commands/show.ts" that also imports from "../types"
    let show_content = r#"
import type { WorkUnit, PrefixesData } from '../types';

export function showWorkUnit(): void {
    return;
}
"#;
    write_test_file(project_dir, "src/commands/show.ts", show_content);
    write_test_file(project_dir, ".gitignore", "node_modules/\ntarget/\n");

    // @step When the extraction pipeline processes the project directory
    let entities = walk_and_extract(project_dir, true);

    // @step Then "src/types/index.ts" should be resolved correctly (not "src/types.ts")
    let types_index_nodes: Vec<_> = entities
        .iter()
        .filter(|e| {
            matches!(e, GraphEntity::Node { node_type, properties, .. }
            if node_type == "File"
                && properties.get("path").and_then(|v| v.as_str()) == Some("src/types/index.ts"))
        })
        .collect();
    assert_eq!(
        types_index_nodes.len(),
        1,
        "Should have exactly 1 File node for src/types/index.ts"
    );

    // @step And NO phantom File node should exist for "src/types.ts"
    let phantom_nodes: Vec<_> = entities
        .iter()
        .filter(|e| {
            matches!(e, GraphEntity::Node { node_type, properties, .. }
            if node_type == "File"
                && properties.get("path").and_then(|v| v.as_str()) == Some("src/types.ts"))
        })
        .collect();
    assert_eq!(
        phantom_nodes.len(),
        0,
        "Should NOT create a phantom File node for src/types.ts — barrel import should resolve to index.ts"
    );

    // @step And Import edges should point to the correct "src-types-index-ts" slug
    let import_edges: Vec<_> = entities
        .iter()
        .filter_map(|e| {
            if let GraphEntity::Edge {
                edge_type,
                to_slug,
                from_slug,
                ..
            } = e
            {
                if edge_type == "Imports" && to_slug.contains("types") {
                    return Some((from_slug.clone(), to_slug.clone()));
                }
            }
            None
        })
        .collect();

    assert_eq!(
        import_edges.len(),
        2,
        "Should have 2 Imports edges to types"
    );
    for (from, to) in &import_edges {
        assert_eq!(
            to, "src/types/index.ts",
            "Import from {} should target src/types/index.ts, got {}",
            from, to
        );
    }

    // @step And the graph should load successfully
    let db_path = temp_dir.path().join("test-ast-barrel.nano");
    let db = GraphDatabase::init(&db_path, AST_CODE_SCHEMA)
        .await
        .expect("DB init should succeed");

    let load_result = db.load_entities(&entities).await;
    assert!(
        load_result.is_ok(),
        "Graph load should succeed, got: {:?}",
        load_result.err()
    );
}

// ============================================================================
// Scenario: Barrel import via subdirectory (e.g. commands/schedule → schedule/index.ts)
// ============================================================================
#[tokio::test]
async fn test_barrel_import_subdirectory_schedule() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a barrel module "src/commands/schedule/index.ts" re-exporting sub-modules
    let index_content = r#"
export { addSchedule } from './add-schedule';
export { removeSchedule } from './remove-schedule';
"#;
    write_test_file(project_dir, "src/commands/schedule/index.ts", index_content);

    let add_content = r#"
export function addSchedule(): void {}
"#;
    write_test_file(
        project_dir,
        "src/commands/schedule/add-schedule.ts",
        add_content,
    );

    let remove_content = r#"
export function removeSchedule(): void {}
"#;
    write_test_file(
        project_dir,
        "src/commands/schedule/remove-schedule.ts",
        remove_content,
    );

    // @step And a CLI file "src/cli/program.ts" that imports from "../commands/schedule"
    let program_content = r#"
import { addSchedule, removeSchedule } from '../commands/schedule';

export function registerCommands(): void {
    addSchedule();
    removeSchedule();
}
"#;
    write_test_file(project_dir, "src/cli/program.ts", program_content);
    write_test_file(project_dir, ".gitignore", "node_modules/\ntarget/\n");

    // @step When the extraction pipeline processes the project directory
    let entities = walk_and_extract(project_dir, true);

    // @step Then the import should resolve to "src/commands/schedule/index.ts"
    // The specifier resolves to the barrel; each imported name also
    // resolves through the barrel's re-exports to the file defining it.
    let from_program: Vec<(String, String)> = named_edges(&entities, "Imports")
        .into_iter()
        .filter(|(from, _)| from == "src/cli/program.ts")
        .collect();
    assert!(
        has_named_edge(
            &entities,
            "Imports",
            "src/cli/program.ts",
            "src/commands/schedule/index.ts"
        ),
        "Import should resolve to schedule/index.ts barrel, not schedule.ts: {from_program:?}"
    );
    assert_eq!(
        from_program,
        [
            (
                "src/cli/program.ts".to_string(),
                "src/commands/schedule/add-schedule.ts".to_string()
            ),
            (
                "src/cli/program.ts".to_string(),
                "src/commands/schedule/index.ts".to_string()
            ),
            (
                "src/cli/program.ts".to_string(),
                "src/commands/schedule/remove-schedule.ts".to_string()
            ),
        ],
    );

    // @step And NO phantom "src/commands/schedule.ts" File node should exist
    let phantom: Vec<_> = entities
        .iter()
        .filter(|e| matches!(e, GraphEntity::Node { node_type, properties, .. }
            if node_type == "File"
                && properties.get("path").and_then(|v| v.as_str()) == Some("src/commands/schedule.ts")))
        .collect();
    assert!(
        phantom.is_empty(),
        "Should NOT create phantom src/commands/schedule.ts node"
    );
}
