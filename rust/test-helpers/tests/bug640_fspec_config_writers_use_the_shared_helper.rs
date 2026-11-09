//! Feature: spec/features/fspec-config-writes-are-atomic-and-tests-never-touch-the-real-user-config.feature
//!
//! BUG-640: source scan. Outside codelet_common::fspec_config (the shared
//! locked read-modify-write helper) no non-test code writes an
//! fspec-config.json directly. A direct write is a call of `fs::write`,
//! `File::create`, `write_bytes_atomic`, `write_json_atomic` or
//! `replace_file_atomically` in a function that names the file (a string
//! literal ending in `fspec-config.json`), or whose path argument is a
//! `config_path` in a source file that names the file.

use std::path::{Path, PathBuf};

use codelet_test_helpers::{OrFail, TestResult};
use quote::ToTokens;
use syn::visit::Visit;

const CONFIG_FILE: &str = "fspec-config.json";
const HELPER: &str = "common/src/fspec_config.rs";
const WRITERS: [&str; 4] = [
    "write_bytes_atomic",
    "write_json_atomic",
    "replace_file_atomically",
    "write_json_atomic_trailing_newline",
];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Every non-test `.rs` file under a workspace crate's `src/`.
fn sources(directory: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let path = entry?.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if path.is_dir() {
            if name != "tests" {
                sources(&path, out)?;
            }
        } else if name.ends_with(".rs") && !name.contains("tests") && name != "tests.rs" {
            out.push(path);
        }
    }
    Ok(())
}

fn is_test_only(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        let text = attr.to_token_stream().to_string();
        attr.path().is_ident("test") || (attr.path().is_ident("cfg") && text.contains("test"))
    })
}

fn names_config_file(tokens: &str) -> bool {
    tokens.contains(&format!("{CONFIG_FILE}\""))
}

/// The direct write calls of one function body: (line, first argument).
#[derive(Default)]
struct Writes {
    calls: Vec<(usize, String)>,
}

impl<'ast> Visit<'ast> for Writes {
    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if let syn::Expr::Path(path) = call.func.as_ref() {
            let segments: Vec<String> = path
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect();
            let last = segments.last().map(String::as_str).unwrap_or_default();
            let owner = segments
                .len()
                .checked_sub(2)
                .and_then(|index| segments.get(index))
                .map(String::as_str);
            let direct = matches!(
                (owner, last),
                (Some("fs"), "write") | (Some("File"), "create")
            ) || WRITERS.contains(&last);
            if direct {
                let argument = call
                    .args
                    .first()
                    .map(|argument| argument.to_token_stream().to_string())
                    .unwrap_or_default();
                let line = path
                    .path
                    .segments
                    .first()
                    .map(|segment| segment.ident.span().start().line)
                    .unwrap_or_default();
                self.calls.push((line, argument));
            }
        }
        syn::visit::visit_expr_call(self, call);
    }
}

/// Every function of a file, test-only items skipped.
struct Functions<'file> {
    file: &'file str,
    file_names_config: bool,
    found: Vec<String>,
}

impl Functions<'_> {
    fn check(&mut self, body: &syn::Block) {
        let names_config = names_config_file(&body.to_token_stream().to_string());
        let mut writes = Writes::default();
        writes.visit_block(body);
        for (line, argument) in writes.calls {
            if names_config || (self.file_names_config && argument.contains("config_path")) {
                self.found.push(format!("{}:{line}", self.file));
            }
        }
    }
}

impl<'ast> Visit<'ast> for Functions<'_> {
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if !is_test_only(&item.attrs) {
            syn::visit::visit_item_mod(self, item);
        }
    }
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if !is_test_only(&item.attrs) {
            self.check(&item.block);
        }
    }
    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        if !is_test_only(&item.attrs) {
            self.check(&item.block);
        }
    }
}

#[test]
fn every_fspec_config_json_writer_goes_through_the_shared_atomic_locked_helper() -> TestResult {
    // @step Given the workspace's non-test Rust sources
    let root = workspace_root();
    let mut files = Vec::new();
    for member in std::fs::read_dir(&root).or_fail("the workspace directory")? {
        let member = member.or_fail("a workspace entry")?.path();
        let name = member
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        // test-helpers is test-only code; patches are vendored crates.
        if ["target", "patches", "test-helpers"].contains(&name) {
            continue;
        }
        let src = member.join("src");
        if src.is_dir() {
            sources(&src, &mut files).or_fail("the crate sources")?;
        }
    }
    assert!(files.len() > 100, "found only {} source files", files.len());

    // @step When every write of a path ending in fspec-config.json is listed
    let mut found = Vec::new();
    for path in &files {
        let relative = path
            .strip_prefix(&root)
            .or_fail("a workspace path")?
            .to_string_lossy()
            .replace('\\', "/");
        if relative == HELPER {
            continue;
        }
        let source = std::fs::read_to_string(path).or_fail("a source file")?;
        let file = syn::parse_file(&source).or_fail("a parsable source file")?;
        let mut functions = Functions {
            file: &relative,
            file_names_config: source.contains(&format!("\"{CONFIG_FILE}\"")),
            found: Vec::new(),
        };
        functions.visit_file(&file);
        found.extend(functions.found);
    }

    // @step Then each one is a call of the shared fspec-config.json update helper
    // @step And no other code writes fspec-config.json with std::fs::write or File::create
    assert!(
        found.is_empty(),
        "fspec-config.json written outside codelet_common::fspec_config::update_config_file:\n{}",
        found.join("\n")
    );
    Ok(())
}
