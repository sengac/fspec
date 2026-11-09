// Feature: spec/features/concurrent-commands-never-corrupt-spec-json-files.feature
//
// BUG-477: concurrent fspec commands corrupted spec/work-units.json (a valid
// document followed by the tail of an earlier, longer write) and could lose
// updates. Every scenario here runs its writers in SEPARATE PROCESSES: the
// test binary re-executes itself (`--exact bug477_child_worker`) with
// BUG477_* environment variables, and each child drives the real commands
// through `dispatch_command`. A child records its outcome in
// `<project>/outcomes/<name>.json` so the parent can assert on it.
//
// The spec lock is the stable file `spec/.fspec-write.lock` held with an
// exclusive advisory lock (fs2). The lock-holder child takes it with fs2
// directly, so the scenario exercises the cross-process protocol itself.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Instant;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use fs2::FileExt;
use serde_json::{json, Value};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const LOCK_FILE: &str = ".fspec-write.lock";
const WORK_UNIT: &str = "LOCK-001";
const HOLDER_READY: &str = "bug477-holder-holds-the-spec-lock";
const WRITERS: usize = 8;
const RULES_PER_WRITER: usize = 10;
const LOCK_TIMEOUT_ENV: &str = "FSPEC_SPEC_LOCK_TIMEOUT_MS";
const BUDGET_MS: u64 = 3_000;

// ---------------------------------------------------------------- fixture

fn seed_project() -> Result<TempDir, Box<dyn std::error::Error>> {
    let dir = TempDir::new()?;
    let spec = dir.path().join("spec");
    fs::create_dir_all(&spec)?;
    fs::create_dir_all(dir.path().join("outcomes"))?;
    let board = json!({
        "meta": {"version": "0.7.1", "lastUpdated": "2026-11-04T00:00:00.000Z"},
        "workUnits": {
            WORK_UNIT: {
                "id": WORK_UNIT,
                "title": "Concurrent writers",
                "type": "story",
                "status": "specifying",
                "createdAt": "2026-11-04T00:00:00.000Z",
                "updatedAt": "2026-11-04T00:00:00.000Z"
            },
            "FLIP-001": {
                "id": "FLIP-001",
                "title": "Long and short",
                "type": "story",
                "status": "backlog",
                "createdAt": "2026-11-04T00:00:00.000Z",
                "updatedAt": "2026-11-04T00:00:00.000Z"
            }
        },
        "states": {
            "backlog": ["FLIP-001"], "specifying": [WORK_UNIT], "testing": [],
            "implementing": [], "validating": [], "done": [], "blocked": []
        }
    });
    fs::write(
        spec.join("work-units.json"),
        serde_json::to_string_pretty(&board)?,
    )?;
    let foundation = json!({
        "version": "2.0.0",
        "project": {"name": "bug477", "vision": "v", "projectType": "cli-tool"},
        "problemSpace": {"primaryProblem": {"title": "t", "description": "d", "impact": "high"}},
        "solutionSpace": {"overview": "o", "capabilities": []},
        "personas": [],
        "architectureDiagrams": []
    });
    fs::write(
        spec.join("foundation.json"),
        serde_json::to_string_pretty(&foundation)?,
    )?;
    Ok(dir)
}

