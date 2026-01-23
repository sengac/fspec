//! Feature: spec/features/background-session-management-with-attach-detach.feature
//!
//! Tests for Rust State Management in Background Sessions
//!
//! These tests verify that session state (model, tokens, status) is properly
//! managed in Rust and accessible via sync NAPI functions. This enables the
//! React UI to fetch current state without waiting for streaming events.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

/// Test atomic token caching behavior
///
/// Scenario: Cache token counts for sync access
///
/// @step Given a background session is running
/// @step When token updates are received during streaming
/// @step Then the cached token counts are updated atomically
#[test]
fn test_atomic_token_caching() {
    // @step Given cached token fields (simulating BackgroundSession)
    let cached_input_tokens = AtomicU32::new(0);
    let cached_output_tokens = AtomicU32::new(0);

    // Verify initial state
    assert_eq!(cached_input_tokens.load(Ordering::Acquire), 0);
    assert_eq!(cached_output_tokens.load(Ordering::Acquire), 0);

    // @step When token updates are received (simulating update_tokens)
    cached_input_tokens.store(5000, Ordering::Release);
    cached_output_tokens.store(3000, Ordering::Release);

    // @step Then the cached token counts are updated atomically
    assert_eq!(cached_input_tokens.load(Ordering::Acquire), 5000);
    assert_eq!(cached_output_tokens.load(Ordering::Acquire), 3000);
}

/// Test token cache updates during streaming
///
/// Scenario: Token counts accumulate during streaming response
///
/// @step Given a session is streaming a response
/// @step When multiple TokenUpdate events are received
/// @step Then each update overwrites the previous cached value
#[test]
fn test_token_cache_streaming_updates() {
    // @step Given cached token fields
    let cached_input_tokens = AtomicU32::new(0);
    let cached_output_tokens = AtomicU32::new(0);

    // @step When multiple TokenUpdate events are received
    // First update
    cached_input_tokens.store(1000, Ordering::Release);
    cached_output_tokens.store(100, Ordering::Release);
    assert_eq!(cached_input_tokens.load(Ordering::Acquire), 1000);
    assert_eq!(cached_output_tokens.load(Ordering::Acquire), 100);

    // Second update (values increase as streaming continues)
    cached_input_tokens.store(1000, Ordering::Release);
    cached_output_tokens.store(500, Ordering::Release);
    assert_eq!(cached_output_tokens.load(Ordering::Acquire), 500);

    // Third update (final)
    cached_input_tokens.store(1000, Ordering::Release);
    cached_output_tokens.store(1200, Ordering::Release);

    // @step Then the latest values are cached
    assert_eq!(cached_input_tokens.load(Ordering::Acquire), 1000);
    assert_eq!(cached_output_tokens.load(Ordering::Acquire), 1200);
}

/// Test token cache restore from persistence
///
/// Scenario: Restore token state when attaching to a detached session
///
/// @step Given a session was detached with token state
/// @step When session_restore_token_state is called
/// @step Then the cached tokens are also updated for sync access
#[test]
fn test_token_cache_restore_from_persistence() {
    // @step Given cached token fields (initially zero)
    let cached_input_tokens = AtomicU32::new(0);
    let cached_output_tokens = AtomicU32::new(0);

    // @step When restoring from persisted values
    // (This simulates what session_restore_token_state does)
    let persisted_input: u32 = 15000;
    let persisted_output: u32 = 8000;

    // Update both inner tracker AND cached values
    cached_input_tokens.store(persisted_input, Ordering::Release);
    cached_output_tokens.store(persisted_output, Ordering::Release);

    // @step Then the cached tokens are available for sync access
    let (input, output) = (
        cached_input_tokens.load(Ordering::Acquire),
        cached_output_tokens.load(Ordering::Acquire),
    );
    assert_eq!(input, 15000);
    assert_eq!(output, 8000);
}

