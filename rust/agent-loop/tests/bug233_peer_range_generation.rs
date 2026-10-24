//! Feature: spec/features/compaction-peer-range-recovery.feature

#[path = "cmpct056/support.rs"]
mod durable;
#[path = "support/dag_generation_recovery.rs"]
mod recovery;

use codelet_core::compaction::{validate_dag_candidate, DagValidationContext};
use proptest::prelude::*;

#[tokio::test]
#[serial_test::serial]
async fn scenario_generation_explains_inclusive_peer_bounds_and_promotion_conflicts() {
    // @step Given a live session with persisted originals
    let server = wiremock::MockServer::start().await;
    let fixture = durable::Fixture::new(&server.uri()).await;
    // @step When fresh and incremental compaction requests reach the provider
    for incremental in [false, true] {
        let outcome = recovery::drive(&fixture, &server, vec![recovery::hierarchy()]).await;
        assert!(outcome.result.is_ok(), "{:?}", outcome.result);
        // @step Then both requests distinguish same-depth conflicts from valid cross-depth overlap
        let prompt = &outcome.prompts[0];
        assert!(prompt.contains("Same-depth peers must be disjoint"));
        assert!(prompt.contains("0-10 and 10-20 overlap"));
        assert!(prompt.contains("0-10 and 11-20 are disjoint"));
        assert!(prompt.contains("Cross-depth overlap is valid"));
        // @step And incremental instructions require checking conflicts after depth promotion
        if incremental {
            assert!(prompt.contains("Recheck same-depth conflicts after promotion"));
        }
    }
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_conflicting_peer_ranges_receive_precise_bounded_correction() {
    // @step Given generated duplicate intersecting shared-endpoint or unsorted same-depth ranges
    let conflicts = [(0, 1, 0, 1), (0, 1, 1, 1), (0, 0, 0, 1), (1, 1, 0, 1)];
    for (first_start, first_end, second_start, second_end) in conflicts {
        let candidate = format!(
            "{}{}",
            recovery::node("D1", first_start, first_end),
            recovery::node("D1", second_start, second_end)
        );
        let server = wiremock::MockServer::start().await;
        let fixture = durable::Fixture::new(&server.uri()).await;
        let replacement = recovery::hierarchy();
        // @step When the production builder receives a valid hierarchical replacement
        let outcome =
            recovery::drive(&fixture, &server, vec![candidate, replacement.clone()]).await;
        assert!(outcome.result.is_ok(), "{:?}", outcome.result);
        assert_eq!(outcome.prompts.len(), 2);
        // @step Then correction names the conflicting depth and both inclusive ranges
        let correction = &outcome.prompts[1];
        assert!(
            correction.contains("overlapping same-depth dag-node ranges: depth=D1"),
            "{correction}"
        );
        assert!(correction.contains(&format!("{first_start}-{first_end}")));
        assert!(correction.contains(&format!("{second_start}-{second_end}")));
        assert!(correction.contains("Same-depth peers must be disjoint"));
        let warning = outcome
            .logs
            .lines()
            .find(|line| line.contains("compaction builder output requires correction"))
            .expect("correction diagnostic");
        assert!(
            warning.contains(&format!("target_session_id={}", fixture.caller.id)),
            "{warning}"
        );
        assert!(
            warning.contains("depth=D1") && warning.contains("operation="),
            "{warning}"
        );
        // @step And only the complete replacement with disjoint peers and cross-depth nesting is committed
        assert_eq!(outcome.summary, Some(replacement));
        assert!(!outcome.blocked);
    }
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_exhausted_peer_conflicts_never_promote_partial_memory() {
    // @step Given every generated candidate contains conflicting peer ranges
    let server = wiremock::MockServer::start().await;
    let fixture = durable::Fixture::new(&server.uri()).await;
    let conflict = format!(
        "{}{}",
        recovery::node("D1", 0, 1),
        recovery::node("D1", 1, 1)
    );
    // @step When the production summary handoff exhausts three candidate attempts
    let outcome = recovery::drive(&fixture, &server, vec![conflict; 3]).await;
    assert_eq!(outcome.prompts.len(), 3);
    // @step Then failure identifies the conflicting depth ranges target operation and attempt
    let error = outcome.result.expect_err("conflicts must fail");
    assert!(error.contains("depth=D1"), "{error}");
    assert!(error.contains("0-1") && error.contains("1-1"), "{error}");
    assert!(error.contains(&fixture.caller.id.to_string()), "{error}");
    assert!(error.contains("operation="), "{error}");
    assert!(error.contains("after 3 attempts"), "{error}");
    // @step And durable originals and committed memory remain unchanged with ordinary inference blocked
    assert_eq!(outcome.before, outcome.after);
    assert!(outcome.blocked);
}

proptest! {
    #[test]
    fn scenario_arbitrary_inclusive_peers_preserve_strict_hierarchy_validation(
        // @step Given arbitrary persisted inclusive range endpoints
        midpoint in 2usize..1000, tail in 1usize..1000, reverse in any::<bool>()
    ) {
        let end = midpoint + tail;
        let context = DagValidationContext {
            persisted_turn_count: end + 1,
            required_coverage_end: Some(end),
            ..Default::default()
        };
        let first = recovery::node("D1", 0, midpoint);
        let last = recovery::node("D1", midpoint + 1, end);
        let peers = if reverse { format!("{last}{first}") } else { format!("{first}{last}") };
        // @step When whole candidates contain disjoint peers nested depths partial intersections or shared endpoints
        let nested = format!("{}{}", recovery::node("D2", 0, end), peers);
        let shared = format!("{}{}", recovery::node("D1", midpoint, end), first);
        let partial = format!("{}{}", recovery::node("D1", midpoint - 1, end), first);
        // @step Then disjoint peers and cross-depth nesting are accepted unchanged while shared endpoints and partial intersections are rejected
        prop_assert_eq!(validate_dag_candidate(&peers, &context).expect("disjoint peers").content, peers);
        prop_assert_eq!(validate_dag_candidate(&nested, &context).expect("nested hierarchy").content, nested);
        let error = validate_dag_candidate(&shared, &context).expect_err("inclusive endpoint conflict").to_string();
        prop_assert!(error.contains("depth=D1"), "{}", error);
        prop_assert!(error.contains(&format!("0-{midpoint}")), "{}", error);
        prop_assert!(error.contains(&format!("{midpoint}-{end}")), "{}", error);
        let error = validate_dag_candidate(&partial, &context).expect_err("partial peer intersection").to_string();
        prop_assert!(error.contains("depth=D1"), "{}", error);
        prop_assert!(error.contains(&format!("0-{midpoint}")), "{}", error);
        prop_assert!(error.contains(&format!("{}-{end}", midpoint - 1)), "{}", error);
    }
}
