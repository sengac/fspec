//! Feature: spec/features/manifest-diagnostics-facade.feature
use codelet_core::persistence::{self, SessionStore};
use codelet_test_helpers::setup_temp_directory;
use serial_test::serial;
use std::{collections::BTreeMap, fs};
use uuid::Uuid;
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
#[serial]
fn manifest_diagnostics_are_stable_and_non_destructive() -> TestResult {
    // @step Given broken and healthy manifest fixtures
    let dir = setup_temp_directory()?;
    codelet_common::set_data_directory(dir.path().to_owned())?;
    persistence::reset_stores_for_tests();
    let healthy = persistence::create_session("healthy", dir.path())?;
    let healthy_path = dir
        .path()
        .join("sessions")
        .join(format!("{}.json", healthy.id));
    let mut originals = BTreeMap::from([(
        healthy_path,
        fs::read(
            dir.path()
                .join("sessions")
                .join(format!("{}.json", healthy.id)),
        )?,
    )]);
    let mut broken_ids = Vec::new();
    for bytes in [b"".as_slice(), b" \n\t", b"{", b"{}"] {
        let id = Uuid::new_v4();
        let path = dir.path().join("sessions").join(format!("{id}.json"));
        fs::write(&path, bytes)?;
        originals.insert(path, bytes.to_vec());
        broken_ids.push(id);
    }
    // @step When the exported diagnostic API is called twice
    let issues = persistence::session_manifest_issues()?;
    assert_eq!(issues, persistence::session_manifest_issues()?);
    // @step Then stable issues include paths identities categories recovery guidance and files stay unchanged
    assert_eq!(issues.len(), 4);
    assert_eq!(
        issues
            .iter()
            .filter(|issue| issue.category == "empty")
            .count(),
        2
    );
    for issue in &issues {
        let id = issue
            .session_id
            .as_deref()
            .ok_or("missing identity")?
            .parse::<Uuid>()?;
        assert!(broken_ids.contains(&id));
        assert!(originals.contains_key(&issue.path));
        assert!(!issue.category.is_empty() && !issue.reason.is_empty());
        assert!(issue.recovery.to_lowercase().contains("backup"));
    }
    for (path, bytes) in &originals {
        assert_eq!(fs::read(path)?, *bytes);
    }
    persistence::reset_stores_for_tests();
    let mut store = SessionStore::new()?;
    assert_eq!(store.list_all().len(), 1);
    assert_eq!(store.load(healthy.id)?.name, "healthy");
    assert!(store.load(broken_ids[0]).is_err());
    let created = store.create("independent new session", dir.path())?;
    assert_ne!(created.id, healthy.id);
    assert_eq!(persistence::session_manifest_issues()?, issues);
    Ok(())
}

#[test]
#[serial]
fn store_wide_diagnostic_failures_remain_errors() -> TestResult {
    // @step Given an invalid sessions root
    let dir = setup_temp_directory()?;
    codelet_common::set_data_directory(dir.path().to_owned())?;
    persistence::reset_stores_for_tests();
    let path = dir.path().join("sessions");
    fs::write(&path, b"not a directory")?;
    // @step When the exported diagnostic API is called
    let result = persistence::session_manifest_issues();
    // @step Then a store-wide error is returned instead of an empty success
    let error = result.expect_err("invalid root must remain an error");
    assert!(error.contains("sessions"));
    assert_eq!(fs::read(path)?, b"not a directory");
    Ok(())
}
