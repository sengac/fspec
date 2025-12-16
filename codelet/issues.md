# Rust Compaction Implementation Issues

## Summary

After conducting a thorough comparison between the Rust port in `core/src/compaction.rs` and the original TypeScript implementation in `~/projects/codelet/src/agent/anchor-point-compaction.ts`, several critical discrepancies were identified that affect the functionality and accuracy of the anchor point detection system.

## ❌ CRITICAL ISSUES

### 1. Missing File Path Extraction Logic

**Issue**: The Rust implementation lacks the file path extraction logic that is present in the TypeScript original.

**TypeScript Implementation (lines 186-191):**
```typescript
const modifiedFiles = turn.toolCalls
  .filter(call => call.tool === 'Edit' || call.tool === 'Write')
  .map(call => call.parameters.file_path as string)
  .filter(Boolean)
  .map(path => path.split('/').pop() || path); // Get filename only
```

**Rust Implementation (lines 481-484):** Only checks for tool type:
```rust
let has_file_modification = turn
    .tool_calls
    .iter()
    .any(|call| call.tool == "Edit" || call.tool == "Write");
```

**Impact**: Anchor descriptions will be generic without file context, reducing the quality and specificity of compaction summaries.

**Fix Required**: Implement file path extraction from tool call parameters.

---

### 2. Incompatible Tool Parameter Structure

**Issue**: The Rust and TypeScript versions use fundamentally different structures for tool call parameters.

**TypeScript ToolCall:**
```typescript
export interface ToolCall {
  tool: string;
  parameters: Record<string, unknown>; // Structured parameters
}
```

**Rust ToolCall:**
```rust
pub struct ToolCall {
    pub tool: String,
    pub id: String,              // Extra field not in TypeScript
    pub input: serde_json::Value, // Generic JSON, not structured
}
```

**Impact**: Cannot extract `file_path` from `call.parameters.file_path` as done in TypeScript because Rust uses `call.input` as generic JSON.

**Fix Required**: Update Rust ToolCall structure to match TypeScript interface or implement helper methods to extract structured parameters from `serde_json::Value`.

---

### 3. Missing Error Field in ToolResult

**Issue**: The Rust ToolResult structure is missing the optional error field present in TypeScript.

**TypeScript ToolResult:**
```typescript
export interface ToolResult {
  success: boolean;
  output: string;
  error?: string; // Missing in Rust
}
```

**Rust ToolResult:**
```rust
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    // Missing error field
}
```

**Impact**: Cannot capture and process error information from tool execution results.

**Fix Required**: Add optional error field to ToolResult structure.

---

### 4. Missing File-Aware Anchor Descriptions

**Issue**: Rust implementation generates generic anchor descriptions without file context.

**TypeScript generates:**
```
"Build error fixed in file1.rs, file2.rs and tests now pass"
```

**Rust generates:**
```
"Build error fixed and tests now pass"
```

**Impact**: Less informative anchor descriptions reduce the usefulness of compaction summaries for understanding what was changed.

**Fix Required**: Implement file-aware description generation.

---

## ✅ CORRECTLY IMPLEMENTED

The following aspects were correctly ported from TypeScript to Rust:

1. **Core anchor detection logic** - Both use same confidence thresholds (0.95, 0.92)
2. **Anchor type priorities** - Error resolution > Task completion  
3. **Pattern matching** - Previous error + file modification + test success
4. **Weight assignments** - 0.9 for error resolution, 0.8 for task completion
5. **Turn selection strategy** - Keep from anchor forward, summarize before

---

## 🔧 RECOMMENDED FIXES

### Fix 1: Update ToolCall Structure

```rust
pub struct ToolCall {
    pub tool: String,
    pub parameters: std::collections::HashMap<String, serde_json::Value>,
    // Remove id field to match TypeScript interface
}
```

### Fix 2: Add Error Field to ToolResult

```rust
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>, // Add this field
}
```

### Fix 3: Implement File Path Extraction

```rust
let modified_files: Vec<String> = turn.tool_calls
    .iter()
    .filter(|call| call.tool == "Edit" || call.tool == "Write")
    .filter_map(|call| {
        call.parameters.get("file_path")
            .and_then(|v| v.as_str())
            .map(|path| path.split('/').last().unwrap_or(path).to_string())
    })
    .collect();
```

### Fix 4: Update Anchor Descriptions with File Context

```rust
let file_list = if !modified_files.is_empty() {
    format!(" in {}", modified_files.join(", "))
} else {
    String::new()
};

description: format!("Build error fixed{} and tests now pass", file_list),
```

---

## Priority

**High Priority**: Issues 1, 2, and 4 are critical for proper anchor point detection functionality.

**Medium Priority**: Issue 3 affects error handling but doesn't break core functionality.

---

## Testing Recommendations

After implementing fixes:

1. Test anchor point detection with file modifications across multiple files
2. Verify anchor descriptions include correct file names
3. Test error resolution patterns with tool execution failures
4. Compare anchor point quality between TypeScript and Rust implementations

---

*Analysis completed: Comparison between `/Users/rquast/projects/fspec/codelet/core/src/compaction.rs` and `~/projects/codelet/src/agent/anchor-point-compaction.ts`*