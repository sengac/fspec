//! TUI-123 — saved-daemon name and URL rules, locked atomic writes and
//! unreadable files.
//!
//! Feature: spec/features/daemon-management-on-par-with-workspaces.feature

use codelet_common::saved_daemons::{self, SavedDaemon, TokenChange};
use std::path::Path;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn daemon(name: &str, url: &str, token: Option<&str>) -> SavedDaemon {
    SavedDaemon {
        name: name.to_string(),
        url: url.to_string(),
        token: token.map(str::to_string),
        ca: None,
    }
}

/// Add `name` at `url` the way the TUI form does (no previous entry).
fn add(path: &Path, name: &str, url: &str, token: Option<&str>) -> Result<SavedDaemon, String> {
    let token = match token {
        Some(token) => TokenChange::Set(token.to_string()),
        None => TokenChange::Keep,
    };
    saved_daemons::update(path, None, name, url, token).map_err(|error| error.to_string())
}

#[test]
fn daemon_names_are_validated() -> TestResult {
    // @step Given an empty fspec home
    let home = tempfile::tempdir()?;
    let path = home.path().join(".fspec/daemons.json");
    let url = "ws://127.0.0.1:4000";

    // @step When these names are saved for "ws://127.0.0.1:4000":
    let padded = add(&path, "  work  ", url, None);
    let empty = add(&path, "", "ws://127.0.0.1:4001", None);
    let control = add(&path, "wo\trk", "ws://127.0.0.1:4002", None);
    let long_name = "a".repeat(65);
    let long = add(&path, &long_name, "ws://127.0.0.1:4003", None);
    let exactly_64 = add(&path, &"b".repeat(64), "ws://127.0.0.1:4004", None);

    // @step Then each name has the outcome in the table
    assert_eq!(padded?.name, "work");
    let empty = empty.err().ok_or("an empty name must be refused")?;
    assert!(empty.contains("required"), "{empty}");
    let control = control.err().ok_or("a control character must be refused")?;
    assert!(control.contains("control character"), "{control}");
    let long = long.err().ok_or("a 65-character name must be refused")?;
    assert!(long.contains("64 characters"), "{long}");
    assert!(exactly_64.is_ok(), "{exactly_64:?}");
    let names: Vec<String> = saved_daemons::list(&path)?
        .into_iter()
        .map(|daemon| daemon.name)
        .collect();
    assert_eq!(names, vec!["work".to_string(), "b".repeat(64)]);
    Ok(())
}

#[test]
fn daemon_urls_are_validated_and_normalised() -> TestResult {
    // @step Given an empty fspec home
    let home = tempfile::tempdir()?;
    let path = home.path().join(".fspec/daemons.json");

    // @step When these URLs are saved:
    let refused: &[(&str, Option<&str>, &str)] = &[
        ("http://fspec.example.com", Some("t"), "ws:// or wss://"),
        ("https://fspec.example.com", Some("t"), "ws:// or wss://"),
        ("fspec.example.com:7443", Some("t"), "scheme"),
        ("wss://", Some("t"), "host"),
        ("wss://:7443", Some("t"), "host"),
        (
            "wss://bob:hunter2@example.com",
            Some("t"),
            "user name or password",
        ),
        (
            "wss://fspec.example.com/?x=1",
            Some("t"),
            "query or fragment",
        ),
        (
            "wss://fspec.example.com/#top",
            Some("t"),
            "query or fragment",
        ),
        ("ws://10.0.0.5:9000", Some("t"), "wss://"),
        ("wss://fspec.example.com", None, "needs a token"),
    ];
    let mut outcomes = Vec::new();
    for (index, (url, token, _)) in refused.iter().enumerate() {
        outcomes.push(add(&path, &format!("r{index}"), url, *token));
    }
    let accepted: &[(&str, Option<&str>, &str)] = &[
        ("ws://localhost:4000", None, "ws://localhost:4000"),
        ("ws://127.0.0.2:4000", None, "ws://127.0.0.2:4000"),
        ("ws://[::1]:4000", None, "ws://[::1]:4000"),
        (
            "wss://fspec.example.com:8443/",
            Some("t"),
            "wss://fspec.example.com:8443",
        ),
    ];
    let mut saved = Vec::new();
    for (index, (url, token, _)) in accepted.iter().enumerate() {
        saved.push(add(&path, &format!("a{index}"), url, *token));
    }

    // @step Then each URL has the outcome in the table
    for ((url, _, reason), outcome) in refused.iter().zip(outcomes) {
        let error = outcome
            .err()
            .ok_or_else(|| format!("{url} must be refused"))?;
        assert!(error.contains(reason), "{url}: {error}");
        assert!(!error.contains("hunter2"), "{url}: {error}");
    }
    for ((url, _, normalised), outcome) in accepted.iter().zip(saved) {
        let daemon = outcome.map_err(|error| format!("{url}: {error}"))?;
        assert_eq!(daemon.url, *normalised, "{url}");
    }
    let listed: Vec<String> = saved_daemons::list(&path)?
        .into_iter()
        .map(|daemon| daemon.name)
        .collect();
    assert_eq!(listed, vec!["a0", "a1", "a2", "a3"]);
    Ok(())
}

#[test]
fn concurrent_saves_in_one_process_lose_nothing() -> TestResult {
    // @step Given an empty fspec home
    let home = tempfile::tempdir()?;
    let path = home.path().join(".fspec/daemons.json");

    // @step When 16 threads save different daemons at the same time
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(16));
    let workers: Vec<_> = (0..16)
        .map(|index| {
            let path = path.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                saved_daemons::save(
                    &path,
                    daemon(
                        &format!("d{index}"),
                        &format!("ws://127.0.0.1:{}", 5000 + index),
                        None,
                    ),
                )
                .map_err(|error| error.to_string())
            })
        })
        .collect();
    let mut failures = Vec::new();
    for worker in workers {
        if let Err(error) = worker.join().map_err(|_| "a saving thread panicked")? {
            failures.push(error);
        }
    }

    // @step Then daemons.json lists all 16 daemons
    assert!(failures.is_empty(), "{failures:?}");
    let mut names: Vec<String> = saved_daemons::list(&path)?
        .into_iter()
        .map(|daemon| daemon.name)
        .collect();
    names.sort();
    let mut expected: Vec<String> = (0..16).map(|index| format!("d{index}")).collect();
    expected.sort();
    assert_eq!(names, expected);
    Ok(())
}

#[test]
fn an_unreadable_daemons_json_is_an_error_and_is_left_in_place() -> TestResult {
    // @step Given daemons.json cannot be read as a file
    let home = tempfile::tempdir()?;
    let path = home.path().join(".fspec/daemons.json");
    std::fs::create_dir_all(path.join("keep"))?;

    // @step When the saved daemons are listed and a daemon is saved
    let listed = saved_daemons::list(&path);
    let saved = add(&path, "work", "ws://127.0.0.1:4000", None);

    // @step Then both fail naming the daemons.json path
    let listed = listed
        .err()
        .ok_or("listing an unreadable file must fail")?
        .to_string();
    assert!(listed.contains("daemons.json"), "{listed}");
    let saved = saved
        .err()
        .ok_or("saving over an unreadable file must fail")?;
    assert!(saved.contains("daemons.json"), "{saved}");

    // @step And daemons.json is left as it was
    assert!(path.join("keep").is_dir());
    Ok(())
}