fn read_json(path: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

fn child_command(root: &Path, role: &str, name: &str) -> std::io::Result<Command> {
    let mut command = Command::new(std::env::current_exe()?);
    command
        .args(["--exact", "bug477_child_worker", "--nocapture", "--quiet"])
        .env("BUG477_ROOT", root)
        .env("BUG477_ROLE", role)
        .env("BUG477_NAME", name);
    Ok(command)
}

fn spawn_child(root: &Path, role: &str, name: &str) -> std::io::Result<Child> {
    child_command(root, role, name)?.spawn()
}

fn wait_all(children: Vec<Child>) -> TestResult {
    for mut child in children {
        let status = child.wait()?;
        assert!(status.success(), "child process exited with {status}");
    }
    Ok(())
}

/// Every outcome file a child wrote: (name, outcome).
fn outcomes(root: &Path) -> Result<Vec<(String, Value)>, Box<dyn std::error::Error>> {
    let mut all = Vec::new();
    for entry in fs::read_dir(root.join("outcomes"))? {
        let path = entry?.path();
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        all.push((name, read_json(&path)?));
    }
    all.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(all)
}

fn assert_every_outcome_succeeded(root: &Path, expected: usize) -> TestResult {
    let all = outcomes(root)?;
    assert_eq!(all.len(), expected, "every child records an outcome");
    for (name, outcome) in &all {
        assert_eq!(
            outcome["failures"],
            json!([]),
            "child {name} had failing commands: {outcome}"
        );
    }
    Ok(())
}

/// The one JSON document in `raw`, rejecting anything after it — including
/// the trailing bytes of the 2026-11-04 corruption.
fn parse_single_document(raw: &str) -> Result<Value, String> {
    let mut stream = serde_json::Deserializer::from_str(raw).into_iter::<Value>();
    let first = match stream.next() {
        Some(Ok(v)) => v,
        Some(Err(e)) => return Err(format!("not JSON: {e}")),
        None => return Err("empty file".to_string()),
    };
    let rest = &raw[stream.byte_offset()..];
    if !rest.trim().is_empty() {
        return Err(format!(
            "{} bytes after the closing brace: {:?}",
            rest.len(),
            rest.chars().take(80).collect::<String>()
        ));
    }
    Ok(first)
}

fn spec_entries(root: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut names = Vec::new();
    for entry in fs::read_dir(root.join("spec"))? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            names.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    names.sort();
    Ok(names)
}

/// Spawn a child that takes the spec lock and holds it until its stdin is
/// closed. Returns once the child reports on stdout that it holds it
/// (TEST-042: the parent waits on that line, not on a polled file).
fn hold_spec_lock_in_child(root: &Path) -> Result<Child, Box<dyn std::error::Error>> {
    let mut child = child_command(root, "hold-lock", "holder")?
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    let stdout = child.stdout.take().ok_or("the holder's stdout is piped")?;
    let mut lines = BufReader::new(stdout).lines();
    loop {
        match lines.next() {
            Some(line) => {
                if line?.trim() == HOLDER_READY {
                    break;
                }
            }
            None => return Err("lock holder exited before it held the lock".into()),
        }
    }
    // Keep reading, so the holder's later output (the test harness's own
    // report) never hits a closed pipe.
    std::thread::spawn(move || lines.for_each(drop));
    Ok(child)
}

/// Let the holder go: closing its stdin releases the lock.
fn release_holder(mut holder: Child) -> TestResult {
    drop(holder.stdin.take());
    wait_all(vec![holder])
}

// ------------------------------------------------------------------ child

fn dispatch(root: &Path, command: &str, args: Value) -> Result<(), String> {
    let result = dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: root.to_path_buf(),
    });
    if !result.success {
        return Err(result.error.unwrap_or_else(|| "no error text".to_string()));
    }
    if let Ok(data) = serde_json::from_str::<Value>(&result.data) {
        if data.get("success") == Some(&Value::Bool(false)) {
            return Err(result.data);
        }
    }
    Ok(())
}

fn letter(i: usize) -> char {
    (b'A' + (i % 26) as u8) as char
}

fn child_commands(role: &str, index: usize) -> Vec<(&'static str, Value)> {
    match role {
        "rules" => (0..RULES_PER_WRITER)
            .map(|k| {
                (
                    "add-rule",
                    json!({"workUnitId": WORK_UNIT, "rule": format!("rule {index}-{k}")}),
                )
            })
            .collect(),
        "tags" => (0..RULES_PER_WRITER)
            .map(|k| {
                (
                    "register-tag",
                    json!({"tag": format!("@bug477-tag-{index}-{k}"),
                           "category": "Technical Tags", "description": "d"}),
                )
            })
            .collect(),
        "epics" => (0..RULES_PER_WRITER)
            .map(|k| {
                (
                    "create-epic",
                    json!({"epicId": format!("bug477-epic-{index}-{k}"), "title": "t"}),
                )
            })
            .collect(),
        "prefixes" => (0..RULES_PER_WRITER)
            .map(|k| {
                (
                    "create-prefix",
                    json!({"prefix": format!("P{}{}", letter(index), letter(k)),
                           "description": "d"}),
                )
            })
            .collect(),
        "personas" => (0..RULES_PER_WRITER)
            .map(|k| {
                (
                    "add-persona",
                    json!({"name": format!("Persona {index}-{k}"), "description": "d",
                           "goals": ["g"]}),
                )
            })
            .collect(),
        "flip" => (0..RULES_PER_WRITER * 2)
            .map(|k| {
                let description = if k % 2 == 0 {
                    "long ".repeat(4096)
                } else {
                    "short".to_string()
                };
                (
                    "update-work-unit",
                    json!({"workUnitId": "FLIP-001", "description": description}),
                )
            })
            .collect(),
        "rule-once" => vec![(
            "add-rule",
            json!({"workUnitId": WORK_UNIT, "rule": "while the lock is held"}),
        )],
        _ => Vec::new(),
    }
}

fn hold_lock(root: &Path) -> TestResult {
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(root.join("spec").join(LOCK_FILE))?;
    file.lock_exclusive()?;
    let mut stdout = std::io::stdout();
    writeln!(stdout, "{HOLDER_READY}")?;
    stdout.flush()?;
    // Hold the lock until the parent closes stdin.
    std::io::stdin().read_to_end(&mut Vec::new())?;
    FileExt::unlock(&file)?;
    Ok(())
}

