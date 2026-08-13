use codelet::tools::{
    AstGrepTool, BashTool, EditTool, GlobTool, GrepTool, ReadTool, Tool, WriteTool,
};
use serde_json::json;
use uuid::Uuid;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let test_dir = "/tmp/tool_detailed_demo";
    std::fs::create_dir_all(test_dir)?;

    println!("=== DETAILED TOOL OUTPUT EXAMPLES ===\n");

    // 1. Write Tool
    println!("📝 WRITE TOOL - Create a multi-line file");
    let write_tool = WriteTool::new(Uuid::nil());
    let result = write_tool.execute(json!({
        "file_path": format!("{}/sample.rs", test_dir),
        "content": "fn fibonacci(n: u32) -> u32 {\n    match n {\n        0 => 0,\n        1 => 1,\n        _ => fibonacci(n - 1) + fibonacci(n - 2),\n    }\n}"
    })).await?;
    println!("   {}\n", result.content);

    // 2. Read Tool
    println!("📖 READ TOOL - Read with line numbers");
    let read_tool = ReadTool::new(Uuid::nil());
    let result = read_tool
        .execute(json!({
            "file_path": format!("{}/sample.rs", test_dir)
        }))
        .await?;
    println!("   Output:");
    for line in result.content.lines().take(8) {
        println!("   {}", line);
    }
    println!();

    // 3. Edit Tool
    println!("✏️  EDIT TOOL - Replace function name");
    let edit_tool = EditTool::new(Uuid::nil());
    let result = edit_tool
        .execute(json!({
            "file_path": format!("{}/sample.rs", test_dir),
            "old_string": "fibonacci",
            "new_string": "fib"
        }))
        .await?;
    println!("   {}\n", result.content);

    // 4. Bash Tool
    println!("🐚 BASH TOOL - Execute command");
    let bash_tool = BashTool::new(Uuid::nil());
    let result = bash_tool
        .execute(json!({
            "command": format!("wc -l {}/sample.rs", test_dir)
        }))
        .await?;
    println!("   Output: {}\n", result.content.trim());

    // 5. Grep Tool
    println!("🔍 GREP TOOL - Search for pattern");
    let grep_tool = GrepTool::new(Uuid::nil());
    let result = grep_tool
        .execute(json!({
            "pattern": "fn.*\\(",
            "path": test_dir,
            "output_mode": "content"
        }))
        .await?;
    println!("   Found:");
    for line in result.content.lines() {
        println!("   {}", line);
    }
    println!();

    // 6. Glob Tool
    println!("🌐 GLOB TOOL - Find files by pattern");
    let glob_tool = GlobTool::new(Uuid::nil());
    let result = glob_tool
        .execute(json!({
            "pattern": "*.rs",
            "path": test_dir
        }))
        .await?;
    println!("   Found:");
    for line in result.content.lines() {
        println!("   {}", line);
    }
    println!();

    // 7. AstGrep Tool
    println!("🌳 ASTGREP TOOL - Find function definitions");
    let astgrep_tool = AstGrepTool::new(Uuid::nil());
    let result = astgrep_tool
        .execute(json!({
            "pattern": "fn $NAME($$$ARGS) { $$$ }",
            "language": "rust",
            "path": test_dir
        }))
        .await?;
    println!("   Found:");
    for line in result.content.lines() {
        println!("   {}", line);
    }

    // Cleanup
    std::fs::remove_dir_all(test_dir)?;

    println!("\n✅ All tools demonstrated successfully!");
    Ok(())
}
