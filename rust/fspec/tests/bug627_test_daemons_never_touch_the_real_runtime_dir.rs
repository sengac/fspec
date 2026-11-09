//! BUG-627 — test daemons never touch the real runtime dir: the harness
//! refuses a daemon spawn that is not isolated, and every daemon or PTY
//! spawn in the fspec tests goes through it.
//!
//! Feature: spec/features/test-daemons-never-touch-the-real-runtime-dir.feature

mod common;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use common::{fspec_bin, make_workspace, spawn_fspec_daemon, IsolatedCommand, TestHome};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn a_daemon_spawn_that_inherits_the_test_process_s_runtime_dir_is_refused() -> TestResult {
    // @step Given an "fspec daemon" command whose environment the test left untouched
    let marker = tempfile::tempdir()?;
    let mut command = IsolatedCommand::new(fspec_bin());
    command.arg("daemon").current_dir(marker.path());

    // @step When the harness checks it before spawning
    let refusal = command
        .check()
        .err()
        .ok_or("the untouched command was accepted")?;
    let spawned = command.spawn();

    // @step Then it refuses naming XDG_RUNTIME_DIR as inherited from the test process
    // HOME is checked first; a command with only HOME isolated names XDG_RUNTIME_DIR.
    assert!(refusal.contains("HOME is inherited"), "{refusal}");
    let home = TestHome::new();
    let mut home_only = IsolatedCommand::new(fspec_bin());
    home_only.arg("daemon").env("HOME", home.home());
    let refusal = home_only
        .check()
        .err()
        .ok_or("the inherited runtime dir was accepted")?;
    assert!(
        refusal.contains("XDG_RUNTIME_DIR is inherited from the test process"),
        "{refusal}"
    );
    // @step And no daemon process is started
    let error = spawned.err().ok_or("the refused daemon was started")?;
    assert_eq!(
        error.kind(),
        std::io::ErrorKind::PermissionDenied,
        "{error}"
    );
    Ok(())
}

#[test]
fn a_daemon_spawn_with_home_outside_the_temp_dir_is_refused() -> TestResult {
    // @step Given an "fspec daemon" command with XDG_RUNTIME_DIR in a temp dir and HOME set to the user's real home
    let runtime = tempfile::tempdir()?;
    let real_home = PathBuf::from(std::env::var_os("HOME").ok_or("the test process has no HOME")?);
    let mut command = IsolatedCommand::new(fspec_bin());
    command
        .arg("daemon")
        .env("HOME", &real_home)
        .env("XDG_RUNTIME_DIR", runtime.path());

    // @step When the harness checks it before spawning
    let refusal = command.check().err().ok_or("the real HOME was accepted")?;

    // @step Then it refuses naming HOME and that path
    assert!(
        refusal.contains(&format!(
            "HOME={} is not inside the temp dir",
            real_home.display()
        )),
        "{refusal}"
    );
    Ok(())
}

/// Wait up to 10 s for `path` to exist.
fn wait_for(path: &Path) -> bool {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if path.exists() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

#[test]
fn an_isolated_test_daemon_writes_its_discovery_files_under_the_test_s_temp_dir() -> TestResult {
    // @step Given a test home with home and runtime dirs inside a temp dir
    let home = TestHome::new();
    let (workspace, _path) = make_workspace(&[("ISO-1", "isolated", "backlog")]);

    // @step When spawn_fspec_daemon starts a daemon with that test home
    let (guard, _port) = spawn_fspec_daemon(workspace.path(), &home);
    let pid = guard.0.id();

    // @step Then the daemon's daemons/<pid>.json and daemon.json are under the test home's runtime dir
    let entry = home.daemons_dir().join(format!("{pid}.json"));
    assert!(
        wait_for(&entry),
        "no discovery entry at {}",
        entry.display()
    );
    assert!(
        wait_for(&home.daemon_json()),
        "no daemon.json at {}",
        home.daemon_json().display()
    );
    let pointer: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(home.daemon_json())?)?;
    assert_eq!(pointer["pid"].as_u64(), Some(u64::from(pid)), "{pointer}");
    drop(guard);
    Ok(())
}

