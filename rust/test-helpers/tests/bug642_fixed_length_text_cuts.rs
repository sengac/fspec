//! BUG-642: no workspace source cuts text at a fixed byte index.
//!
//! Feature: spec/features/text-cuts-land-on-char-boundaries.feature

use codelet_test_helpers::fixed_length_cuts::{fixed_length_cuts, FixedLengthCut};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The exact list of fixed-bound slices that are safe: ASCII-only text
/// (UUID, git hash and hex digest prefixes) or non-text data (byte arrays,
/// line vectors, button arrays). Keyed by file (relative to rust/) and the
/// slice's exact source text.
const SAFE_CUTS: &[(&str, &str)] = &[
    // UUID and git-hash prefixes (ASCII hex).
    (
        "agent-loop/src/agent_manager_handler.rs",
        "subordinate_id.to_string()[..8]",
    ),
    ("cli/src/session/context_gathering.rs", "commit[..8]"),
    (
        "sessions/src/handle_impl.rs",
        "Uuid::new_v4().simple().to_string()[..8]",
    ),
    ("sessions/src/handle_creation.rs", "id.to_string()[..8]"),
    (
        "sessions/src/session_manager/manager_impl_1.rs",
        "id.to_string()[..8]",
    ),
    // Hex digest prefixes.
    ("fspec-core/src/types/guided_session.rs", "hex[..12]"),
    ("graph/src/database.rs", "compiled_hash[..12]"),
    ("graph/src/database.rs", "on_disk_hash[..12]"),
    ("graph/src/bundle.rs", "bundle_hash[..12]"),
    ("graph/src/bundle.rs", "current_hash[..12]"),
    ("providers/src/codex/auth_storage.rs", "hash[..16]"),
    // Bytes, line and row vectors, button arrays.
    ("common/src/image_dimensions.rs", "bytes[..8]"),
    ("core/src/compaction/trimmer.rs", "lines[..BASH_HEAD_LINES]"),
    (
        "core/src/compaction/trimmer.rs",
        "lines[lines.len() - BASH_TAIL_LINES..]",
    ),
    (
        "core/src/compaction/trimmer.rs",
        "lines[..GREP_MATCH_THRESHOLD]",
    ),
    ("fspec/src/create_feature.rs", "parts[parts.len() - 2..]"),
    (
        "fspec-tui/src/store/agent_view/patch_diff.rs",
        "rows[..DIFF_COLLAPSED_LINES]",
    ),
    (
        "fspec-tui/src/views/multiplex/menu_bar_rows.rs",
        "MUX_ZONE_C[..START_BUTTONS]",
    ),
    ("git/src/diff.rs", "lines[..MAX_LINES]"),
    (
        "graph/src/ast_pipeline/metadata.rs",
        "line_capped.as_bytes()[..=MAX_SOURCE_BYTES]",
    ),
    // A unit test's ASCII fixture ("x" repeated).
    (
        "tools/src/session_search/tests.rs",
        "long_content[..MESSAGE_TRUNCATION_LIMIT]",
    ),
];

#[test]
fn a_new_fixed_length_text_cut_fails_the_workspace_guard() -> TestResult {
    // @step Given a source file holding "let p = &title[..40];"
    let source = "fn preview(title: &str) -> &str {\n    let p = &title[..40];\n    p\n}\n";
    // @step When the guard scans it
    let found = fixed_length_cuts(source)?;
    // @step Then it reports the slice "&title[..40]" with its line
    assert_eq!(
        found,
        vec![FixedLengthCut {
            line: 2,
            text: "title[..40]".to_string()
        }]
    );

    // @step And every literal- or constant-bounded slice in the workspace sources is on the guard's exact list
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut found = Vec::new();
    let mut crates: Vec<_> = std::fs::read_dir(&root)?.collect::<Result<_, _>>()?;
    crates.sort_by_key(std::fs::DirEntry::path);
    for crate_dir in crates {
        let name = crate_dir.file_name().to_string_lossy().into_owned();
        let src = crate_dir.path().join("src");
        if name == "patches" || name == "target" || !src.is_dir() {
            continue;
        }
        let mut stack = vec![src];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir)? {
                let path = entry?.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|ext| ext == "rs") {
                    let source = std::fs::read_to_string(&path)?;
                    let relative = path.strip_prefix(&root)?.to_string_lossy().into_owned();
                    for cut in fixed_length_cuts(&source)
                        .map_err(|error| format!("{relative}: {error}"))?
                    {
                        found.push((relative.clone(), cut));
                    }
                }
            }
        }
    }
    let unlisted: Vec<String> = found
        .iter()
        .filter(|(file, cut)| !SAFE_CUTS.contains(&(file.as_str(), cut.text.as_str())))
        .map(|(file, cut)| format!("{file}:{}: {}", cut.line, cut.text))
        .collect();
    let stale: Vec<String> = SAFE_CUTS
        .iter()
        .filter(|(file, text)| !found.iter().any(|(f, cut)| f == file && cut.text == *text))
        .map(|(file, text)| format!("{file}: {text}"))
        .collect();
    assert!(
        unlisted.is_empty() && stale.is_empty(),
        "fixed-length cuts not on SAFE_CUTS (cut on a char boundary instead):\n{}\nSAFE_CUTS entries no longer found:\n{}",
        unlisted.join("\n"),
        stale.join("\n")
    );
    Ok(())
}
