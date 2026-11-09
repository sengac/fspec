// Feature: spec/features/migrate-session-message-persistence-from-typescript-to-rust.feature
//
// Session Message Persistence Tests (REFAC-007)
//
// These tests verify that Rust correctly persists message envelopes for all
// message types (user, assistant, tool_result) and that sessions can be
// resumed with complete message history.
//
// These tests share global state (codelet_core::persistence stores); every
// test takes the `setup_test_env` mutex guard so they run serially.
//
// MESSAGE PERSISTENCE SCENARIOS:
// - User message is persisted by Rust when prompt is received
// - Assistant message with tool_use is persisted before tool execution
// - Tool result is persisted by Rust when tool execution completes
// - Final assistant response is persisted after tool result
// - Multiple tool uses in single assistant response are all persisted
// - Sequential tool calls across multiple turns are persisted
// - Resumed session contains all messages including final responses
//
// TOKEN/COMPACTION STATE SCENARIOS:
// - Token state is persisted and restored accurately
// - Compaction state is persisted and restored
// - Resumed session restores compaction summary as first message
//
// ERROR HANDLING SCENARIOS:
// - Persistence failure propagates error to user
// - Invalid session operations fail gracefully
// - API error mid-stream persists accumulated content
// - User interrupt preserves accumulated content

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_core::persistence::{
    append_message, append_message_with_metadata, create_session, get_session_messages,
    load_session, set_compaction_state, update_session_tokens,
};
use fixtures::{
    add_alternating_messages, add_simple_alternating_messages, assert_content_block_count,
    assert_content_block_type, count_content_blocks_of_type, create_assistant_multi_tool_envelope,
    create_assistant_text_envelope, create_assistant_tool_use_envelope,
    create_tool_result_envelope, create_user_envelope,
};
use persistence_env::setup_test_env;
use std::path::PathBuf;

#[path = "session_persistence_test/error_handling.rs"]
mod error_handling;
#[path = "session_persistence_test/manifest_integrity.rs"]
mod manifest_integrity;
#[path = "session_persistence_test/message_persistence.rs"]
mod message_persistence;
#[path = "session_persistence_test/ordering_and_boundaries.rs"]
mod ordering_and_boundaries;
#[path = "session_persistence_test/resume_and_tool_calls.rs"]
mod resume_and_tool_calls;
#[path = "session_persistence_test/token_and_compaction.rs"]
mod token_and_compaction;

#[path = "support/persistence_env.rs"]
mod persistence_env;

#[path = "session_persistence_test/fixtures.rs"]
mod fixtures;