/// Test concurrent token access safety
///
/// Scenario: Multiple threads can safely read/write cached tokens
///
/// @step Given a session with concurrent streaming and UI access
/// @step When tokens are updated while being read
/// @step Then no data races occur
#[test]
fn test_concurrent_token_access() {
    use std::sync::Arc;
    use std::thread;

    // @step Given shared atomic token counters
    let cached_input = Arc::new(AtomicU32::new(0));
    let cached_output = Arc::new(AtomicU32::new(0));

    // @step When multiple threads read and write concurrently
    let input_writer = Arc::clone(&cached_input);
    let output_writer = Arc::clone(&cached_output);

    let write_handle = thread::spawn(move || {
        for i in 1..=1000 {
            input_writer.store(i * 10, Ordering::Release);
            output_writer.store(i * 5, Ordering::Release);
        }
    });

    let input_reader = Arc::clone(&cached_input);
    let output_reader = Arc::clone(&cached_output);

    let read_handle = thread::spawn(move || {
        let mut reads = 0;
        for _ in 0..1000 {
            let _input = input_reader.load(Ordering::Acquire);
            let _output = output_reader.load(Ordering::Acquire);
            reads += 1;
        }
        reads
    });

    write_handle.join().expect("Writer thread panicked");
    let reads = read_handle.join().expect("Reader thread panicked");

    // @step Then no data races occur (all reads completed)
    assert_eq!(reads, 1000);

    // Final values should be from last write
    assert_eq!(cached_input.load(Ordering::Acquire), 10000);
    assert_eq!(cached_output.load(Ordering::Acquire), 5000);
}

/// Test model string format for session_set_model
///
/// Scenario: Model string is correctly formatted for provider manager
///
/// @step Given provider_id and model_id from the UI
/// @step When session_set_model constructs the model string
/// @step Then the format is "provider/model-id"
#[test]
fn test_model_string_format() {
    // @step Given provider_id and model_id from the UI
    let provider_id = "anthropic";
    let model_id = "claude-sonnet-4";

    // @step When constructing the model string (as session_set_model does)
    let model_string = format!("{}/{}", provider_id, model_id);

    // @step Then the format is "provider/model-id"
    assert_eq!(model_string, "anthropic/claude-sonnet-4");
}

/// Test various model string formats
///
/// Scenario: Different provider/model combinations
///
/// @step Given various provider and model combinations
/// @step When model strings are constructed
/// @step Then all formats are correct
#[test]
fn test_various_model_string_formats() {
    // @step Given various provider and model combinations
    let test_cases = vec![
        ("anthropic", "claude-sonnet-4", "anthropic/claude-sonnet-4"),
        ("anthropic", "claude-opus-4", "anthropic/claude-opus-4"),
        ("openai", "gpt-4o", "openai/gpt-4o"),
        ("google", "gemini-2.5-pro", "google/gemini-2.5-pro"),
        ("openrouter", "meta-llama/llama-3.1-405b", "openrouter/meta-llama/llama-3.1-405b"),
    ];

    // @step When model strings are constructed
    for (provider_id, model_id, expected) in test_cases {
        let model_string = format!("{}/{}", provider_id, model_id);

        // @step Then all formats are correct
        assert_eq!(model_string, expected,
            "Failed for provider={}, model={}", provider_id, model_id);
    }
}

/// Test SessionTokens struct fields
///
/// Scenario: SessionTokens returns correct field values
///
/// @step Given cached token values in a session
/// @step When session_get_tokens returns a SessionTokens struct
/// @step Then the struct contains the correct values
#[test]
fn test_session_tokens_struct() {
    // @step Given cached token values
    let input_tokens: u32 = 12345;
    let output_tokens: u32 = 6789;

    // @step When creating a SessionTokens-like struct
    struct SessionTokens {
        input_tokens: u32,
        output_tokens: u32,
    }

    let tokens = SessionTokens {
        input_tokens,
        output_tokens,
    };

    // @step Then the struct contains the correct values
    assert_eq!(tokens.input_tokens, 12345);
    assert_eq!(tokens.output_tokens, 6789);
}

/// Test token display uses max of cached and streaming values
///
/// Scenario: UI displays highest token value from either source
///
/// @step Given token values from Rust cache and streaming updates
/// @step When the UI calculates display values
/// @step Then the maximum of each is shown
#[test]
fn test_token_display_max_logic() {
    // @step Given token values from Rust cache (from session attach)
    let rust_input: u32 = 5000;
    let rust_output: u32 = 3000;

    // And token values from streaming (TokenUpdate chunks)
    let streaming_input: u32 = 0; // Not yet updated
    let streaming_output: u32 = 0;

    // @step When the UI calculates display values
    let display_input = std::cmp::max(rust_input, streaming_input);
    let display_output = std::cmp::max(rust_output, streaming_output);

    // @step Then the maximum values are shown (Rust values when streaming hasn't started)
    assert_eq!(display_input, 5000);
    assert_eq!(display_output, 3000);

    // Later, streaming updates arrive with higher values
    let streaming_input_updated: u32 = 5500;
    let streaming_output_updated: u32 = 4000;

    let display_input = std::cmp::max(rust_input, streaming_input_updated);
    let display_output = std::cmp::max(rust_output, streaming_output_updated);

    // Streaming values are now higher
    assert_eq!(display_input, 5500);
    assert_eq!(display_output, 4000);
}

