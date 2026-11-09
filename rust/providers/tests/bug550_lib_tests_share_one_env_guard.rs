//! BUG-550 — every providers lib test that changes the process environment
//! runs under the one shared guard, serial_test's default `#[serial]` key.
//!
//! A module-local mutex and a serial_test group do not exclude each other,
//! so tests in different modules overwrote each other's FSPEC_HOME. The
//! guard scans the lib test code: a test that changes the environment
//! directly, through a helper or through a shared env fixture must carry the
//! default `#[serial]` attribute, and no module-local lock or keyed serial
//! group may exist.
//!
//! Feature: spec/features/providers-lib-tests-share-one-environment-guard.feature

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The one FSPEC_HOME redirect the lib tests share.
const SHARED_FSPEC_HOME: &str = "src/test_fixtures/env.rs";

/// A function found in a source file.
struct Function {
    /// `Type::name` inside an impl block, `name` otherwise.
    qualified: String,
    name: String,
    attributes: String,
    body: String,
    takes_self: bool,
    in_drop_impl: bool,
}

impl Function {
    fn is_test(&self) -> bool {
        self.attributes.contains("#[test]") || self.attributes.contains("::test")
    }

    fn has_default_serial(&self) -> bool {
        self.attributes
            .lines()
            .any(|line| matches!(line.trim(), "#[serial]" | "#[serial_test::serial]"))
    }

    fn changes_env_directly(&self) -> bool {
        self.body.contains("env::set_var(") || self.body.contains("env::remove_var(")
    }
}

fn brace_delta(line: &str) -> i64 {
    line.chars().fold(0, |depth, character| match character {
        '{' => depth + 1,
        '}' => depth - 1,
        _ => depth,
    })
}

fn fn_name(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let rest = ["pub(crate) ", "pub(super) ", "pub "]
        .iter()
        .find_map(|prefix| trimmed.strip_prefix(prefix))
        .unwrap_or(trimmed);
    let rest = rest.strip_prefix("async ").unwrap_or(rest);
    let rest = rest.strip_prefix("fn ")?;
    let end = rest.find(|character: char| !(character.is_alphanumeric() || character == '_'))?;
    Some(&rest[..end])
}

fn impl_type(line: &str) -> Option<(String, bool)> {
    let trimmed = line.trim_start();
    let rest = trimmed.strip_prefix("impl")?;
    if !(rest.starts_with(' ') || rest.starts_with('<')) || !trimmed.ends_with('{') {
        return None;
    }
    let header = rest.trim_end_matches('{').trim();
    // `impl<T> Type<T>`: drop the impl's own generic parameters.
    let header = match header.strip_prefix('<') {
        Some(generic) => generic.split_once("> ").map_or(header, |(_, rest)| rest),
        None => header,
    };
    let target = header.rsplit(" for ").next().unwrap_or(header);
    // `path::to::Type<T>` names `Type`.
    let path = target.split('<').next().unwrap_or(target).trim();
    let name = path.rsplit("::").next().unwrap_or(path).to_string();
    Some((name, header.contains("Drop for ")))
}

/// Every function in `source`, with its attributes and body.
fn functions(source: &str) -> Vec<Function> {
    let lines: Vec<&str> = source.lines().collect();
    let mut found = Vec::new();
    let mut current_impl: Option<(String, bool, i64)> = None;
    let mut depth = 0_i64;
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        if let Some((name, drop)) = impl_type(line) {
            current_impl = Some((name, drop, depth));
        }
        if let Some(name) = fn_name(line) {
            let mut attributes = Vec::new();
            let mut back = index;
            while back > 0 {
                let previous = lines[back - 1].trim();
                if previous.starts_with("#[") || previous.starts_with("///") {
                    attributes.push(previous);
                    back -= 1;
                } else {
                    break;
                }
            }
            let mut body = String::new();
            let mut body_depth = 0_i64;
            let mut end = index;
            let mut opened = false;
            while end < lines.len() {
                body.push_str(lines[end]);
                body.push('\n');
                body_depth += brace_delta(lines[end]);
                opened |= lines[end].contains('{');
                if (opened && body_depth <= 0) || (!opened && lines[end].trim_end().ends_with(';'))
                {
                    break;
                }
                end += 1;
            }
            let (qualified, in_drop_impl) = match &current_impl {
                Some((type_name, drop, _)) => (format!("{type_name}::{name}"), *drop),
                None => (name.to_string(), false),
            };
            let signature = body.split('{').next().unwrap_or_default();
            found.push(Function {
                qualified,
                name: name.to_string(),
                attributes: attributes.join("\n"),
                takes_self: signature.contains("self"),
                body,
                in_drop_impl,
            });
            for skipped in &lines[index..=end.min(lines.len() - 1)] {
                depth += brace_delta(skipped);
            }
            index = end + 1;
            if let Some((_, _, impl_depth)) = &current_impl {
                if depth <= *impl_depth {
                    current_impl = None;
                }
            }
            continue;
        }
        depth += brace_delta(line);
        if let Some((_, _, impl_depth)) = &current_impl {
            if depth <= *impl_depth && line.contains('}') {
                current_impl = None;
            }
        }
        index += 1;
    }
    found
}

