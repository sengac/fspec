//! TEST-042 — fspec's own test code never waits on a wall-clock sleep or a
//! fixed timeout; the only timed wait is the deadlock guard.
//!
//! Feature: spec/features/fspec-tests-never-wait-on-wall-clock-sleeps-or-fixed-timeouts.feature

use std::path::Path;

use codelet_test_helpers::wall_clock_waits::{
    crate_wall_clock_waits, scan_src_source, scan_test_source,
};
use codelet_test_helpers::TestResult;

fn calls(found: &[(usize, String)]) -> Vec<(usize, &str)> {
    found
        .iter()
        .map(|(line, call)| (*line, call.as_str()))
        .collect()
}

#[test]
fn a_tokio_sleep_in_a_test_file_is_reported() -> TestResult {
    // @step Given a tests/ file that calls tokio::time::sleep before reading a state
    let source = r#"
#[tokio::test]
async fn reads_state() {
    start();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert!(ready());
}
"#;
    // @step When the wall-clock wait check scans it
    let found = scan_test_source(source)?;
    // @step Then it reports the file, the line and "tokio::time::sleep"
    assert_eq!(calls(&found), vec![(5, "tokio::time::sleep")]);
    Ok(())
}

#[test]
fn an_imported_timeout_inside_a_macro_is_reported() -> TestResult {
    // @step Given a tests/ file with "use tokio::time::timeout;" that calls timeout inside assert!
    let source = r#"
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn replies() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    tx.send(1).await.unwrap();
    assert!(timeout(Duration::from_secs(5), rx.recv()).await.is_ok());
    let timeout = Duration::from_secs(1);
    let _ = timeout;
}
"#;
    // @step When the wall-clock wait check scans it
    let found = scan_test_source(source)?;
    // @step Then it reports that line and "tokio::time::timeout"
    assert_eq!(calls(&found), vec![(9, "tokio::time::timeout")]);
    let aliased =
        "use tokio::time;\nuse std::thread::sleep as nap;\nfn f() { time::sleep(D); nap(D); }\n";
    assert_eq!(
        calls(&scan_test_source(aliased)?),
        vec![(3, "tokio::time::sleep"), (3, "std::thread::sleep")]
    );
    let globbed = "mod m {\n    use tokio::time::*;\n    async fn f() { interval(D); }\n}\n";
    assert_eq!(
        calls(&scan_test_source(globbed)?),
        vec![(3, "tokio::time::interval")]
    );
    Ok(())
}

