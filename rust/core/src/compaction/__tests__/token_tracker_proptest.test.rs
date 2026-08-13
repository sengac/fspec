// FV-002 Property-Based Tests — `TokenTracker` (rust/core/src/compaction/model.rs)
//
// Cross-checks the Alloy model `token_tracker.als` against the real Rust
// implementation. Generates random sequences of `update_from_usage` and
// `reset_after_compaction` calls and asserts every documented invariant
// holds on every intermediate state.
//
// Cross-reference: rust/core/spec/compaction/token_tracker.als

use crate::compaction::model::TokenTracker;
use crate::token_usage::ApiTokenUsage;
use proptest::prelude::*;

// ────────────────────────────────────────────────────────────────────────────
// Generators
// ────────────────────────────────────────────────────────────────────────────

/// A randomly-generated `ApiTokenUsage`.
///
/// Bounded to avoid u64 overflow when accumulating across many operations.
fn arb_usage() -> impl Strategy<Value = ApiTokenUsage> {
    (
        0u64..1_000_000,  // input
        0u64..1_000_000,  // cache_read
        0u64..1_000_000,  // cache_creation
        0u64..1_000_000,  // output
        0u64..1_000_000,  // reasoning
    )
        .prop_map(|(input, cr, cc, output, reasoning)| {
            ApiTokenUsage::new(input, cr, cc, output).with_reasoning_tokens(reasoning)
        })
}

/// A sequence of operations the tracker can be subjected to.
#[derive(Debug, Clone)]
enum Op {
    /// Apply update_from_usage with the given usage and a cumulative-output
    /// delta added to the previous outputTokens.
    Update {
        usage: ApiTokenUsage,
        cumulative_delta: u64,
    },
    /// Apply reset_after_compaction.
    Compact,
}

fn arb_op() -> impl Strategy<Value = Op> {
    prop_oneof![
        (arb_usage(), 0u64..2_000_000)
            .prop_map(|(usage, cumulative_delta)| Op::Update { usage, cumulative_delta }),
        Just(Op::Compact),
    ]
}

fn arb_op_sequence() -> impl Strategy<Value = Vec<Op>> {
    prop::collection::vec(arb_op(), 0..15)
}

// ────────────────────────────────────────────────────────────────────────────
// Apply an operation, tracking what the property tests need to verify.
// ────────────────────────────────────────────────────────────────────────────

