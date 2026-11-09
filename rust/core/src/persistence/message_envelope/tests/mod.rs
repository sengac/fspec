#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

// Note: the original `test_blob_threshold` test referencing
// `crate::persistence::should_use_blob_storage` is now covered by
// `crate::persistence::blob::tests::test_should_use_blob_storage`
// (and `test_blob_threshold_boundary_is_strict_greater_than`),
// since RPC-034 lifted `should_use_blob_storage` from
// `napi::persistence::blob` into the sibling `blob` module.

// =========================================================================
// Extended Message Envelope Tests (NAPI-008)
// =========================================================================

// =========================================================================
// PROV-039: stop_reason persistence must use real value, not hardcoded end_turn
// Feature: spec/features/stop-reason-lost-in-streaming-output-truncation-silently-treated-as-normal-completion.feature
// =========================================================================

mod part_1;
mod part_2;
mod part_3;