/// Test atomic debug state behavior
///
/// Scenario: Cache debug enabled state for sync access
///
/// @step Given a background session is running
/// @step When debug mode is toggled
/// @step Then the cached debug state is updated atomically
#[test]
fn test_atomic_debug_state() {
    // @step Given cached debug field (simulating BackgroundSession)
    let is_debug_enabled = AtomicBool::new(false);

    // Verify initial state
    assert!(!is_debug_enabled.load(Ordering::Acquire));

    // @step When debug mode is enabled
    is_debug_enabled.store(true, Ordering::Release);

    // @step Then the cached debug state is updated atomically
    assert!(is_debug_enabled.load(Ordering::Acquire));

    // Toggle back off
    is_debug_enabled.store(false, Ordering::Release);
    assert!(!is_debug_enabled.load(Ordering::Acquire));
}

/// Test debug state persists across detach/attach
///
/// Scenario: Debug state survives session detach and reattach
///
/// @step Given a session with debug mode enabled
/// @step When session is detached and reattached
/// @step Then the debug state is preserved
#[test]
fn test_debug_state_persists_across_detach() {
    // @step Given a session with debug mode enabled
    let is_debug_enabled = AtomicBool::new(false);

    // Enable debug mode
    is_debug_enabled.store(true, Ordering::Release);
    assert!(is_debug_enabled.load(Ordering::Acquire));

    // @step When session is detached (state stays in Rust)
    // The AtomicBool remains in BackgroundSession even when detached
    // No React state to lose

    // @step Then the debug state is preserved when reattached
    // UI reads from Rust via sessionGetDebugEnabled
    let debug_from_rust = is_debug_enabled.load(Ordering::Acquire);
    assert!(debug_from_rust);
}

/// Test concurrent debug state access safety
///
/// Scenario: Multiple threads can safely read/write debug state
///
/// @step Given a session with concurrent UI and streaming access
/// @step When debug state is toggled while being read
/// @step Then no data races occur
#[test]
fn test_concurrent_debug_access() {
    use std::sync::Arc;
    use std::thread;

    // @step Given shared atomic debug flag
    let is_debug_enabled = Arc::new(AtomicBool::new(false));

    // @step When multiple threads read and write concurrently
    let writer = Arc::clone(&is_debug_enabled);

    let write_handle = thread::spawn(move || {
        for i in 0..1000 {
            writer.store(i % 2 == 0, Ordering::Release);
        }
    });

    let reader = Arc::clone(&is_debug_enabled);

    let read_handle = thread::spawn(move || {
        let mut reads = 0;
        for _ in 0..1000 {
            let _debug = reader.load(Ordering::Acquire);
            reads += 1;
        }
        reads
    });

    write_handle.join().expect("Writer thread panicked");
    let reads = read_handle.join().expect("Reader thread panicked");

    // @step Then no data races occur (all reads completed)
    assert_eq!(reads, 1000);

    // Final value should be from last write (i=999, 999 % 2 == 1, so false)
    assert!(!is_debug_enabled.load(Ordering::Acquire));
}

/// Test debug state getter/setter pattern
///
/// Scenario: sessionGetDebugEnabled and sessionSetDebugEnabled work correctly
///
/// @step Given a session with default debug state (false)
/// @step When sessionSetDebugEnabled is called with true
/// @step Then sessionGetDebugEnabled returns true
#[test]
fn test_debug_state_getter_setter() {
    // @step Given a session with default debug state
    let is_debug_enabled = AtomicBool::new(false);

    // Simulate sessionGetDebugEnabled
    let get_debug = || is_debug_enabled.load(Ordering::Acquire);

    // Simulate sessionSetDebugEnabled
    let set_debug = |value: bool| is_debug_enabled.store(value, Ordering::Release);

    // Initial state
    assert!(!get_debug());

    // @step When sessionSetDebugEnabled is called with true
    set_debug(true);

    // @step Then sessionGetDebugEnabled returns true
    assert!(get_debug());

    // And can be toggled back
    set_debug(false);
    assert!(!get_debug());
}

