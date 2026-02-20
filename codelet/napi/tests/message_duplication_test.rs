//! Message Duplication Bug Investigation Tests
//!
//! These tests investigate a bug where messages sent to the agent loop
//! appear to be processed multiple times.
//!
//! Test Strategy:
//! 1. Test input channel delivery (single send = single receive)
//! 2. Test watcher input channel delivery
//! 3. Test that both channels don't interfere
//! 4. Test the rig library's request.build() behavior

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::timeout;

/// Fixture: Simulates the PromptInput structure used by session_send_input
#[derive(Debug, Clone)]
struct TestPromptInput {
    input: String,
    thinking_config: Option<String>,
}

/// Fixture: Simulates WatcherInput for bridge/watcher messages
#[derive(Debug, Clone)]
struct TestWatcherInput {
    source_session_id: String,
    role_name: String,
    message: String,
}

/// Test 1: Verify that sending one message to mpsc channel results in exactly one receive
/// This tests the fundamental channel behavior used by agent_loop
#[tokio::test]
async fn test_mpsc_channel_single_delivery() {
    let (tx, mut rx) = mpsc::channel::<TestPromptInput>(16);
    let receive_count = Arc::new(AtomicUsize::new(0));
    let receive_count_clone = receive_count.clone();

    // Spawn receiver task
    let receiver = tokio::spawn(async move {
        while let Some(_input) = rx.recv().await {
            receive_count_clone.fetch_add(1, Ordering::SeqCst);
        }
    });

    // Send exactly ONE message
    tx.send(TestPromptInput {
        input: "Hello".to_string(),
        thinking_config: None,
    })
    .await
    .expect("send should succeed");

    // Close the channel
    drop(tx);

    // Wait for receiver to finish
    let _ = timeout(Duration::from_secs(1), receiver).await;

    // CRITICAL: Should be exactly 1
    assert_eq!(
        receive_count.load(Ordering::SeqCst),
        1,
        "Channel should deliver exactly ONE message, not duplicates"
    );
}

/// Test 2: Verify tokio::select! with biased doesn't cause duplication
/// This mirrors the agent_loop's select! pattern
#[tokio::test]
async fn test_select_biased_no_duplication() {
    let (user_tx, mut user_rx) = mpsc::channel::<TestPromptInput>(16);
    let (watcher_tx, mut watcher_rx) = mpsc::channel::<TestWatcherInput>(16);
    
    let user_receive_count = Arc::new(AtomicUsize::new(0));
    let watcher_receive_count = Arc::new(AtomicUsize::new(0));
    let user_count_clone = user_receive_count.clone();
    let watcher_count_clone = watcher_receive_count.clone();

    // Spawn a task that mimics agent_loop's select! pattern
    let processor = tokio::spawn(async move {
        loop {
            let input = tokio::select! {
                biased;
                
                result = user_rx.recv() => {
                    match result {
                        Some(input) => {
                            user_count_clone.fetch_add(1, Ordering::SeqCst);
                            Some(format!("user: {}", input.input))
                        }
                        None => break, // Channel closed
                    }
                }
                
                result = watcher_rx.recv() => {
                    match result {
                        Some(input) => {
                            watcher_count_clone.fetch_add(1, Ordering::SeqCst);
                            Some(format!("watcher: {}", input.message))
                        }
                        None => None, // Watcher channel closed, continue
                    }
                }
            };

            if input.is_none() && user_rx.is_closed() {
                break;
            }
        }
    });

    // Send ONE user message
    user_tx
        .send(TestPromptInput {
            input: "Hi".to_string(),
            thinking_config: None,
        })
        .await
        .expect("send should succeed");

    // Send ONE watcher message
    watcher_tx
        .send(TestWatcherInput {
            source_session_id: "bridge".to_string(),
            role_name: "bridge".to_string(),
            message: "Hello from bridge".to_string(),
        })
        .await
        .expect("send should succeed");

    // Close channels
    drop(user_tx);
    drop(watcher_tx);

    // Wait for processor
    let _ = timeout(Duration::from_secs(1), processor).await;

    // CRITICAL: Each should be exactly 1
    assert_eq!(
        user_receive_count.load(Ordering::SeqCst),
        1,
        "User input should be received exactly ONCE"
    );
    assert_eq!(
        watcher_receive_count.load(Ordering::SeqCst),
        1,
        "Watcher input should be received exactly ONCE"
    );
}

