// Feature: spec/features/incremental-reindexing.feature
use super::*;

// ============================================================================
// Scenario: Incremental re-index adds new file entities
// ============================================================================
#[tokio::test]
async fn test_incremental_adds_new_file_entities() {
    let (_temp_dir, root) = temp_project();

    // @step Given a project that has been fully indexed
    run_index(&root, false).await;
    let before = graph_entities(&root).await;

    // @step And a new source file has been added to the project
    std::fs::write(
        root.join("src/extra.ts"),
        "export function extraFunc(): void {\n  console.log('extra');\n}\n",
    )
    .expect("write extra.ts");

    // @step When I run an incremental ast_index on the project
    let result = run_index(&root, true).await;
    assert_eq!(result["files_parsed"], 1, "{result}");
    assert_eq!(result["files_reused"], 3, "{result}");

    // @step Then the new file's entities are extracted and added to the graph
    let after = graph_entities(&root).await;
    let names = node_names(&after);
    assert!(names.contains(&("File".into(), "src/extra.ts".into())));
    assert!(names.contains(&("Function".into(), "src/extra.ts::extraFunc".into())));
    let file_count = names.iter().filter(|(t, _)| t == "File").count();
    assert_eq!(file_count, 4, "4 files total (3 original + 1 new)");

    // @step And entities from previously indexed files are preserved
    let fn_count = names.iter().filter(|(t, _)| t == "Function").count();
    assert_eq!(fn_count, 4, "4 functions total (3 original + 1 new)");
    for path in ["src/main.ts", "src/utils.ts", "src/helper.ts"] {
        assert_eq!(
            entities_of(&after, path),
            entities_of(&before, path),
            "entities of {path} are preserved"
        );
    }
}

// ============================================================================
// Scenario: An incremental index of a project with no stored graph parses and writes every file
// ============================================================================
#[tokio::test]
async fn test_incremental_index_without_stored_graph_parses_and_writes_every_file() {
    let (_temp_dir, root) = temp_project();
    let files = codelet_graph::ast_pipeline::walk_source_files(&root, true);

    // @step Given a project directory with source files
    assert_eq!(files.len(), 3);

    // @step And the AST graph is empty with no prior index
    assert!(
        !registry::code_graph_path(&root).exists(),
        "no code graph before the first index"
    );

    // @step When I run an incremental ast_index on the project
    let result = run_index(&root, true).await;

    // @step Then every source file is parsed
    assert_eq!(result["incremental"], true, "{result}");
    assert_eq!(result["files_parsed"], 3, "{result}");
    assert_eq!(result["files_reused"], 0, "{result}");

    // @step And all entities are loaded into the graph
    assert_eq!(result["written"], true, "{result}");
    let names = node_names(&graph_entities(&root).await);
    assert_eq!(
        names.iter().filter(|(t, _)| t == "File").count(),
        3,
        "{names:?}"
    );
    assert_eq!(
        names.iter().filter(|(t, _)| t == "Function").count(),
        3,
        "{names:?}"
    );
}
