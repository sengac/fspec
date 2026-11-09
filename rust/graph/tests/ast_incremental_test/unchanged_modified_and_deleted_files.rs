// Feature: spec/features/incremental-reindexing.feature
use super::*;

// ============================================================================
// Scenario: Incremental re-index detects no changes and skips extraction
// ============================================================================
#[tokio::test]
async fn test_incremental_no_changes_skips_extraction() {
    let (_temp_dir, root) = temp_project();

    // @step Given a project that has been fully indexed
    run_index(&root, false).await;
    let before = graph_entities(&root).await;

    // @step And no source files have been modified since the last index
    // (nothing is written to the project between the two runs)

    // @step When I run an incremental ast_index on the project
    let result = run_index(&root, true).await;

    // @step Then the result reports 0 files parsed
    assert_eq!(result["incremental"], true, "{result}");
    assert_eq!(result["files_parsed"], 0, "{result}");
    assert_eq!(result["files_reused"], 3, "{result}");
    assert_eq!(result["written"], false, "nothing is written: {result}");
    assert_eq!(result["entities_loaded"], 0, "nothing is written: {result}");

    // @step And all existing entities in the graph are preserved unchanged
    let after = graph_entities(&root).await;
    let debug = |all: &[GraphEntity]| {
        let mut list: Vec<String> = all.iter().map(|e| format!("{e:?}")).collect();
        list.sort();
        list
    };
    assert_eq!(debug(&after), debug(&before), "the graph is unchanged");
    let file_count = node_names(&after)
        .iter()
        .filter(|(t, _)| t == "File")
        .count();
    assert_eq!(file_count, 3, "all 3 file nodes preserved");
}

// ============================================================================
// Scenario: Incremental re-index extracts only modified files
// ============================================================================
#[tokio::test]
async fn test_incremental_extracts_only_modified_files() {
    let (_temp_dir, root) = temp_project();

    // @step Given a project that has been fully indexed
    run_index(&root, false).await;
    let before = graph_entities(&root).await;

    // @step And one source file has been modified since the last index
    let main_ts = root.join("src/main.ts");
    std::fs::write(
        &main_ts,
        "export function handleUpdatedRequest(req: any, res: any) {\n  console.log('updated');\n  return res.json({ok: true});\n}\n",
    )
    .expect("modify main.ts");

    // @step When I run an incremental ast_index on the project
    let result = run_index(&root, true).await;

    // @step Then only the modified file is parsed
    assert_eq!(result["incremental"], true, "{result}");
    assert_eq!(result["files_parsed"], 1, "{result}");

    // @step And the facts of the unchanged files are reused
    assert_eq!(result["files_reused"], 2, "{result}");
    let after = graph_entities(&root).await;
    for path in ["src/utils.ts", "src/helper.ts"] {
        assert_eq!(
            entities_of(&after, path),
            entities_of(&before, path),
            "entities of unchanged {path} are identical after the re-index"
        );
    }

    // @step And the graph contains the updated entities for the modified file
    let names = node_names(&after);
    assert!(
        names.contains(&(
            "Function".into(),
            "src/main.ts::handleUpdatedRequest".into()
        )),
        "the modified file's new function is in the graph: {names:?}"
    );
    assert!(
        !names.contains(&("Function".into(), "src/main.ts::handleRequest".into())),
        "the modified file's old function is gone: {names:?}"
    );
}

// ============================================================================
// Scenario: Incremental re-index removes deleted file entities
// ============================================================================
#[tokio::test]
async fn test_incremental_removes_deleted_file_entities() {
    let (_temp_dir, root) = temp_project();

    // @step Given a project that has been fully indexed
    run_index(&root, false).await;
    assert!(!entities_of(&graph_entities(&root).await, "src/helper.ts").is_empty());

    // @step And one source file has been deleted from the filesystem
    std::fs::remove_file(root.join("src/helper.ts")).expect("delete helper.ts");

    // @step When I run an incremental ast_index on the project
    let result = run_index(&root, true).await;
    assert_eq!(result["files_parsed"], 0, "{result}");
    assert_eq!(result["files_reused"], 2, "{result}");

    // @step Then the deleted file's File node is no longer in the graph
    let after = graph_entities(&root).await;
    let names = node_names(&after);
    let files: Vec<&str> = names
        .iter()
        .filter(|(t, _)| t == "File")
        .map(|(_, n)| n.as_str())
        .collect();
    assert_eq!(
        files,
        ["src/main.ts", "src/utils.ts"],
        "only 2 files remaining"
    );

    // @step And the deleted file's Function and Type nodes are no longer in the graph
    assert!(
        entities_of(&after, "src/helper.ts").is_empty(),
        "no entity of helper.ts remains"
    );
    let functions = names.iter().filter(|(t, _)| t == "Function").count();
    assert_eq!(
        functions, 2,
        "only 2 functions remaining (helper.ts function removed)"
    );
}