#[test]
fn separate_temp_dirs_or_a_temp_home_with_xdg_runtime_dir_removed_are_accepted() -> TestResult {
    // @step Given an "fspec daemon" command with HOME and XDG_RUNTIME_DIR in two different temp dirs, and another with a temp HOME and XDG_RUNTIME_DIR removed
    let home = tempfile::tempdir()?;
    let runtime = tempfile::tempdir()?;
    let mut separate = IsolatedCommand::new(fspec_bin());
    separate
        .arg("daemon")
        .env("HOME", home.path())
        .env("XDG_RUNTIME_DIR", runtime.path());
    let mut removed = IsolatedCommand::new(fspec_bin());
    removed
        .arg("daemon")
        .env("HOME", home.path())
        .env_remove("XDG_RUNTIME_DIR");

    // @step When the harness checks them before spawning
    let checks = [separate.check(), removed.check()];

    // @step Then both are accepted
    for check in checks {
        check?;
    }
    Ok(())
}

/// One daemon or PTY spawn built outside the harness.
#[derive(Debug, PartialEq, Eq)]
struct Violation {
    file: String,
    line: usize,
    what: &'static str,
}

/// `source` without `//` and `/* */` comments, keeping string and char
/// literals intact (a `//` inside `"ws://..."` is not a comment).
fn strip_comments(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(source.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '/' && next == Some('*') {
            i += 2;
            while i < chars.len() && !(chars[i] == '*' && chars.get(i + 1) == Some(&'/')) {
                if chars[i] == '\n' {
                    out.push('\n');
                }
                i += 1;
            }
            i += 2;
            continue;
        }
        if c == 'r' && matches!(next, Some('"') | Some('#')) {
            // A raw string: r"...", r#"..."#.
            let mut j = i + 1;
            let mut hashes = 0;
            while chars.get(j) == Some(&'#') {
                hashes += 1;
                j += 1;
            }
            if chars.get(j) == Some(&'"') {
                let end: String = std::iter::once('"')
                    .chain(std::iter::repeat_n('#', hashes))
                    .collect();
                let rest: String = chars[j + 1..].iter().collect();
                let len = rest.find(&end).map_or(rest.len(), |at| at + end.len());
                out.extend(&chars[i..j + 1 + rest[..len].chars().count()]);
                i = j + 1 + rest[..len].chars().count();
                continue;
            }
        }
        if c == '"' {
            out.push(c);
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' {
                    out.push(chars[i]);
                    i += 1;
                }
                if let Some(&ch) = chars.get(i) {
                    out.push(ch);
                }
                i += 1;
            }
            if i < chars.len() {
                out.push('"');
            }
            i += 1;
            continue;
        }
        if c == '\'' {
            // A char literal ('x', '\'', '\n'); a lifetime is left alone.
            let len = if next == Some('\\') { 4 } else { 3 };
            if chars.get(i + len - 1) == Some(&'\'') {
                out.extend(&chars[i..i + len]);
                i += len;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

/// The text of the statement starting at `start`: up to the first `;` at
/// bracket depth 0 outside string literals (or an unmatched closing bracket).
fn statement(source: &str, start: usize) -> &str {
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    for (offset, c) in source[start..].char_indices() {
        if in_string {
            match c {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => {
                depth -= 1;
                if depth < 0 {
                    return &source[start..start + offset];
                }
            }
            ';' if depth == 0 => return &source[start..start + offset],
            _ => {}
        }
    }
    &source[start..]
}

fn line_of(source: &str, offset: usize) -> usize {
    source[..offset].matches('\n').count() + 1
}

/// Whether `text` passes `daemon` as an argument (`.arg("daemon")` or a
/// `"daemon"` element of an `.args([...])` list).
fn passes_daemon(text: &str) -> bool {
    let arg = regex::Regex::new(r#"\.arg\(\s*"daemon"\s*\)"#).expect("arg regex");
    let args = regex::Regex::new(r#"\.args\(\s*&?\[[^\]]*"daemon""#).expect("args regex");
    arg.is_match(text) || args.is_match(text)
}

/// Whether `text` starts combined mode: it passes string-literal arguments,
/// none of them a subcommand (every one a `--flag`), and none of them
/// `--help` / `--version`.
fn starts_combined_mode(text: &str) -> bool {
    let single = regex::Regex::new(r#"\.arg\(\s*"([^"]*)"\s*\)"#).expect("arg regex");
    let list = regex::Regex::new(r#"\.args\(\s*&?\[([^\]]*)\]"#).expect("args regex");
    let quoted = regex::Regex::new(r#""([^"]*)""#).expect("quoted regex");
    let mut args: Vec<&str> = single
        .captures_iter(text)
        .filter_map(|c| c.get(1).map(|m| m.as_str()))
        .collect();
    for items in list.captures_iter(text).filter_map(|c| c.get(1)) {
        args.extend(
            quoted
                .captures_iter(items.as_str())
                .filter_map(|c| c.get(1).map(|m| m.as_str())),
        );
    }
    !args.is_empty()
        && args.iter().all(|a| a.starts_with('-'))
        && !args
            .iter()
            .any(|a| matches!(*a, "--help" | "-h" | "--version" | "-V"))
}

/// BUG-627 rule 3: in `source` (comments stripped), every
/// `Command::new(<fspec binary>)` whose statement passes the daemon
/// subcommand or starts combined mode, every later `<var>.arg("daemon")` on
/// a variable bound to one, and every `CommandBuilder::new(` (a PTY spawn).
fn violations(file: &str, source: &str) -> Vec<Violation> {
    let source = strip_comments(source);
    let mut found = Vec::new();
    let new = regex::Regex::new(r"(?:std::process::)?\bCommand::new\(\s*(?:common::)?(?:fspec_bin\(\)|env!\(\s*\x22CARGO_BIN_EXE_fspec\x22\s*\))")
        .expect("Command::new regex");
    let binding = regex::Regex::new(r"let\s+(?:mut\s+)?(\w+)\s*=\s*$").expect("binding regex");
    for m in new.find_iter(&source) {
        let text = statement(&source, m.start());
        if passes_daemon(text) {
            found.push(Violation {
                file: file.to_owned(),
                line: line_of(&source, m.start()),
                what: "fspec daemon spawn built outside the harness",
            });
            continue;
        }
        if starts_combined_mode(text) {
            found.push(Violation {
                file: file.to_owned(),
                line: line_of(&source, m.start()),
                what: "fspec combined-mode spawn built outside the harness",
            });
            continue;
        }
        let before = &source[source[..m.start()].rfind('\n').map_or(0, |i| i + 1)..m.start()];
        if let Some(var) = binding.captures(before).and_then(|c| c.get(1)) {
            let var = regex::escape(var.as_str());
            let later =
                regex::Regex::new(&format!(r"\b{var}\s*\.\s*args?\(")).expect("variable regex");
            for use_site in later.find_iter(&source[m.end()..]) {
                let at = m.end() + use_site.start();
                if passes_daemon(statement(&source, at)) {
                    found.push(Violation {
                        file: file.to_owned(),
                        line: line_of(&source, at),
                        what: "fspec daemon spawn built outside the harness",
                    });
                }
            }
        }
    }
    for m in regex::Regex::new(r"\bCommandBuilder::new\(")
        .expect("pty regex")
        .find_iter(&source)
    {
        found.push(Violation {
            file: file.to_owned(),
            line: line_of(&source, m.start()),
            what: "PTY spawn built outside the harness (use common::fspec_pty_command)",
        });
    }
    found
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            rust_files(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    Ok(())
}

#[test]
fn every_daemon_and_pty_spawn_in_the_fspec_tests_goes_through_the_harness() -> TestResult {
    // @step Given the integration test sources under rust/fspec/tests
    let tests = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    let harness = tests.join("common");
    let mut files = Vec::new();
    rust_files(&tests, &mut files)?;
    files.sort();

    // @step When they are scanned for fspec daemon and PTY spawns built outside the checked harness
    let mut found = Vec::new();
    for file in files.iter().filter(|f| !f.starts_with(&harness)) {
        let name = file.strip_prefix(&tests)?.display().to_string();
        found.extend(violations(&name, &std::fs::read_to_string(file)?));
    }

    // @step Then none is found
    assert!(found.is_empty(), "{found:#?}");
    // @step And a spawn of Command::new(fspec_bin()).arg("daemon") outside the harness would be reported with its file and line
    // (Built from pieces so this file's own scan does not see it.)
    let pty = ["Command", "Builder::new("].concat();
    let sample = format!(
        "fn t() {{\n    let child = Command::new(fspec_bin())\n        .arg(\"daemon\")\n        .spawn();\n    let mut cmd = Command::new(fspec_bin());\n    cmd.arg(\"daemon\");\n    let pty = {pty}fspec_bin());\n    let tui = Command::new(fspec_bin()).arg(\"--workspace\").arg(ws).spawn();\n    let help = Command::new(fspec_bin()).arg(\"--help\").output();\n}}\n"
    );
    let reported: Vec<(usize, &str)> = violations("sample.rs", &sample)
        .iter()
        .map(|v| (v.line, v.what))
        .collect();
    assert_eq!(
        reported,
        vec![
            (2, "fspec daemon spawn built outside the harness"),
            (6, "fspec daemon spawn built outside the harness"),
            (8, "fspec combined-mode spawn built outside the harness"),
            (
                7,
                "PTY spawn built outside the harness (use common::fspec_pty_command)"
            ),
        ]
    );
    Ok(())
}