#[test]
fn only_the_test_code_of_a_src_file_is_scanned() -> TestResult {
    // @step Given a src file that calls std::thread::sleep in production code and in a #[cfg(test)] module
    let source = r#"
pub fn backoff() {
    std::thread::sleep(std::time::Duration::from_millis(10));
}

#[cfg(test)]
mod tests {
    #[test]
    fn waits() {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[cfg(all(test, feature = "slow"))]
fn gated() {
    std::thread::sleep(std::time::Duration::from_millis(10));
}

#[cfg(not(test))]
fn production_only() {
    std::thread::sleep(std::time::Duration::from_millis(10));
}
"#;
    // @step When the wall-clock wait check scans it
    let found = scan_src_source(source)?;
    // @step Then it reports only the call in the #[cfg(test)] module
    assert_eq!(
        calls(&found),
        vec![(10, "std::thread::sleep"), (16, "std::thread::sleep")]
    );
    Ok(())
}

#[test]
fn a_tests_own_function_named_timeout_is_not_reported() -> TestResult {
    // @step Given a tests/ file that defines and calls its own fn timeout without importing tokio::time
    let source = r#"
fn timeout(limit: u64) -> u64 { limit * 2 }
fn sleep() {}

#[test]
fn doubles() {
    assert_eq!(timeout(2), 4);
    sleep();
    let client = Client::builder().timeout(std::time::Duration::from_secs(1));
}
"#;
    // @step When the wall-clock wait check scans it
    let found = scan_test_source(source)?;
    // @step Then it reports nothing
    assert_eq!(calls(&found), Vec::<(usize, &str)>::new());
    Ok(())
}

#[test]
fn timed_wait_methods_are_reported() -> TestResult {
    // @step Given a tests/ file that calls recv_timeout on a std channel and wait_timeout on a condvar
    let source = r#"
#[test]
fn waits() {
    let (_tx, rx) = std::sync::mpsc::channel::<()>();
    let _ = rx.recv_timeout(std::time::Duration::from_secs(1));
    let (lock, condvar) = &*pair;
    let _ = condvar.wait_timeout(lock.lock().unwrap(), std::time::Duration::from_secs(1));
}
"#;
    // @step When the wall-clock wait check scans it
    let found = scan_test_source(source)?;
    // @step Then it reports "recv_timeout" and "wait_timeout" with their lines
    assert_eq!(
        calls(&found),
        vec![(5, "recv_timeout"), (7, "wait_timeout")]
    );
    Ok(())
}

#[test]
fn a_paused_clock_test_may_use_tokio_time() -> TestResult {
    // @step Given a tests/ file whose #[tokio::test(start_paused = true)] function calls tokio::time::timeout and tokio::time::sleep
    // @step And a plain #[tokio::test] function in the same file that calls tokio::time::sleep
    let source = r#"
#[tokio::test(start_paused = true)]
async fn virtual_time() {
    let _ = tokio::time::timeout(std::time::Duration::from_millis(50), pending()).await;
    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
}

#[tokio::test]
async fn wall_clock() {
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
}
"#;
    // @step When the wall-clock wait check scans it
    let found = scan_test_source(source)?;
    // @step Then it reports only the call in the plain test
    assert_eq!(calls(&found), vec![(10, "tokio::time::sleep")]);
    Ok(())
}

#[test]
#[ignore = "TEST-042: enabled once remaining-wall-clock-waits.md is empty"]
fn fspecs_own_test_code_waits_on_no_wall_clock_sleep_or_fixed_timeout() -> TestResult {
    // @step Given every workspace member's tests/ tree and the test code of its src files
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let manifest: toml::Value =
        toml::from_str(&std::fs::read_to_string(workspace.join("Cargo.toml"))?)?;
    let members: Vec<String> = manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("members"))
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(toml::Value::as_str)
        .map(str::to_string)
        .collect();
    assert!(
        members.len() > 10,
        "the workspace lists its members: {members:?}"
    );
    // @step When the wall-clock wait check scans them
    let mut found = Vec::new();
    for member in &members {
        let waits = crate_wall_clock_waits(&workspace.join(member))?;
        found.extend(waits.into_iter().map(|wait| {
            let file = wait
                .file
                .strip_prefix(&workspace)
                .unwrap_or(&wait.file)
                .to_path_buf();
            format!("{}:{}: {}", file.display(), wait.line, wait.call)
        }));
    }
    // @step Then it reports nothing
    assert!(
        found.is_empty(),
        "{} wall-clock waits in fspec's test code (wait on the event or state instead; the only \
         timed wait is codelet_test_helpers::deadlock_guard):\n{}",
        found.len(),
        found.join("\n")
    );
    Ok(())
}

#[test]
fn a_comparison_with_a_clock_reading_is_reported() -> TestResult {
    // @step Given a tests/ file that compares started.elapsed(), Instant::now() and a let-bound elapsed reading with a bound
    // @step And the same file asserts an elapsed() equality and prints an elapsed() reading
    let source = r#"
use std::time::{Duration, Instant};

#[test]
fn bounded() {
    let started = Instant::now();
    run();
    assert!(started.elapsed() < Duration::from_millis(200));
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {}
    let waited = started.elapsed();
    assert!(waited >= limit);
    assert!(limit > started.elapsed().as_millis());
    assert_eq!(clock.elapsed(), Duration::from_secs(1));
    println!("{:?}", started.elapsed());
    let items: Vec<Instant> = Vec::new();
}
"#;
    // @step When the wall-clock wait check scans it
    let found = scan_test_source(source)?;
    // @step Then it reports each comparison with its line and the clock it reads
    // @step And it reports neither the equality nor the printed reading
    assert_eq!(
        calls(&found),
        vec![
            (8, "elapsed() comparison"),
            (10, "std::time::Instant::now() comparison"),
            (12, "elapsed() comparison"),
            (13, "elapsed() comparison"),
        ]
    );
    Ok(())
}

#[test]
fn a_support_modules_glob_import_from_its_parent_is_resolved() -> TestResult {
    // @step Given a tests/x.rs file with "use tokio::time::timeout;"
    let directory = tempfile::tempdir()?;
    let tests = directory.path().join("tests");
    std::fs::create_dir_all(tests.join("x"))?;
    std::fs::write(
        tests.join("x.rs"),
        "use std::time::Duration;\nuse tokio::time::timeout;\nmod support;\n",
    )?;
    // @step And a tests/x/support.rs file with "use super::*;" that calls timeout
    std::fs::write(
        tests.join("x").join("support.rs"),
        "use super::*;\n\npub async fn reply(rx: R) {\n    let _ = timeout(Duration::from_secs(1), rx.recv()).await;\n    let _ = super::timeout(Duration::from_secs(1), rx.recv()).await;\n}\n",
    )?;
    // @step When the wall-clock wait check scans the crate
    let found = crate_wall_clock_waits(directory.path())?;
    // @step Then it reports "tokio::time::timeout" in tests/x/support.rs
    let found: Vec<(String, usize, String)> = found
        .into_iter()
        .map(|wait| {
            let file = wait
                .file
                .strip_prefix(directory.path())
                .unwrap_or(&wait.file)
                .display()
                .to_string();
            (file, wait.line, wait.call)
        })
        .collect();
    assert_eq!(
        found,
        vec![
            (
                "tests/x/support.rs".to_string(),
                4,
                "tokio::time::timeout".to_string()
            ),
            (
                "tests/x/support.rs".to_string(),
                5,
                "tokio::time::timeout".to_string()
            ),
        ]
    );
    Ok(())
}

#[test]
fn a_paused_clock_test_still_may_not_read_or_sleep_on_the_wall_clock() -> TestResult {
    // @step Given a #[tokio::test(start_paused = true)] function that calls std::thread::sleep
    // @step And compares a std::time::Instant::now() reading and a tokio::time::Instant::now() reading with a bound
    let source = r#"
#[tokio::test(start_paused = true)]
async fn virtual_time() {
    std::thread::sleep(std::time::Duration::from_millis(5));
    let real = std::time::Instant::now();
    let fake = tokio::time::Instant::now();
    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
    assert!(real.elapsed() < std::time::Duration::from_secs(1));
    assert!(fake.elapsed() >= std::time::Duration::from_secs(30));
    assert!(std::time::Instant::now() < real_deadline);
    assert!(tokio::time::Instant::now() >= fake_deadline);
}
"#;
    // @step When the wall-clock wait check scans it
    let found = scan_test_source(source)?;
    // @step Then it reports "std::thread::sleep" and the std::time::Instant::now() comparison
    // @step And it does not report the tokio::time::Instant::now() comparison
    assert_eq!(
        calls(&found),
        vec![
            (4, "std::thread::sleep"),
            (8, "elapsed() comparison"),
            (10, "std::time::Instant::now() comparison"),
        ]
    );
    Ok(())
}

#[test]
fn an_included_part_resolves_names_through_the_file_that_includes_it() -> TestResult {
    // @step Given a tests/x.rs file with "use tokio::time::timeout;" that includes x/part.rs with include!
    let directory = tempfile::tempdir()?;
    let tests = directory.path().join("tests");
    std::fs::create_dir_all(tests.join("x"))?;
    std::fs::write(
        tests.join("x.rs"),
        "use std::time::Duration;\nuse tokio::time::timeout;\n\ninclude!(\"x/part.rs\");\n",
    )?;
    // @step And a tests/x/part.rs file with no use declarations that calls timeout
    std::fs::write(
        tests.join("x").join("part.rs"),
        "#[tokio::test]\nasync fn replies() {\n    let _ = timeout(Duration::from_secs(1), rx.recv()).await;\n}\n",
    )?;
    // @step When the wall-clock wait check scans the crate
    let found = crate_wall_clock_waits(directory.path())?;
    // @step Then it reports "tokio::time::timeout" in tests/x/part.rs
    let found: Vec<(String, usize, String)> = found
        .into_iter()
        .map(|wait| {
            let file = wait
                .file
                .strip_prefix(directory.path())
                .unwrap_or(&wait.file)
                .display()
                .to_string();
            (file, wait.line, wait.call)
        })
        .collect();
    assert_eq!(
        found,
        vec![(
            "tests/x/part.rs".to_string(),
            3,
            "tokio::time::timeout".to_string()
        )]
    );
    Ok(())
}
