# AST Research: Codelet TypeScript Structure

## Tools Module (src/agent/tools.ts)

### Key Functions:
- `truncateOutput(lines, maxChars)` - Utility for truncating output
- `formatTruncationWarning(remainingCount, itemType, charTruncated)` - Formats warning message
- `getErrorMessage(error)` - Error message extraction
- `executeBash(command)` - Execute shell command with truncation
- `executeRead(filePath, offset?, limit?)` - Read file with line numbers
- `executeEdit(filePath, oldString, newString)` - Edit file with exact match
- `executeWrite(filePath, content)` - Write file content
- `executeAstGrep(pattern, language, paths?)` - AST-based code search
- `executeGrep(params)` - Ripgrep-based search
- `executeGlob(pattern, path?)` - File pattern matching (discovered)

### Constants:
- `OUTPUT_LIMITS.MAX_OUTPUT_CHARS` - Character limit (30000)
- `OUTPUT_LIMITS.MAX_LINE_LENGTH` - Line length limit (2000)
- `OUTPUT_LIMITS.MAX_LINES` - Line count limit (2000)

## Token Tracker Module (src/agent/token-tracker.ts)

### Key Functions:
- `createTokenTracker()` - Creates initial TokenUsage object
- `updateFromClaudeMessageStart(tracker, event)` - Updates from Claude message start
- `updateFromClaudeMessageDelta(tracker, event)` - Updates from Claude message delta
- `updateFromCodexResponseCompleted(tracker, event)` - Updates from Codex response
- `estimateTokens(text)` - Estimates tokens from text

### TokenUsage Interface Fields:
- inputTokens: number
- outputTokens: number
- cachedInputTokens: number
- reasoningTokens: number
- totalTokens: number
- cacheReadInputTokens: number
- cacheCreationInputTokens: number

## Mapping to Rust Bounded Contexts

| TypeScript Module | Rust Bounded Context | Key Types |
|-------------------|---------------------|-----------|
| tools.ts | Tool Execution | Tool trait, ToolRegistry |
| token-tracker.ts | Context Management | TokenTracker struct |
| runner.ts | Agent Execution | Runner, Message types |
| provider-manager.ts | Provider Management | LlmProvider trait, ProviderType |
| cli.ts | CLI Interface | Cli struct |
