//! KGRAPH-076: content-hash incremental indexing that equals a full index.
//!
//! Feature: spec/features/content-hash-incremental-indexing-equals-a-full-index.feature
//!
//! Every scenario indexes a temp project with `ast_index`. "Equals a full
//! index" compares the sorted JSONL export of the incrementally indexed
//! graph with that of a full index of a copy of the tree, every property
//! included. The edits are fixed (TEST-042): no randomness, no timing.

use std::path::{Path, PathBuf};

use codelet_graph::ast_index::dispatch_ast_index;
use codelet_graph::graph_entities::{entities_to_jsonl, GraphEntity};
use codelet_graph::registry;
use codelet_test_helpers::{setup_temp_directory, TestError, TestResult};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub mod code_graph_fixture;
use code_graph_fixture::{callees_of, entities, nodes, project, slug_of, CRATE_MANIFEST};

/// Run `ast_index` on `root` and return its result, failing on an error.
async fn run(root: &Path, reset: bool, incremental: bool) -> Result<Value, TestError> {
    let output: Value = serde_json::from_str(&dispatch_ast_index(root, reset, incremental).await)?;
    if let Some(error) = output.get("error") {
        return Err(format!("ast_index failed: {error}").into());
    }
    Ok(output)
}

async fn full(root: &Path) -> Result<Value, TestError> {
    run(root, false, false).await
}

async fn incremental(root: &Path) -> Result<Value, TestError> {
    run(root, false, true).await
}

fn count(result: &Value, field: &str) -> Result<u64, TestError> {
    result[field]
        .as_u64()
        .ok_or_else(|| format!("no {field} in {result}").into())
}

fn written(result: &Value) -> Result<bool, TestError> {
    result["written"]
        .as_bool()
        .ok_or_else(|| format!("no written in {result}").into())
}

/// The graph of `root` as sorted JSONL lines.
async fn sorted_export(root: &Path) -> Result<Vec<String>, TestError> {
    let mut lines: Vec<String> = entities_to_jsonl(&entities(root).await?)
        .lines()
        .map(str::to_owned)
        .collect();
    lines.sort();
    Ok(lines)
}