fn apply(tracker: &mut TokenTracker, op: &Op) {
    match op {
        Op::Update { usage, cumulative_delta } => {
            // Simulate the streaming display reporting an ever-growing
            // cumulative output. We add `cumulative_delta` to whatever was
            // there before so the running total only grows.
            let cumulative = tracker
                .output_tokens
                .saturating_add(*cumulative_delta);
            tracker.update_from_usage(usage, cumulative);
        }
        Op::Compact => {
            tracker.reset_after_compaction();
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Property tests — Alloy invariants checked on the real implementation
// ────────────────────────────────────────────────────────────────────────────

proptest! {
    /// INV-2 (Alloy: CumulativeBilledInputMonotonic).
    ///
    /// Across any execution trace, cumulative_billed_input only grows.
    #[test]
    fn cumulative_billed_input_monotonic(ops in arb_op_sequence()) {
        let mut tracker = TokenTracker::new();
        let mut prev = tracker.cumulative_billed_input;
        for op in &ops {
            apply(&mut tracker, op);
            prop_assert!(
                tracker.cumulative_billed_input >= prev,
                "cumulative_billed_input regressed: {} -> {}",
                prev, tracker.cumulative_billed_input
            );
            prev = tracker.cumulative_billed_input;
        }
    }

    /// INV-3 (Alloy: CumulativeBilledOutputMonotonic).
    #[test]
    fn cumulative_billed_output_monotonic(ops in arb_op_sequence()) {
        let mut tracker = TokenTracker::new();
        let mut prev = tracker.cumulative_billed_output;
        for op in &ops {
            apply(&mut tracker, op);
            prop_assert!(
                tracker.cumulative_billed_output >= prev,
                "cumulative_billed_output regressed: {} -> {}",
                prev, tracker.cumulative_billed_output
            );
            prev = tracker.cumulative_billed_output;
        }
    }

    /// INV-4 (Alloy: InputTokensAbsolute).
    ///
    /// After update_from_usage, input_tokens equals THIS request's
    /// total_input — not a sum across requests.
    #[test]
    fn input_tokens_absolute_after_update(usage in arb_usage(), cumulative in 0u64..2_000_000) {
        let mut tracker = TokenTracker::new();
        // Pre-load with some history so we can detect accidental accumulation.
        let prelude = ApiTokenUsage::new(7_000, 3_000, 2_000, 500);
        tracker.update_from_usage(&prelude, 500);

        tracker.update_from_usage(&usage, cumulative);

        prop_assert_eq!(tracker.input_tokens, usage.total_input());
    }

    /// INV-5 (Alloy: CompactionPreservesCumulativeBilling).
    #[test]
    fn compaction_preserves_cumulative_billing(ops in arb_op_sequence()) {
        let mut tracker = TokenTracker::new();
        for op in &ops {
            apply(&mut tracker, op);
        }
        let before_input = tracker.cumulative_billed_input;
        let before_output = tracker.cumulative_billed_output;

        tracker.reset_after_compaction();

        prop_assert_eq!(tracker.cumulative_billed_input, before_input);
        prop_assert_eq!(tracker.cumulative_billed_output, before_output);
    }

    /// INV-6 (Alloy: CompactionClearsTransientState).
    #[test]
    fn compaction_clears_transient_state(ops in arb_op_sequence()) {
        let mut tracker = TokenTracker::new();
        for op in &ops {
            apply(&mut tracker, op);
        }

        tracker.reset_after_compaction();

        prop_assert_eq!(tracker.output_tokens, 0);
        prop_assert_eq!(tracker.reasoning_tokens, 0);
        prop_assert!(tracker.cache_read_input_tokens.is_none());
        prop_assert!(tracker.cache_creation_input_tokens.is_none());
    }

    /// INV-7 (Alloy: CumulativeBilledLowerBound).
    ///
    /// Each update_from_usage increases cumulative_billed_input by AT LEAST
    /// usage.input_tokens (the raw fresh-input portion — not total_input,
    /// because billing only counts fresh tokens).
    #[test]
    fn update_increases_cumulative_billed_input_by_at_least_input(
        usage in arb_usage(),
        cumulative in 0u64..2_000_000,
    ) {
        let mut tracker = TokenTracker::new();
        let before = tracker.cumulative_billed_input;
        tracker.update_from_usage(&usage, cumulative);
        prop_assert!(
            tracker.cumulative_billed_input >= before.saturating_add(usage.input_tokens)
        );
        // And exactly equal in the no-compaction case (stronger):
        prop_assert_eq!(
            tracker.cumulative_billed_input,
            before.saturating_add(usage.input_tokens)
        );
    }

    /// INV-1 / total_input identity (Alloy: TotalInputIdentity).
    ///
    /// `total_input() == input + cache_read + cache_creation`.
    #[test]
    fn total_input_identity(usage in arb_usage()) {
        prop_assert_eq!(
            usage.total_input(),
            usage.input_tokens + usage.cache_read_input_tokens + usage.cache_creation_input_tokens
        );
    }

    /// Bonus invariant: total_context = total_input + output + reasoning.
    #[test]
    fn total_context_identity(usage in arb_usage()) {
        prop_assert_eq!(
            usage.total_context(),
            usage.total_input() + usage.output_tokens + usage.reasoning_tokens
        );
    }

    /// Bonus invariant: effective_tokens never exceeds input_tokens.
    ///
    /// This is the cache-discount calculation in `effective_tokens()`. It
    /// uses saturating_sub, so it's bounded below by 0 and above by
    /// input_tokens.
    #[test]
    fn effective_tokens_bounded(input in 0u64..1_000_000, cache_read in 0u64..1_000_000) {
        let mut tracker = TokenTracker::new();
        tracker.input_tokens = input;
        tracker.cache_read_input_tokens = Some(cache_read);
        let eff = tracker.effective_tokens();
        prop_assert!(eff <= input);
    }
}