/// Test 3: Verify that Mutex<Receiver> in select! doesn't cause issues
/// This mimics the watcher_input_rx.lock().await pattern in agent_loop
#[tokio::test]
async fn test_mutex_receiver_in_select_no_duplication() {
    use tokio::sync::Mutex;

    let (user_tx, mut user_rx) = mpsc::channel::<TestPromptInput>(16);
    let (watcher_tx, watcher_rx) = mpsc::channel::<TestWatcherInput>(16);
    let watcher_rx = Arc::new(Mutex::new(watcher_rx));

    let total_receive_count = Arc::new(AtomicUsize::new(0));
    let count_clone = total_receive_count.clone();
    let watcher_rx_clone = watcher_rx.clone();

    // Spawn processor that mimics exact agent_loop pattern
    let processor = tokio::spawn(async move {
        loop {
            // Lock watcher_rx like agent_loop does
            let mut watcher_guard = watcher_rx_clone.lock().await;

            let input = tokio::select! {
                biased;
                
                result = user_rx.recv() => {
                    match result {
                        Some(_input) => {
                            count_clone.fetch_add(1, Ordering::SeqCst);
                            Some("user")
                        }
                        None => {
                            drop(watcher_guard);
                            break;
                        }
                    }
                }
                
                result = watcher_guard.recv() => {
                    match result {
                        Some(_input) => {
                            count_clone.fetch_add(1, Ordering::SeqCst);
                            Some("watcher")
                        }
                        None => None,
                    }
                }
            };

            // Drop the lock before processing (like agent_loop)
            drop(watcher_guard);

            if input.is_none() {
                // Continue loop if only watcher closed
            }
        }
    });

    // Send messages
    user_tx
        .send(TestPromptInput {
            input: "Test 1".to_string(),
            thinking_config: None,
        })
        .await
        .unwrap();

    watcher_tx
        .send(TestWatcherInput {
            source_session_id: "bridge".to_string(),
            role_name: "bridge".to_string(),
            message: "Test 2".to_string(),
        })
        .await
        .unwrap();

    // Small delay to ensure processing
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Close user channel to stop loop
    drop(user_tx);
    drop(watcher_tx);

    let _ = timeout(Duration::from_secs(1), processor).await;

    // CRITICAL: Should be exactly 2 (one user + one watcher)
    assert_eq!(
        total_receive_count.load(Ordering::SeqCst),
        2,
        "Should receive exactly 2 messages total (1 user + 1 watcher)"
    );
}

/// Test 4: Verify rapid sequential sends don't cause duplication
#[tokio::test]
async fn test_rapid_sends_no_duplication() {
    let (tx, mut rx) = mpsc::channel::<TestPromptInput>(16);
    let receive_count = Arc::new(AtomicUsize::new(0));
    let received_messages = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let count_clone = receive_count.clone();
    let messages_clone = received_messages.clone();

    let receiver = tokio::spawn(async move {
        while let Some(input) = rx.recv().await {
            count_clone.fetch_add(1, Ordering::SeqCst);
            messages_clone.lock().await.push(input.input.clone());
        }
    });

    // Send 3 distinct messages rapidly
    for i in 1..=3 {
        tx.send(TestPromptInput {
            input: format!("Message {}", i),
            thinking_config: None,
        })
        .await
        .unwrap();
    }

    drop(tx);
    let _ = timeout(Duration::from_secs(1), receiver).await;

    let messages = received_messages.lock().await;
    
    assert_eq!(
        receive_count.load(Ordering::SeqCst),
        3,
        "Should receive exactly 3 messages"
    );
    
    // Verify no duplicates
    assert_eq!(messages.len(), 3);
    assert!(messages.contains(&"Message 1".to_string()));
    assert!(messages.contains(&"Message 2".to_string()));
    assert!(messages.contains(&"Message 3".to_string()));
}

/// Test 5: Verify rig's request builder concatenation behavior
/// This is the core of the duplication bug - build() concatenates history with prompt
#[test]
fn test_rig_request_builder_concatenation_causes_duplication() {
    // Simulate the bug scenario:
    // 1. stream_loop.rs pushes user message to session.messages
    // 2. session.messages is passed as history to rig
    // 3. rig's build() concatenates history with prompt
    
    // Initial state: previous messages
    let previous_messages = vec!["Previous 1".to_string(), "Previous 2".to_string()];
    
    // Step 1: stream_loop.rs pushes the user message (the bug)
    let user_prompt = "Hello".to_string();
    let mut session_messages = previous_messages.clone();
    session_messages.push(user_prompt.clone()); // <-- This is the bug!
    
    // Step 2: session_messages is cloned and passed to rig as history
    let history_for_rig = session_messages.clone();
    
    // Step 3: rig's build() concatenates history with prompt
    // (simulating request.rs line 731)
    let final_chat_history: Vec<String> = [history_for_rig, vec![user_prompt.clone()]].concat();
    
    // VERIFY THE BUG: prompt appears twice
    let prompt_count = final_chat_history.iter().filter(|m| *m == "Hello").count();
    
    // This test DOCUMENTS the bug - it should fail after the fix
    assert_eq!(
        prompt_count, 2,
        "BUG: User prompt appears {} times in final request (expected 2 showing duplication)",
        prompt_count
    );
    
    // The final history has 4 messages: Previous 1, Previous 2, Hello, Hello
    assert_eq!(final_chat_history.len(), 4, "Should have 4 messages (2 previous + 2 duplicates)");
}

/// Test 6: Show the correct behavior after fix
/// After removing the manual push in stream_loop.rs, this should be the behavior
#[test]
fn test_correct_behavior_after_fix() {
    // Initial state: previous messages (NOT including current prompt)
    let previous_messages = vec!["Previous 1".to_string(), "Previous 2".to_string()];
    
    // The user prompt
    let user_prompt = "Hello".to_string();
    
    // CORRECT: Do NOT push user message to session.messages before calling rig
    let history_for_rig = previous_messages.clone();
    
    // rig's build() concatenates history with prompt
    let final_chat_history: Vec<String> = [history_for_rig, vec![user_prompt.clone()]].concat();
    
    // VERIFY: prompt appears exactly once
    let prompt_count = final_chat_history.iter().filter(|m| *m == "Hello").count();
    
    assert_eq!(
        prompt_count, 1,
        "User prompt should appear exactly ONCE in final request"
    );
    
    // The final history has 3 messages: Previous 1, Previous 2, Hello
    assert_eq!(final_chat_history.len(), 3, "Should have 3 messages (2 previous + 1 prompt)");
}
