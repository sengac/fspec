//! Feature: spec/features/indexing-lock-diagnostics.feature
//! Post-fix controls; original diagnostic sources and red logs are archived in task validation.
#[path = "fixtures/indexing_lock_diagnostics.rs"]
mod locks;
#[path = "fixtures/indexing_diagnostic_support.rs"]
mod support;

use codelet_core::persistence::messages::{durability, MessageStore};
use serial_test::serial;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::time::Instant;
use support::{observed, Fixture, TestResult};
use uuid::Uuid;

#[test]
#[serial]
fn valid_index_reopens_and_warm_reads_do_not_republish_index() -> TestResult {
    // @step Given a valid synthetic compatible index
    let fixture = Fixture::new(3, 64)?;
    let original = fs::read(fixture.path("messages.idx"))?;
    // @step When stores reopen and read cached messages
    for _ in 0..5 {
        let (store, writes) = observed(MessageStore::new);
        // @step Then no index publication occurs
        let store = store?;
        assert_eq!(writes, 0);
        let (_, writes) = observed(|| {
            for id in &fixture.ids {
                assert!(store.get(*id).is_some());
            }
        });
        assert_eq!(writes, 0);
    }
    assert_eq!(fs::read(fixture.path("messages.idx"))?, original);
    Ok(())
}

#[test]
#[serial]
fn unknown_layout_rebuild_failure_preserves_index() -> TestResult {
    // @step Given a synthetic index with an unknown layout
    let fixture = Fixture::new(3, 64)?;
    let mut incompatible = fs::read(fixture.path("messages.idx"))?;
    incompatible[4..8].copy_from_slice(&99_u32.to_le_bytes());
    fs::write(fixture.path("messages.idx"), incompatible)?;
    let original = fs::read(fixture.path("messages.idx"))?;
    // @step When the current reader opens it with an index-publication failure control
    let (refused, _) = durability::observe(Some("index.write"), MessageStore::new);
    // @step Then the control fails without replacing the incompatible index
    assert!(
        refused.is_err(),
        "unknown layout did not reach real index publication"
    );
    assert_eq!(fs::read(fixture.path("messages.idx"))?, original);
    // @step And a successful open republishes a compatible index once
    let (store, writes) = observed(MessageStore::new);
    let store = store?;
    assert_eq!(writes, 1);
    assert_eq!(fixture.version()?, 1);
    for id in fixture.ids {
        assert!(store.get(id).is_some());
    }
    Ok(())
}

#[test]
#[serial]
fn alternating_historical_and_current_readers_share_the_index() -> TestResult {
    // @step Given synthetic V1 and V2 readers sharing one fixture
    // @step When they alternate four times
    let (legacy, current) = support::mixed_versions(4)?;
    // @step Then neither version republishes the unchanged index
    assert_eq!((legacy, current), (0, 0));
    println!("MIXED_VERSION_REBUILDS legacy={legacy} current={current}; same-version control=0");
    Ok(())
}

#[test]
#[serial]
fn red_witness_mixed_versions_should_not_rebuild_repeatedly() -> TestResult {
    let (_, rebuilds) = support::mixed_versions(3)?;
    assert_eq!(
        rebuilds, 0,
        "mixed-format opens repeatedly scan and republish the index"
    );
    Ok(())
}

#[test]
#[serial]
fn peer_append_invalidates_cache_but_valid_v2_index_avoids_rescan() -> TestResult {
    // @step Given a reader with cached messages
    let fixture = Fixture::new(2, 64)?;
    let reader = MessageStore::new()?;
    for id in &fixture.ids {
        assert!(reader.get(*id).is_some());
    }
    assert_eq!(reader.cache_len(), 2);
    // @step When a V2 peer appends a committed message
    let writer = MessageStore::new()?;
    let added = writer.store("user", "synthetic peer append")?;
    // @step Then the reader invalidates its cache and uses the saved index
    let (message, writes) = observed(|| reader.get(added));
    assert!(message.is_some());
    assert_eq!(writes, 0);
    assert_eq!(
        reader.cache_len(),
        1,
        "generation refresh did not invalidate old cache"
    );
    Ok(())
}

#[test]
#[serial]
fn verified_absence_survives_matching_peer_generations() -> TestResult {
    // @step Given a valid index and an absent message identity
    let _fixture = Fixture::new(2, 64)?;
    let reader = MessageStore::new()?;
    let missing = Uuid::new_v4();
    // @step When a reader repeats lookup and a peer then advances the generation
    let (message, first) = observed(|| reader.get(missing));
    assert!(message.is_none());
    // @step Then verified absence does not republish the index
    assert_eq!(first, 0);
    let (_, repeated) = observed(|| {
        for _ in 0..8 {
            assert!(reader.get(missing).is_none());
        }
    });
    // @step And repeated misses in one generation do not
    assert_eq!(repeated, 0);
    let writer = MessageStore::new()?;
    writer.store("user", "new generation")?;
    let (message, next_generation) = observed(|| reader.get(missing));
    assert!(message.is_none());
    assert_eq!(next_generation, 0);
    println!("MISS_REBUILDS initial={first} repeated={repeated} next_generation={next_generation}");
    Ok(())
}