/// Copy the files of `from` (not its `.fspec` graph) to `to`.
fn copy_sources(from: &Path, to: &Path) -> TestResult {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        if entry.file_name() == ".fspec" {
            continue;
        }
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_sources(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// The sorted export of a full index of a fresh copy of `root`'s tree.
async fn full_index_of_a_copy(root: &Path) -> Result<Vec<String>, TestError> {
    let directory = setup_temp_directory()?;
    let copy = directory.path().join("project");
    copy_sources(root, &copy)?;
    let copy: PathBuf = copy.canonicalize()?;
    full(&copy).await?;
    sorted_export(&copy).await
}

async fn assert_equals_a_full_index(root: &Path, context: &str) -> TestResult {
    let incremental = sorted_export(root).await?;
    let expected = full_index_of_a_copy(root).await?;
    if incremental != expected {
        let missing: Vec<&String> = expected
            .iter()
            .filter(|line| !incremental.contains(line))
            .collect();
        let extra: Vec<&String> = incremental
            .iter()
            .filter(|line| !expected.contains(line))
            .collect();
        return Err(format!(
            "{context}: the incremental graph differs from a full index\nmissing: {missing:#?}\nextra: {extra:#?}"
        )
        .into());
    }
    Ok(())
}

fn write(root: &Path, path: &str, text: &str) -> TestResult {
    let full = root.join(path);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(full, text)?;
    Ok(())
}

/// The qualifiedName of every Function, sorted.
fn function_names(all: &[GraphEntity]) -> Vec<String> {
    let mut names: Vec<String> = nodes(all, "Function")
        .into_iter()
        .filter_map(|(_, p)| Some(p.get("qualifiedName")?.as_str()?.to_string()))
        .collect();
    names.sort();
    names
}

const THREE_FILES: &[(&str, &str)] = &[
    ("src/main.py", "def handle():\n    return 1\n"),
    ("src/utils.py", "def parse():\n    return 2\n"),
    ("src/helper.py", "def format_output():\n    return 3\n"),
];

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_definition_added_to_the_target_of_an_unchanged_files_import_is_called_after_an_incremental_index(
) -> TestResult {
    // @step Given a Python package where a.py imports helper from b.py and calls it, and b.py has no helper
    let (_directory, root) = project(&[
        ("pkg/__init__.py", ""),
        (
            "pkg/a.py",
            "from .b import helper\n\n\ndef caller():\n    return helper()\n",
        ),
        ("pkg/b.py", "def other():\n    return 1\n"),
    ])?;

    // @step And the package has been indexed
    full(&root).await?;

    // @step When def helper is added to b.py and an incremental ast_index runs
    write(
        &root,
        "pkg/b.py",
        "def other():\n    return 1\n\n\ndef helper():\n    return 2\n",
    )?;
    incremental(&root).await?;

    // @step Then a Calls edge leads from a.py's caller to b.py's helper
    let all = entities(&root).await?;
    let caller = slug_of(&all, "Function", "pkg/a.py::caller")?;
    assert_eq!(callees_of(&all, &caller), ["pkg/b.py::helper"]);

    // @step And the graph equals a full index of a copy of the tree
    assert_equals_a_full_index(&root, "after adding helper").await
}

/// The fixed edit script: (description, files to write, files to delete).
type Edit = (
    &'static str,
    &'static [(&'static str, &'static str)],
    &'static [&'static str],
);

const EDITS: &[Edit] = &[
    (
        "add fn extra to src/util.rs and call it from run",
        &[
            (
                "src/util.rs",
                "pub fn helper() -> u32 {\n    1\n}\n\npub fn extra() -> u32 {\n    2\n}\n",
            ),
            (
                "src/lib.rs",
                "mod util;\n\npub fn run() -> u32 {\n    util::helper() + util::extra()\n}\n",
            ),
        ],
        &[],
    ),
    (
        "rename helper to assist, leaving its caller unresolved",
        &[(
            "src/util.rs",
            "pub fn assist() -> u32 {\n    1\n}\n\npub fn extra() -> u32 {\n    2\n}\n",
        )],
        &[],
    ),
    (
        "call util::assist from run",
        &[(
            "src/lib.rs",
            "mod util;\n\npub fn run() -> u32 {\n    util::assist() + util::extra()\n}\n",
        )],
        &[],
    ),
    (
        "move assist into a new module src/tools.rs",
        &[
            ("src/tools.rs", "pub fn assist() -> u32 {\n    1\n}\n"),
            ("src/util.rs", "pub fn extra() -> u32 {\n    2\n}\n"),
            (
                "src/lib.rs",
                "mod tools;\nmod util;\n\npub fn run() -> u32 {\n    tools::assist() + util::extra()\n}\n",
            ),
        ],
        &[],
    ),
    (
        "delete src/util.rs and its module declaration",
        &[(
            "src/lib.rs",
            "mod tools;\n\npub fn run() -> u32 {\n    tools::assist()\n}\n",
        )],
        &["src/util.rs"],
    ),
    (
        "add def helper to py/lib.py and import it in py/app.py",
        &[
            (
                "py/lib.py",
                "def parse():\n    return 1\n\n\ndef helper():\n    return 2\n",
            ),
            (
                "py/app.py",
                "from .lib import parse, helper\n\n\ndef main():\n    return parse() + helper()\n",
            ),
        ],
        &[],
    ),
    ("delete py/lib.py", &[], &["py/lib.py"]),
    (
        "add py/lib.py back with parse only",
        &[("py/lib.py", "def parse():\n    return 1\n")],
        &[],
    ),
    (
        "add ts/utils/extra.ts, export it from the barrel and use it",
        &[
            (
                "ts/utils/extra.ts",
                "export function extra(): string {\n  return 'e';\n}\n",
            ),
            (
                "ts/utils/index.ts",
                "export { format } from './format';\nexport { extra } from './extra';\n",
            ),
            (
                "ts/main.ts",
                "import { format, extra } from './utils';\n\nexport function show(): string {\n  return format('x') + extra();\n}\n",
            ),
        ],
        &[],
    ),
    ("delete the barrel ts/utils/index.ts", &[], &["ts/utils/index.ts"]),
    (
        "rename the crate and add a package.json dependency",
        &[
            ("Cargo.toml", "[package]\nname = \"renamed\"\n"),
            (
                "package.json",
                "{\n  \"name\": \"fixture\",\n  \"dependencies\": { \"left-pad\": \"1.3.0\" }\n}\n",
            ),
        ],
        &[],
    ),
    (
        "ignore ts/ in .gitignore",
        &[(".gitignore", "ts/\n")],
        &[],
    ),
];

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn after_every_edit_of_a_fixed_sequence_the_incremental_graph_equals_a_full_index(
) -> TestResult {
    // @step Given a project holding a Rust crate, a Python package and a TypeScript barrel has been indexed
    let (_directory, root) = project(&[
        CRATE_MANIFEST,
        (
            "src/lib.rs",
            "mod util;\n\npub fn run() -> u32 {\n    util::helper()\n}\n",
        ),
        ("src/util.rs", "pub fn helper() -> u32 {\n    1\n}\n"),
        ("py/__init__.py", ""),
        (
            "py/app.py",
            "from .lib import parse\n\n\ndef main():\n    return parse()\n",
        ),
        ("py/lib.py", "def parse():\n    return 1\n"),
        ("ts/utils/index.ts", "export { format } from './format';\n"),
        (
            "ts/utils/format.ts",
            "export function format(s: string): string {\n  return s;\n}\n",
        ),
        (
            "ts/main.ts",
            "import { format } from './utils';\n\nexport function show(): string {\n  return format('x');\n}\n",
        ),
    ])?;
    full(&root).await?;
    assert_equals_a_full_index(&root, "the first index").await?;

    // @step When each of 12 fixed edits is applied in turn, with an incremental ast_index after each
    assert_eq!(EDITS.len(), 12);
    for (description, writes, deletes) in EDITS {
        for (path, text) in *writes {
            write(&root, path, text)?;
        }
        for path in *deletes {
            std::fs::remove_file(root.join(path))?;
        }
        let result = incremental(&root).await?;
        assert!(written(&result)?, "{description}: {result}");

        // @step Then after every edit the graph's sorted export equals that of a full index of a copy of the tree
        assert_equals_a_full_index(&root, description).await?;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_incremental_index_with_nothing_changed_parses_nothing_and_writes_nothing() -> TestResult
{
    // @step Given a project of 3 source files has been fully indexed
    let (_directory, root) = project(THREE_FILES)?;
    full(&root).await?;
    let before = sorted_export(&root).await?;

    // @step When an incremental ast_index runs in the same process with nothing changed
    let result = incremental(&root).await?;

    // @step Then it parses 0 files and reuses 3
    assert_eq!(count(&result, "files_parsed")?, 0, "{result}");
    assert_eq!(count(&result, "files_reused")?, 3, "{result}");
    assert_eq!(result["files_failed"], serde_json::json!([]), "{result}");
    assert_eq!(result["files_skipped"], serde_json::json!([]), "{result}");

    // @step And it reports that the graph was not written
    assert!(!written(&result)?, "{result}");
    assert_eq!(sorted_export(&root).await?, before);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_incremental_index_parses_only_the_edited_file() -> TestResult {
    // @step Given a project of 3 source files has been fully indexed
    let (_directory, root) = project(THREE_FILES)?;
    full(&root).await?;

    // @step When one file is edited and an incremental ast_index runs
    write(&root, "src/utils.py", "def parse_all():\n    return 2\n")?;
    let result = incremental(&root).await?;

    // @step Then it parses 1 file and reuses 2
    assert_eq!(count(&result, "files_parsed")?, 1, "{result}");
    assert_eq!(count(&result, "files_reused")?, 2, "{result}");

    // @step And the graph holds the edited file's new definition
    assert_eq!(
        function_names(&entities(&root).await?),
        [
            "src/helper.py::format_output",
            "src/main.py::handle",
            "src/utils.py::parse_all"
        ]
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_content_change_that_keeps_the_modification_time_is_indexed() -> TestResult {
    // @step Given a project of 3 source files has been fully indexed
    let (_directory, root) = project(THREE_FILES)?;
    full(&root).await?;

    // @step When a file's text changes, its modification time is set back to the old one, and an incremental ast_index runs
    let path = root.join("src/helper.py");
    let old_time = std::fs::metadata(&path)?.modified()?;
    let new_text = "def format_text():\n    return 3\n";
    std::fs::write(&path, new_text)?;
    std::fs::File::options()
        .write(true)
        .open(&path)?
        .set_modified(old_time)?;
    assert_eq!(std::fs::metadata(&path)?.modified()?, old_time);
    let result = incremental(&root).await?;

    // @step Then it parses that file
    assert_eq!(count(&result, "files_parsed")?, 1, "{result}");

    // @step And the graph holds the new definition, and the File node's contentHash is the SHA-256 of the new text
    let all = entities(&root).await?;
    assert!(function_names(&all).contains(&"src/helper.py::format_text".to_string()));
    let file = nodes(&all, "File")
        .into_iter()
        .find(|(slug, _)| slug == "src/helper.py")
        .ok_or("no File src/helper.py")?;
    let expected = format!("{:x}", Sha256::digest(new_text.as_bytes()));
    assert_eq!(
        file.1.get("contentHash").and_then(Value::as_str),
        Some(expected.as_str())
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_manifest_change_alone_makes_an_incremental_index_write_the_graph() -> TestResult {
    // @step Given a project with a package.json and source files has been indexed
    let (_directory, root) = project(&[
        (
            "package.json",
            "{\n  \"name\": \"fixture\",\n  \"dependencies\": { \"left-pad\": \"1.3.0\" }\n}\n",
        ),
        (
            "src/main.ts",
            "export function show(): string {\n  return 'x';\n}\n",
        ),
    ])?;
    full(&root).await?;

    // @step When a dependency is added to package.json and an incremental ast_index runs
    write(
        &root,
        "package.json",
        "{\n  \"name\": \"fixture\",\n  \"dependencies\": { \"left-pad\": \"1.3.0\", \"is-odd\": \"3.0.1\" }\n}\n",
    )?;
    let result = incremental(&root).await?;

    // @step Then the graph is written
    assert!(written(&result)?, "{result}");

    // @step And it holds the new Dependency
    let mut dependencies: Vec<String> = nodes(&entities(&root).await?, "Dependency")
        .into_iter()
        .filter_map(|(_, p)| Some(p.get("name")?.as_str()?.to_string()))
        .collect();
    dependencies.sort();
    assert_eq!(dependencies, ["is-odd", "left-pad"]);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn another_write_to_the_graph_makes_the_next_incremental_index_write() -> TestResult {
    // @step Given a project has been indexed
    let (_directory, root) = project(THREE_FILES)?;
    full(&root).await?;

    // @step And a bundle exported from another project's graph has been imported into its graph
    let (other_directory, other) = project(&[("lib.py", "def elsewhere():\n    return 0\n")])?;
    full(&other).await?;
    let bundle = other_directory.path().join("other.astbundle");
    registry::read_code_graph(&other)
        .await?
        .export_bundle(&bundle, registry::AST_CODE_SCHEMA)
        .await?;
    let mut lease = registry::write_code_graph(&root).await?;
    lease
        .database()
        .await?
        .import_bundle(&bundle, registry::AST_CODE_SCHEMA, "overwrite")
        .await?;
    drop(lease);

    // @step When an incremental ast_index runs with no file changed
    let result = incremental(&root).await?;

    // @step Then the graph is written
    assert!(written(&result)?, "{result}");

    // @step And the graph equals a full index of a copy of the tree
    assert_equals_a_full_index(&root, "after the bundle import").await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_full_index_parses_every_file() -> TestResult {
    // @step Given a project of 3 source files has been indexed
    let (_directory, root) = project(THREE_FILES)?;
    incremental(&root).await?;

    // @step When a full ast_index runs with nothing changed
    let result = full(&root).await?;

    // @step Then it parses 3 files and reuses 0
    assert_eq!(count(&result, "files_parsed")?, 3, "{result}");
    assert_eq!(count(&result, "files_reused")?, 0, "{result}");

    // @step And the graph is written
    assert!(written(&result)?, "{result}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_reset_empties_the_roots_fact_cache() -> TestResult {
    // @step Given a project of 3 source files has been indexed
    let (_directory, root) = project(THREE_FILES)?;
    full(&root).await?;

    // @step When an incremental ast_index runs with reset
    let result = run(&root, true, true).await?;

    // @step Then it parses 3 files and reuses 0
    assert_eq!(count(&result, "files_parsed")?, 3, "{result}");
    assert_eq!(count(&result, "files_reused")?, 0, "{result}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn files_that_are_not_parsed_are_reported() -> TestResult {
    // @step Given a project holding a source file that is not UTF-8, a 2 MiB source file and one valid source file
    let (_directory, root) = project(&[("src/ok.py", "def fine():\n    return 1\n")])?;
    std::fs::write(root.join("src/latin1.py"), b"x = '\xe9'\n")?;
    std::fs::write(root.join("src/huge.py"), "#".repeat(2 * 1024 * 1024))?;

    // @step When ast_index runs for the project
    let result = full(&root).await?;

    // @step Then files_failed lists the file that is not UTF-8 with its error
    let failed = result["files_failed"]
        .as_array()
        .ok_or_else(|| format!("no files_failed in {result}"))?;
    assert_eq!(failed.len(), 1, "{result}");
    assert_eq!(failed[0]["path"], "src/latin1.py");
    assert!(
        failed[0]["error"]
            .as_str()
            .is_some_and(|error| error.contains("UTF-8")),
        "{result}"
    );

    // @step And files_skipped lists the 2 MiB file with reason too_large
    assert_eq!(
        result["files_skipped"],
        serde_json::json!([{ "path": "src/huge.py", "reason": "too_large" }])
    );

    // @step And it parses 1 file and the result names the project's canonical root
    assert_eq!(count(&result, "files_parsed")?, 1, "{result}");
    assert_eq!(result["root"], Value::String(root.display().to_string()));
    Ok(())
}