fn calls(body: &str, name: &str) -> bool {
    body.match_indices(name).any(|(at, _)| {
        let before = body[..at].chars().next_back();
        let after = body[at + name.len()..].chars().next();
        after == Some('(')
            && !before.is_some_and(|character| character.is_alphanumeric() || character == '_')
    })
}

/// A lib test source: its path and the part of it that is test code.
struct TestSource {
    path: String,
    test_code: String,
}

/// Functions whose body changes the process environment, followed through
/// the helpers that call them (to a fixed point). Drop impls only restore.
struct EnvChangers {
    /// `Type::name` of associated functions, matched anywhere.
    qualified: BTreeSet<String>,
    /// Methods (`.name(`) of any scanned file.
    methods: BTreeSet<String>,
}

fn env_changers(sources: &[String]) -> EnvChangers {
    let all: Vec<Function> = sources
        .iter()
        .flat_map(|source| functions(source))
        .collect();
    let mut changers = EnvChangers {
        qualified: BTreeSet::new(),
        methods: BTreeSet::new(),
    };
    loop {
        let before = changers.qualified.len() + changers.methods.len();
        for function in all.iter().filter(|function| !function.in_drop_impl) {
            let changes = function.changes_env_directly()
                || changers
                    .qualified
                    .iter()
                    .any(|name| calls(&function.body, name))
                || changers
                    .methods
                    .iter()
                    .any(|name| function.body.contains(&format!(".{name}(")));
            if changes {
                if function.takes_self {
                    changers.methods.insert(function.name.clone());
                } else if function.qualified.contains("::") {
                    changers.qualified.insert(function.qualified.clone());
                }
            }
        }
        if changers.qualified.len() + changers.methods.len() == before {
            return changers;
        }
    }
}

/// Tests in `source` that change the environment without the default
/// `#[serial]` guard, plus module-local locks and keyed serial groups.
fn offenders(source: &TestSource, changers: &EnvChangers) -> Vec<String> {
    let functions = functions(&source.test_code);
    // Same-file free helpers that change the environment, to a fixed point.
    let mut local: BTreeSet<&str> = BTreeSet::new();
    loop {
        let before = local.len();
        for function in functions.iter().filter(|function| !function.in_drop_impl) {
            let changes = function.changes_env_directly()
                || local.iter().any(|name| calls(&function.body, name))
                || changers
                    .qualified
                    .iter()
                    .any(|name| calls(&function.body, name))
                || changers
                    .methods
                    .iter()
                    .any(|name| function.body.contains(&format!(".{name}(")));
            if changes && !function.is_test() {
                local.insert(function.name.as_str());
            }
        }
        if local.len() == before {
            break;
        }
    }
    let mut found = Vec::new();
    for function in functions.iter().filter(|function| function.is_test()) {
        let changes = function.changes_env_directly()
            || local.iter().any(|name| calls(&function.body, name))
            || changers
                .qualified
                .iter()
                .any(|name| calls(&function.body, name))
            || changers
                .methods
                .iter()
                .any(|name| function.body.contains(&format!(".{name}(")));
        if changes && !function.has_default_serial() {
            found.push(format!(
                "{}: {} changes the environment without #[serial]",
                source.path, function.name
            ));
        }
    }
    for line in source.test_code.lines().map(str::trim) {
        if line.starts_with("#[serial(") || line.starts_with("#[serial_test::serial(") {
            found.push(format!("{}: keyed serial group {line}", source.path));
        }
        if line.contains("fn env_lock(") || (line.contains("static ") && line.contains("Mutex<()>"))
        {
            found.push(format!("{}: module-local lock {line}", source.path));
        }
        if line.contains("set_var(\"FSPEC_HOME\"") && !source.path.ends_with(SHARED_FSPEC_HOME) {
            found.push(format!(
                "{}: redirects FSPEC_HOME itself instead of through {SHARED_FSPEC_HOME}",
                source.path
            ));
        }
    }
    found
}