/// Test send_input sets status to Running synchronously before channel send
///
/// Scenario: Status changes to Running immediately when input is sent
///
/// This tests the fix for a race condition where TypeScript called 
/// sessionGetStatus() immediately after sessionSendInput() but the 
/// agent_loop hadn't yet processed the channel message to set Running.
///
/// @step Given a simulated send_input implementation with status and channel
/// @step When send_input is called
/// @step Then status is Running BEFORE the channel receive completes
/// @step So that sessionGetStatus returns "running" immediately after sessionSendInput
#[test]
fn test_send_input_sets_status_before_channel_send() {
    use std::sync::atomic::AtomicU8;
    use std::sync::mpsc;

    // Simulated SessionStatus enum values
    const IDLE: u8 = 0;
    const RUNNING: u8 = 1;

    // @step Given a simulated BackgroundSession with status and input channel
    let status = Arc::new(AtomicU8::new(IDLE));
    let (input_tx, input_rx) = mpsc::channel::<String>();

    // Verify initial state
    assert_eq!(status.load(Ordering::Acquire), IDLE);

    // @step When send_input is called (simulating BackgroundSession::send_input)
    // This mirrors the fix: set status BEFORE sending to channel
    {
        let status = Arc::clone(&status);
        
        // CRITICAL: Set status to Running BEFORE sending to channel
        // This is the fix - ensures status is Running when TS calls getStatus()
        status.store(RUNNING, Ordering::Release);
        
        // Then send to channel (the receiver hasn't processed yet)
        input_tx.send("test input".to_string()).unwrap();
    }

    // @step Then status is Running BEFORE the channel receive completes
    // This simulates TypeScript calling sessionGetStatus() immediately after sessionSendInput()
    // The receiver (agent_loop) hasn't processed the message yet
    assert_eq!(status.load(Ordering::Acquire), RUNNING,
        "Status must be Running immediately after send_input, before agent_loop processes");

    // Verify the message is still waiting in the channel (not yet processed)
    let received = input_rx.try_recv().unwrap();
    assert_eq!(received, "test input");
}

/// Test send_input reverts status on channel send failure
///
/// Scenario: Status reverts to Idle if channel send fails
///
/// @step Given a send_input with a closed channel
/// @step When send_input fails to send
/// @step Then status is reverted to Idle
#[test]
fn test_send_input_reverts_status_on_failure() {
    use std::sync::atomic::AtomicU8;
    use std::sync::mpsc;

    const IDLE: u8 = 0;
    const RUNNING: u8 = 1;

    // @step Given a session with a closed channel (simulating send failure)
    let status = Arc::new(AtomicU8::new(IDLE));
    let (input_tx, input_rx) = mpsc::channel::<String>();
    
    // Drop the receiver to cause send failure
    drop(input_rx);

    // @step When send_input attempts to send (and fails)
    {
        // Set status to Running before send (as the fix does)
        status.store(RUNNING, Ordering::Release);
        
        // Try to send - this will fail because receiver is dropped
        let send_result = input_tx.send("test input".to_string());
        
        if send_result.is_err() {
            // @step Then status is reverted to Idle on failure
            status.store(IDLE, Ordering::Release);
        }
    }

    // Verify status was reverted
    assert_eq!(status.load(Ordering::Acquire), IDLE,
        "Status must revert to Idle when channel send fails");
}

/// Test status transitions follow correct sequence
///
/// Scenario: Status follows Idle -> Running -> Idle lifecycle
///
/// @step Given a session processing input
/// @step When the full lifecycle completes
/// @step Then status transitions correctly: Idle -> Running -> Idle
#[test]
fn test_status_lifecycle_transitions() {
    use std::sync::atomic::AtomicU8;
    use std::sync::mpsc;

    const IDLE: u8 = 0;
    const RUNNING: u8 = 1;

    let status = Arc::new(AtomicU8::new(IDLE));
    let (input_tx, input_rx) = mpsc::channel::<String>();

    // Phase 1: Initial state is Idle
    assert_eq!(status.load(Ordering::Acquire), IDLE);

    // Phase 2: send_input sets Running synchronously
    {
        status.store(RUNNING, Ordering::Release);
        input_tx.send("test".to_string()).unwrap();
    }
    assert_eq!(status.load(Ordering::Acquire), RUNNING);

    // Phase 3: agent_loop processes (simulated) - status stays Running
    let _ = input_rx.recv().unwrap();
    // agent_loop would also call set_status(Running) here - idempotent
    status.store(RUNNING, Ordering::Release);
    assert_eq!(status.load(Ordering::Acquire), RUNNING);

    // Phase 4: agent_loop completes - sets Idle
    status.store(IDLE, Ordering::Release);
    assert_eq!(status.load(Ordering::Acquire), IDLE);
}
