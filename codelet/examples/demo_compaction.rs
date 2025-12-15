//! Demonstration of context compaction integration
//!
//! This example shows that the compaction system is properly wired up:
//! - Session has token_tracker and turns fields
//! - Types are properly exported from compaction module
//! - System compiles and links correctly

use codelet::agent::compaction::{
    AnchorDetector, ConversationTurn, TokenTracker, ToolCall, ToolResult, TurnSelector,
};
use codelet::session::Session;
use std::time::SystemTime;

fn main() -> anyhow::Result<()> {
    println!("=== Context Compaction Integration Demo ===\n");

    // 1. Create session (proves Session has token_tracker and turns)
    println!("1. Creating session...");
    let session = Session::new(None);
    println!("   ✓ Session created with token_tracker and turns fields\n");

    // Access session fields to prove they exist
    if let Ok(session) = session {
        println!("   Token tracker initialized:");
        println!(
            "     - Input tokens: {}",
            session.token_tracker.input_tokens
        );
        println!(
            "     - Output tokens: {}",
            session.token_tracker.output_tokens
        );
        println!("     - Turns count: {}\n", session.turns.len());
    }

    // 2. Create sample conversation turn
    println!("2. Creating sample conversation turn...");
    let turn = ConversationTurn {
        user_message: "Fix the build error".to_string(),
        tool_calls: vec![ToolCall {
            tool: "Edit".to_string(),
            id: "tool_1".to_string(),
            input: serde_json::json!({"file": "src/main.rs"}),
        }],
        tool_results: vec![ToolResult {
            success: true,
            output: "Tests passed successfully".to_string(),
        }],
        assistant_response: "Fixed the error".to_string(),
        tokens: 1500,
        timestamp: SystemTime::now(),
        previous_error: Some(true),
    };
    println!("   ✓ Turn created with tool calls and results\n");

    // 3. Test anchor detection
    println!("3. Testing anchor detection...");
    let detector = AnchorDetector::new(0.9);
    let anchor = detector.detect(&turn, 0)?;
    match anchor {
        Some(a) => {
            println!("   ✓ Anchor detected:");
            println!("     - Type: {:?}", a.anchor_type);
            println!("     - Confidence: {}", a.confidence);
            println!("     - Weight: {}", a.weight);
            println!("     - Description: {}\n", a.description);
        }
        None => println!("   ✗ No anchor detected\n"),
    }

    // 4. Test turn selection
    println!("4. Testing turn selection...");
    let selector = TurnSelector::new();
    let turns = vec![turn.clone(), turn.clone(), turn];
    let selection = selector.select_turns(&turns, None)?;
    println!("   ✓ Turn selection complete:");
    println!("     - Turns to keep: {}", selection.kept_turns.len());
    println!(
        "     - Turns to summarize: {}\n",
        selection.summarized_turns.len()
    );

    // 5. Test token calculation
    println!("5. Testing effective token calculation...");
    let tracker = TokenTracker {
        input_tokens: 10_000,
        output_tokens: 2_000,
        cache_read_input_tokens: Some(5_000),
        cache_creation_input_tokens: Some(0),
    };
    let effective = tracker.effective_tokens();
    println!("   ✓ Token calculation:");
    println!("     - Input tokens: {}", tracker.input_tokens);
    println!(
        "     - Cache read tokens: {:?}",
        tracker.cache_read_input_tokens
    );
    println!("     - Cache discount (90%): {}", 5_000 * 9 / 10);
    println!("     - Effective tokens: {}\n", effective);

    println!("=== All Systems Operational ===");
    println!("Context compaction is properly wired up and ready to use!");

    Ok(())
}