fn providers_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rust_files(directory: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            rust_files(&path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    Ok(())
}

fn is_test_file(path: &Path) -> bool {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .is_some_and(|stem| {
            stem.starts_with("tests") || stem.ends_with("_tests") || stem.ends_with("_test")
        })
        || path
            .components()
            .any(|part| part.as_os_str() == "test_fixtures")
}

/// The lib's test code: whole unit-test files, and everything from the
/// first `#[cfg(test)]` of other source files. Also returns every source
/// (production code and the shared fixtures under tests/ the lib includes)
/// for finding the functions that change the environment.
fn lib_sources() -> Result<(Vec<TestSource>, Vec<String>), Box<dyn std::error::Error>> {
    let root = providers_root();
    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files)?;
    files.sort();
    let mut tests = Vec::new();
    let mut everything = Vec::new();
    for path in files {
        let source = std::fs::read_to_string(&path)?;
        let relative = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .display()
            .to_string();
        let test_code = if is_test_file(&path) {
            Some(source.clone())
        } else {
            source
                .find("#[cfg(test)]")
                .map(|start| source[start..].to_string())
        };
        for line in source.lines() {
            if let Some(rest) = line.trim().strip_prefix("#[path = \"") {
                if let Some((included, _)) = rest.split_once('"') {
                    let directory = path.parent().unwrap_or(&root);
                    if let Ok(text) = std::fs::read_to_string(directory.join(included)) {
                        everything.push(text);
                    }
                }
            }
        }
        everything.push(source);
        if let Some(test_code) = test_code {
            tests.push(TestSource {
                path: relative,
                test_code,
            });
        }
    }
    Ok((tests, everything))
}

#[test]
fn every_providers_lib_test_that_changes_the_environment_runs_under_the_shared_guard() -> TestResult
{
    // @step Given the providers lib test sources
    let (tests, everything) = lib_sources()?;
    assert!(tests.len() > 50, "the guard found too few lib test sources");

    // @step When the guard scans every test that changes the environment directly, through a same-file helper or through a shared env fixture
    let changers = env_changers(&everything);
    let found: Vec<String> = tests
        .iter()
        .flat_map(|source| offenders(source, &changers))
        .collect();

    // @step Then each such test carries the default #[serial] attribute, and no module-local lock or keyed serial group exists
    assert!(
        found.is_empty(),
        "providers lib tests outside the shared environment guard:\n{}",
        found.join("\n")
    );
    Ok(())
}

#[test]
fn the_guard_reports_a_test_that_changes_the_environment_through_a_helper_without_the_shared_guard(
) -> TestResult {
    // @step Given a lib test source whose test calls a same-file helper that sets OPENAI_API_KEY and has no #[serial] attribute
    let source = TestSource {
        path: "src/example_tests.rs".to_string(),
        test_code: r#"
fn with_key<T>(f: impl FnOnce() -> T) -> T {
    std::env::set_var("OPENAI_API_KEY", "test");
    f()
}

#[test]
fn unguarded() {
    with_key(|| ());
}

#[test]
#[serial_test::serial]
fn serialised() {
    with_key(|| ());
}
"#
        .to_string(),
    };

    // @step When the guard scans it
    let found = offenders(
        &source,
        &EnvChangers {
            qualified: BTreeSet::new(),
            methods: BTreeSet::new(),
        },
    );

    // @step Then the test is reported with its file and name
    assert_eq!(
        found,
        vec![
            "src/example_tests.rs: unguarded changes the environment without #[serial]".to_string()
        ]
    );
    Ok(())
}
