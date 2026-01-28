# AST Research: Codelet Tool Patterns

## Tool Implementation Pattern Analysis

### Research Commands Used
```bash
fspec research --tool=ast --pattern="impl rig::tool::Tool for $NAME" --lang=rust --path=codelet/tools/src/
fspec research --tool=ast --pattern="pub struct $_Tool" --lang=rust --path=codelet/tools/src/
```

### Existing Tools Structure

All tools follow the same pattern:

1. **Tool Struct**: Simple unit struct (e.g., `pub struct BashTool;`)
2. **Args Struct**: Parameters for the tool (e.g., `pub struct BashArgs { pub command: String }`)
3. **Tool Trait Implementation**: `impl rig::tool::Tool for ToolName`

### Tool Trait Pattern

From the research, all tools implement:
```rust
impl rig::tool::Tool for ToolName {
    const NAME: &'static str = "ToolName";
    type Error = ToolError;
    type Args = ToolNameArgs;
    type Output = String; // or other appropriate type
    
    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        // Tool definition with name, description, parameters
    }
    
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Tool implementation logic
    }
}
```

### Examples Found

1. **BashTool**: Executes shell commands
   - `pub struct BashTool;`
   - `pub struct BashArgs { pub command: String }`
   - Spawns processes and handles stdout/stderr

2. **ReadTool**: Reads file contents
   - `pub struct ReadTool;`
   - `pub struct ReadArgs { file_path, offset, limit, etc. }`
   - Direct file system operations

3. **WriteTool**: Writes file contents
   - `pub struct WriteTool;`
   - `pub struct WriteArgs { file_path, content }`
   - Direct file system operations

4. **AstGrepTool**: AST-based code search
   - `pub struct AstGrepTool;`
   - `pub struct AstGrepArgs { pattern, language, path, etc. }`
   - Complex pattern matching logic

### Key Insights for FspecTool

1. **Follow Established Pattern**: Create `FspecTool` and `FspecArgs` structs
2. **Command Interface**: FspecArgs should have `command: String` and `args: Vec<String>` 
3. **Error Handling**: Use `ToolError` for consistency
4. **Direct Function Calls**: Unlike BashTool which spawns processes, FspecTool should call fspec TypeScript functions directly via NAPI-RS
5. **System Reminder Preservation**: Need to capture and return system reminders in tool response

### Recommended FspecTool Structure

```rust
pub struct FspecTool;

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct FspecArgs {
    pub command: String,
    pub args: Option<Vec<String>>,
    pub options: Option<serde_json::Value>,
}

impl rig::tool::Tool for FspecTool {
    const NAME: &'static str = "Fspec";
    type Error = ToolError;
    type Args = FspecArgs;
    type Output = String; // JSON response with data + system_reminder
    
    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        // Define tool parameters
    }
    
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Call fspec TypeScript function via NAPI-RS
        // Parse response to extract data + system_reminder
        // Return structured response
    }
}
```