#[test]
#[serial]
fn stale_size_scans_history_once_and_retains_the_original_bytes() -> TestResult {
    // @step Given a synthetic index with a stale recorded size
    let fixture = Fixture::new(2, 64)?;
    let reader = MessageStore::new()?;
    let mut changed = fs::read(fixture.path("messages.idx"))?;
    changed[8..16].copy_from_slice(&0_u64.to_le_bytes());
    fs::write(fixture.path("messages.idx"), changed)?;
    let original = fs::read(fixture.path("messages.jsonl"))?;
    // @step When an existing original is requested
    let (message, writes) = observed(|| reader.get(fixture.ids[0]));
    // @step Then the index is republished once without changing message bytes
    assert!(message.is_some());
    assert_eq!(writes, 1);
    assert_eq!(fs::read(fixture.path("messages.jsonl"))?, original);
    Ok(())
}

#[test]
#[serial]
fn each_append_republishes_the_full_growing_index() -> TestResult {
    // @step Given a synthetic message store
    let fixture = Fixture::new(1, 64)?;
    let store = MessageStore::new()?;
    // @step When fixed-size messages are appended
    for number in 2..=9 {
        let (result, writes) = observed(|| store.store("user", "fixed synthetic payload"));
        result?;
        // @step Then each append publishes an index containing every current entry
        let expected = 1;
        assert_eq!(
            writes, expected,
            "verified identity admission publishes once"
        );
        assert_eq!(
            fs::metadata(fixture.path("messages.idx"))?.len(),
            20 + number * 28
        );
    }
    Ok(())
}

#[test]
#[serial]
fn current_rebuild_validates_content_that_historical_uuid_scan_skips() -> TestResult {
    // @step Given a synthetic message with an invalid content hash
    let fixture = Fixture::new(1, 64)?;
    let extra = serde_json::json!({"id":Uuid::new_v4(),"content_hash":"invalid-hash",
        "created_at":"2026-10-27T00:00:00Z","role":"user","content":"synthetic corruption",
        "token_count":null,"blob_refs":[],"metadata":{}});
    let mut file = OpenOptions::new()
        .append(true)
        .open(fixture.path("messages.jsonl"))?;
    writeln!(file, "{extra}")?;
    // @step When historical and current scanners inspect it
    // @step Then historical UUID scanning accepts its identity
    assert_eq!(support::legacy_scan(fixture.directory.path())?, 2);
    // @step And the current rebuild rejects the content hash
    let error = MessageStore::new()
        .err()
        .ok_or("corrupt full scan unexpectedly accepted")?;
    assert!(error.contains("content hash mismatch"), "{error}");
    Ok(())
}

#[test]
#[serial]
fn bounded_fixture_timings_compare_uuid_scan_with_verified_rebuild() -> TestResult {
    // @step Given a small synthetic history
    let fixture = Fixture::new(32, 4096)?;
    // @step When UUID scanning and verified rebuilding run
    let started = Instant::now();
    assert_eq!(support::legacy_scan(fixture.directory.path())?, 32);
    let uuid_scan = started.elapsed();
    fs::remove_file(fixture.path("messages.idx"))?;
    let started = Instant::now();
    let (store, publications) = observed(MessageStore::new);
    assert_eq!(store?.index_len(), 32);
    assert_eq!(publications, 1);
    let verified_rebuild = started.elapsed();
    // @step Then durations are reported without a speed-ratio assertion
    println!("SYNTHETIC_TIMING bytes={} uuid_scan_us={} verified_rebuild_us={}; different work: rebuild includes hash validation and durable publication", fs::metadata(fixture.path("messages.jsonl"))?.len(), uuid_scan.as_micros(), verified_rebuild.as_micros());
    Ok(())
}

#[test]
#[serial]
fn same_version_peer_writes_reuse_verified_completeness() -> TestResult {
    // @step Given two V2 stores with a valid shared synthetic index
    // @step When they alternate appending fresh message identities
    let (publications, warm) = support::peer_write_publications()?;
    // @step Then each write publishes only the append index
    assert_eq!(publications, vec![1, 1, 1, 1]);
    // @step And a consecutive write by the warm writer publishes only the append index
    assert_eq!(warm, 1);
    println!("SAME_V2_PUBLICATIONS alternating={publications:?} warm_single_writer={warm}; each alternating write reuses verified completeness");
    Ok(())
}

#[test]
#[serial]
fn red_witness_same_version_peers_should_avoid_recovery_before_new_appends() -> TestResult {
    let (publications, _) = support::peer_write_publications()?;
    assert_eq!(
        publications,
        vec![1, 1, 1, 1],
        "same-format peer writes rescan history before appending a fresh identity"
    );
    Ok(())
}
