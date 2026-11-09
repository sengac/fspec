// Feature: spec/features/ast-index-class-import-crash.feature
use super::*;

#[test]
fn test_deduplicate_prunes_calls_edge_to_type_only_slug() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given extracted entities containing a Type node with slug "file-a::MyClass"
    write_test_file(
        project_dir,
        "file_a.py",
        "class MyClass:\n    def __init__(self):\n        self.value = 1\n",
    );

    // @step And no Function node exists with slug "file-a::MyClass"
    // (file_a.py defines no function named MyClass)

    // @step And a Calls edge from "file-b::caller" to "file-a::MyClass"
    // file_b.py calls MyClass() as a constructor.
    write_test_file(
        project_dir,
        "file_b.py",
        "from file_a import MyClass\n\ndef caller():\n    return MyClass()\n",
    );

    // @step When I run deduplicate_entities on the entity list
    // The resolve pass builds the graph; it is the only producer of edges.
    let indexed = index(project_dir);
    let entities = indexed.entities;
    let types: Vec<_> = find_nodes_by_name(&entities, "Type", "MyClass");
    assert_eq!(types.len(), 1, "one Type node MyClass: {types:?}");
    assert!(
        find_nodes_by_name(&entities, "Function", "MyClass").is_empty(),
        "no Function node is named MyClass"
    );
    assert!(
        named_edges(&entities, "Imports").contains(&("file_b.py".into(), "file_a.py".into())),
        "the import resolves: {:?}",
        named_edges(&entities, "Imports")
    );

    // @step Then the Calls edge from "file-b::caller" to "file-a::MyClass" should be pruned
    let remaining_calls: Vec<_> = named_edges(&entities, "Calls")
        .into_iter()
        .filter(|(from, _)| from == "file_b.py::caller")
        .collect();
    assert!(
        remaining_calls.is_empty(),
        "A constructor call is no Calls edge. Remaining Calls: {remaining_calls:?}"
    );

    // @step And no Calls edge should remain targeting "file-a::MyClass"
    assert!(
        edges_touching(&entities, "Calls", "MyClass").is_empty(),
        "No Calls edge may target the Type MyClass: {:?}",
        named_edges(&entities, "Calls")
    );
}

#[test]
fn test_python_stdlib_import_no_edges() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    // @step Given a Python file "src/app/core.py" with "from os import path"
    let source = r#"
from os import path

def resolve(a, b):
    return path.join(a, b)
"#;
    write_test_file(temp_dir.path(), "src/app/core.py", source);

    // @step And "src/app/core.py" has function "resolve" that calls "path.join(a, b)"
    // @step When I extract entities with known_files NOT containing any "os" path
    let indexed = index(temp_dir.path());

    // @step Then no Imports edge should exist for the "os" import
    let import_edges = named_edges(&indexed.entities, "Imports");
    assert!(
        import_edges.is_empty(),
        "stdlib imports (os) should NOT produce Imports edges. Found: {import_edges:?}"
    );
    assert_eq!(
        unresolved(&indexed, "imports", "external"),
        1,
        "the os import is counted as external: {:?}",
        indexed.unresolved
    );

    // @step And indexing should complete without errors
    assert!(indexed.failed.is_empty(), "{:?}", indexed.failed);
    assert_eq!(indexed.files_parsed, 1);
}
