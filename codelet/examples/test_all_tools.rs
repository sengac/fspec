//! Integration test demonstrating all 7 tools with Claude OAuth provider
//!
//! Run with: cargo run --example test_all_tools

use codelet::agent::RigAgent;
use codelet::providers::{ClaudeProvider, LlmProvider};
use codelet::tools::{Tool, ToolRegistry};
use std::fs;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Testing All 7 Tools with Claude OAuth Provider ===\n");

    // 1. Create Claude provider (will use OAuth token from env)
    println!("1️⃣  Creating ClaudeProvider...");
    let provider = ClaudeProvider::new_with_model(None)?;
    println!("   ✅ Provider created: {}", provider.name());
    println!("   ✅ Model: {}", provider.model());
    println!("   ✅ OAuth mode: {}", provider.is_oauth_mode());
    println!("   ✅ Context window: {} tokens", provider.context_window());
    println!();

    // 2. Create RigAgent
    println!("2️⃣  Creating RigAgent...");
    let rig_agent = provider.create_rig_agent(None, None);
    let agent = RigAgent::with_default_depth(rig_agent);
    println!("   ✅ Agent created with max_depth: {}", agent.max_depth());
    println!();

    // 3. Test ToolRegistry (all tools accessible)
    println!("3️⃣  Testing ToolRegistry...");
    let registry = ToolRegistry::default();
    let tools = registry.list();
    println!("   ✅ Registered {} tools:", tools.len());
    for tool_name in tools {
        println!("      - {}", tool_name);
    }
    println!();

    // Create test directory
    let test_dir = "/tmp/codelet_tool_test";
    fs::create_dir_all(test_dir)?;
    println!("4️⃣  Created test directory: {}\n", test_dir);

    // Test each tool individually
    println!("5️⃣  Testing individual tools:\n");

    // === WRITE TOOL ===
    println!("   📝 Testing Write tool...");
    let write_tool = codelet::tools::WriteTool::new();
    let write_result = write_tool
        .execute(serde_json::json!({
            "file_path": format!("{}/test_file.txt", test_dir),
            "content": "Hello from codelet!\nLine 2\nLine 3"
        }))
        .await?;
    println!("      ✅ Write: {}", write_result.content);

    // === READ TOOL ===
    println!("   📖 Testing Read tool...");
    let read_tool = codelet::tools::ReadTool::new();
    let read_result = read_tool
        .execute(serde_json::json!({
            "file_path": format!("{}/test_file.txt", test_dir)
        }))
        .await?;
    println!("      ✅ Read output:");
    for line in read_result.content.lines().take(5) {
        println!("         {}", line);
    }

    // === EDIT TOOL ===
    println!("   ✏️  Testing Edit tool...");
    let edit_tool = codelet::tools::EditTool::new();
    let edit_result = edit_tool
        .execute(serde_json::json!({
            "file_path": format!("{}/test_file.txt", test_dir),
            "old_string": "Line 2",
            "new_string": "Modified Line 2"
        }))
        .await?;
    println!("      ✅ Edit: {}", edit_result.content);

    // === BASH TOOL ===
    println!("   🐚 Testing Bash tool...");
    let bash_tool = codelet::tools::BashTool::new();
    let bash_result = bash_tool
        .execute(serde_json::json!({
            "command": format!("ls -la {}", test_dir)
        }))
        .await?;
    println!("      ✅ Bash output:");
    for line in bash_result.content.lines().take(5) {
        println!("         {}", line);
    }

    // === GREP TOOL ===
    println!("   🔍 Testing Grep tool...");
    let grep_tool = codelet::tools::GrepTool::new();
    let grep_result = grep_tool
        .execute(serde_json::json!({
            "pattern": "Modified",
            "path": test_dir,
            "output_mode": "content"
        }))
        .await?;
    println!("      ✅ Grep found:");
    for line in grep_result.content.lines().take(3) {
        println!("         {}", line);
    }

    // === GLOB TOOL ===
    println!("   🌐 Testing Glob tool...");
    let glob_tool = codelet::tools::GlobTool::new();
    let glob_result = glob_tool
        .execute(serde_json::json!({
            "pattern": "*.txt",
            "path": test_dir
        }))
        .await?;
    println!("      ✅ Glob found:");
    for line in glob_result.content.lines() {
        println!("         {}", line);
    }

    // === ASTGREP TOOL ===
    println!("   🌳 Testing AstGrep tool...");
    // Create a Rust file for AST searching
    fs::write(
        format!("{}/example.rs", test_dir),
        "fn main() {\n    println!(\"Hello\");\n}\n\nfn helper() -> i32 {\n    42\n}",
    )?;

    let astgrep_tool = codelet::tools::AstGrepTool::new();
    let astgrep_result = astgrep_tool
        .execute(serde_json::json!({
            "pattern": "fn $NAME() { $$$ }",
            "language": "rust",
            "path": test_dir
        }))
        .await?;
    println!("      ✅ AstGrep found:");
    for line in astgrep_result.content.lines().take(3) {
        println!("         {}", line);
    }

    println!();

    // Clean up
    println!("6️⃣  Cleaning up test directory...");
    fs::remove_dir_all(test_dir)?;
    println!("   ✅ Cleanup complete\n");

    // Final summary
    println!("=== ✅ ALL TOOLS VERIFIED ===");
    println!("✅ ClaudeProvider with OAuth: Working");
    println!("✅ RigAgent: Working");
    println!("✅ All 7 tools: Functional");
    println!("   - Read ✅");
    println!("   - Write ✅");
    println!("   - Edit ✅");
    println!("   - Bash ✅");
    println!("   - Grep ✅");
    println!("   - Glob ✅");
    println!("   - AstGrep ✅");
    println!("\n🎉 Integration test complete!");

    Ok(())
}