/// Child-process entry point. A no-op unless BUG477_ROLE is set.
#[test]
fn bug477_child_worker() -> TestResult {
    let (Ok(root), Ok(role), Ok(name)) = (
        std::env::var("BUG477_ROOT"),
        std::env::var("BUG477_ROLE"),
        std::env::var("BUG477_NAME"),
    ) else {
        return Ok(());
    };
    let root = PathBuf::from(root);
    if role == "hold-lock" {
        return hold_lock(&root);
    }
    let index: usize = name
        .rsplit('-')
        .next()
        .and_then(|n| n.parse().ok())
        .unwrap_or(0);
    let started = Instant::now();
    let mut failures = Vec::new();
    for (command, args) in child_commands(&role, index) {
        if let Err(error) = dispatch(&root, command, args) {
            failures.push(json!({"command": command, "error": error}));
        }
    }
    let outcome = json!({
        "failures": failures,
        "elapsedMs": started.elapsed().as_millis() as u64,
    });
    fs::write(
        root.join("outcomes").join(format!("{name}.json")),
        outcome.to_string(),
    )?;
    Ok(())
}

// -------------------------------------------------------------- scenarios

#[test]
fn concurrent_board_writers_in_separate_processes_lose_no_update() -> TestResult {
    // @step Given a project whose board has the work unit "LOCK-001"
    let project = seed_project()?;
    let root = project.path();

    // @step When eight separate processes each add ten rules to "LOCK-001" at the same time
    let children = (0..WRITERS)
        .map(|i| spawn_child(root, "rules", &format!("rules-{i}")))
        .collect::<Result<Vec<_>, _>>()?;
    wait_all(children)?;

    // @step Then every process succeeds
    assert_every_outcome_succeeded(root, WRITERS)?;

    // @step And spec/work-units.json parses as one JSON document
    let raw = fs::read_to_string(root.join("spec").join("work-units.json"))?;
    let board = parse_single_document(&raw)?;

    // @step And "LOCK-001" has all 80 rules
    let rules = board["workUnits"][WORK_UNIT]["rules"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut texts: Vec<String> = rules
        .iter()
        .filter_map(|r| r["text"].as_str().or_else(|| r.as_str()).map(String::from))
        .collect();
    texts.sort();
    let mut expected: Vec<String> = (0..WRITERS)
        .flat_map(|i| (0..RULES_PER_WRITER).map(move |k| format!("rule {i}-{k}")))
        .collect();
    expected.sort();
    assert_eq!(texts, expected, "a concurrent add-rule update was lost");
    Ok(())
}

#[test]
fn concurrent_writers_of_tags_epics_prefixes_and_foundation_lose_no_update() -> TestResult {
    // @step Given a project with tags.json, epics.json, prefixes.json and foundation.json
    let project = seed_project()?;
    let root = project.path();
    for (command, args) in [
        (
            "register-tag",
            json!({"tag": "@bug477-seed", "category": "Technical Tags", "description": "d"}),
        ),
        (
            "create-epic",
            json!({"epicId": "bug477-seed", "title": "t"}),
        ),
        (
            "create-prefix",
            json!({"prefix": "SEED", "description": "d"}),
        ),
    ] {
        dispatch(root, command, args)?;
    }

    // @step When separate processes register tags, create epics, create prefixes and add personas at the same time
    let roles = ["tags", "epics", "prefixes", "personas"];
    let mut children = Vec::new();
    for role in roles {
        for i in 0..2 {
            children.push(spawn_child(root, role, &format!("{role}-{i}"))?);
        }
    }
    wait_all(children)?;

    // @step Then every process succeeds
    assert_every_outcome_succeeded(root, roles.len() * 2)?;

    // @step And tags.json, epics.json, prefixes.json and foundation.json each parse and contain every entry
    let spec = root.join("spec");
    let tags = parse_single_document(&fs::read_to_string(spec.join("tags.json"))?)?;
    let epics = parse_single_document(&fs::read_to_string(spec.join("epics.json"))?)?;
    let prefixes = parse_single_document(&fs::read_to_string(spec.join("prefixes.json"))?)?;
    let foundation = parse_single_document(&fs::read_to_string(spec.join("foundation.json"))?)?;
    let tag_names: Vec<String> = tags["categories"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .flat_map(|c| c["tags"].as_array().cloned().unwrap_or_default())
        .filter_map(|t| t["name"].as_str().map(String::from))
        .collect();
    let persona_names: Vec<String> = foundation["personas"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|p| p["name"].as_str().map(String::from))
        .collect();
    for i in 0..2 {
        for k in 0..RULES_PER_WRITER {
            let tag = format!("@bug477-tag-{i}-{k}");
            assert!(tag_names.contains(&tag), "tags.json lost {tag}");
            let epic = format!("bug477-epic-{i}-{k}");
            assert!(
                epics["epics"].get(&epic).is_some(),
                "epics.json lost {epic}"
            );
            let prefix = format!("P{}{}", letter(i), letter(k));
            assert!(
                prefixes["prefixes"].get(&prefix).is_some(),
                "prefixes.json lost {prefix}"
            );
            let persona = format!("Persona {i}-{k}");
            assert!(
                persona_names.contains(&persona),
                "foundation.json lost {persona}"
            );
        }
    }
    Ok(())
}

#[test]
fn a_shorter_write_never_leaves_the_tail_of_a_longer_one() -> TestResult {
    let project = seed_project()?;
    let root = project.path();

    // @step Given a writer process that alternately writes a long and a short spec/work-units.json
    let mut children = vec![spawn_child(root, "flip", "flip-0")?];

    // @step And other processes adding rules at the same time
    for i in 0..4 {
        children.push(spawn_child(root, "rules", &format!("rules-{i}"))?);
    }

    // @step When a reader process polls spec/work-units.json throughout
    let path = root.join("spec").join("work-units.json");
    let mut bad_reads = Vec::new();
    let mut reads = 0usize;
    loop {
        let all_done = children
            .iter_mut()
            .map(|c| c.try_wait().map(|s| s.is_some()))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .all(|done| done);
        match fs::read_to_string(&path) {
            Ok(raw) => {
                reads += 1;
                if let Err(e) = parse_single_document(&raw) {
                    bad_reads.push(e);
                }
            }
            Err(e) => bad_reads.push(format!("read failed: {e}")),
        }
        if all_done {
            break;
        }
    }
    wait_all(children)?;

    // @step Then every read parses as one JSON document with nothing after its closing brace
    assert!(reads > 0);
    assert!(
        bad_reads.is_empty(),
        "{} of {reads} reads were corrupt, first: {:?}",
        bad_reads.len(),
        bad_reads.first()
    );
    assert_every_outcome_succeeded(root, 5)?;
    Ok(())
}

#[test]
fn concurrent_writes_leave_no_temp_files_behind() -> TestResult {
    // @step Given a project where separate processes have written spec JSON files at the same time
    let project = seed_project()?;
    let root = project.path();
    let mut children = Vec::new();
    for role in ["rules", "tags", "epics", "prefixes", "personas"] {
        children.push(spawn_child(root, role, &format!("{role}-0"))?);
    }

    // @step When they have all finished
    wait_all(children)?;
    assert_every_outcome_succeeded(root, 5)?;

    // @step Then spec/ holds no temp file next to the JSON files
    let allowed = [
        LOCK_FILE,
        "epics.json",
        "foundation.json",
        "prefixes.json",
        "tags.json",
        "work-units.json",
    ];
    let unexpected: Vec<String> = spec_entries(root)?
        .into_iter()
        .filter(|name| !allowed.contains(&name.as_str()) && !name.ends_with(".md"))
        .collect();
    assert!(
        unexpected.is_empty(),
        "left behind in spec/: {unexpected:?}"
    );
    Ok(())
}

#[test]
fn a_writer_that_cannot_get_the_lock_fails_and_changes_nothing() -> TestResult {
    let project = seed_project()?;
    let root = project.path();
    let board_path = root.join("spec").join("work-units.json");
    let before = fs::read(&board_path)?;

    // @step Given another process holds the spec lock and does not release it
    let holder = hold_spec_lock_in_child(root)?;

    // @step When add-rule runs against the same project
    // The lock budget is shortened through its documented override so the
    // scenario does not wait the default minute.
    let writer = child_command(root, "rule-once", "writer-0")?
        .env(LOCK_TIMEOUT_ENV, BUDGET_MS.to_string())
        .spawn()?;
    let writer_result = wait_all(vec![writer]);
    release_holder(holder)?;
    writer_result?;

    // @step Then it fails with "Failed to acquire file lock" after the lock budget
    let outcome = read_json(&root.join("outcomes").join("writer-0.json"))?;
    let error = outcome["failures"][0]["error"].as_str().unwrap_or_default();
    assert!(
        error.contains("Failed to acquire file lock"),
        "add-rule must fail on the held lock, outcome: {outcome}"
    );
    assert!(
        outcome["elapsedMs"].as_u64().unwrap_or_default() >= BUDGET_MS,
        "add-rule gives up only after the lock budget, outcome: {outcome}"
    );

    // @step And spec/work-units.json is byte-for-byte unchanged
    assert_eq!(fs::read(&board_path)?, before);
    Ok(())
}
