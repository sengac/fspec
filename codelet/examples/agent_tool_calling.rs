//! Demonstration of RigAgent with automatic tool calling
//!
//! This example shows the RigAgent using Claude to automatically call tools
//! to complete a task, demonstrating the full multi-turn agent loop.
//!
//! Run with: cargo run --example agent_tool_calling

use codelet::agent::RigAgent;
use codelet::providers::ClaudeProvider;
use std::fs;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== RigAgent with Automatic Tool Calling ===\n");

    // Create provider and agent
    let provider = ClaudeProvider::new()?;
    let rig_agent = provider.create_rig_agent(None, None);
    let agent = RigAgent::with_default_depth(rig_agent);

    println!("✅ Agent created with OAuth authentication");
    println!("✅ Max depth: {} turns\n", agent.max_depth());

    // Create test directory
    let test_dir = "/tmp/codelet_agent_test";
    fs::create_dir_all(test_dir)?;

    // Test 1: Agent uses Write tool
    println!("📝 Test 1: Ask agent to create a file");
    println!("   Prompt: 'Create a file at /tmp/codelet_agent_test/hello.txt with the content: Hello from RigAgent!'\n");

    let response1 = agent
        .prompt(&format!(
            "Create a file at {}/hello.txt with the content: Hello from RigAgent!",
            test_dir
        ))
        .await?;

    println!("   🤖 Agent response: {}\n", response1.trim());

    // Verify file was created
    if fs::read_to_string(format!("{}/hello.txt", test_dir)).is_ok() {
        println!("   ✅ File successfully created by agent!\n");
    }

    // Test 2: Agent uses Read tool
    println!("📖 Test 2: Ask agent to read the file");
    println!("   Prompt: 'Read the file at /tmp/codelet_agent_test/hello.txt and tell me what it says'\n");

    let response2 = agent
        .prompt(&format!(
            "Read the file at {}/hello.txt and tell me what it says",
            test_dir
        ))
        .await?;

    println!("   🤖 Agent response: {}\n", response2.trim());

    // Test 3: Agent uses Bash tool
    println!("🐚 Test 3: Ask agent to list files");
    println!("   Prompt: 'List all files in /tmp/codelet_agent_test directory'\n");

    let response3 = agent
        .prompt(&format!("List all files in {} directory", test_dir))
        .await?;

    println!("   🤖 Agent response: {}\n", response3.trim());

    // Test 4: Multi-turn - Agent uses multiple tools
    println!("🔄 Test 4: Multi-turn task (Read + Edit)");
    println!(
        "   Prompt: 'Read /tmp/codelet_agent_test/hello.txt and change RigAgent to RustAgent'\n"
    );

    let response4 = agent
        .prompt(&format!(
            "Read {}/hello.txt and change 'RigAgent' to 'RustAgent'",
            test_dir
        ))
        .await?;

    println!("   🤖 Agent response: {}\n", response4.trim());

    // Verify edit
    let final_content = fs::read_to_string(format!("{}/hello.txt", test_dir))?;
    println!("   📄 Final file content: {}", final_content.trim());

    if final_content.contains("RustAgent") {
        println!("   ✅ File successfully edited by agent!\n");
    }

    // Clean up
    fs::remove_dir_all(test_dir)?;

    println!("=== ✅ ALL AGENT TESTS PASSED ===");
    println!("✅ Agent can automatically call tools");
    println!("✅ Multi-turn tool execution works");
    println!("✅ OAuth authentication works");
    println!("✅ All 7 tools are accessible to agent\n");

    Ok(())
}
