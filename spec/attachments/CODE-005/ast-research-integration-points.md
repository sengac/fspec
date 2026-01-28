# AST Research: Integration Points for CODE-005

## Research Commands Used
```bash
fspec research --tool=ast --pattern="async fn call" --lang=rust --path=codelet/tools/src/fspec.rs
fspec research --tool=ast --pattern="call_fspec_command" --lang=rust --path=codelet/napi/
fspec research --tool=ast --pattern="function $NAME" --lang=typescript --path=src/commands/list-work-units.ts
```

## Key Findings

### 1. FspecTool Tool::call() Method Location
**File:** `codelet/tools/src/fspec.rs:126`
**Pattern:** `async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error>`

**Current Status:** Contains TODO placeholder that needs replacement:
```rust
// TODO: This needs to be connected to the actual NAPI callFspecCommand function
```

**Integration Point:** This is where we need to wire up the NAPI callFspecCommand function.

### 2. NAPI Function Location  
**File:** `codelet/napi/src/fspec.rs:11`
**Pattern:** `call_fspec_command`

**Current Status:** NAPI bridge function exists and ready for integration
**Function Signature:** Takes command, args_json, project_root, and callback

### 3. TypeScript Command Structure
**File:** `src/commands/list-work-units.ts`
**Functions Found:** 3 functions at lines 28, 76, 119

**Key Insight:** TypeScript commands are exported as named functions (like `listWorkUnits`) that need to be dynamically imported by the TypeScript callback.

## Integration Architecture

### Current Flow (Broken - TODO)
```
Agent -> FspecTool.call() -> TODO placeholder -> Error response
```

### Target Flow (After Integration)
```
Agent -> FspecTool.call() -> callFspecCommand(NAPI) -> TypeScript callback -> import src/commands/X.ts -> Execute function -> Return JSON + system reminders
```

## Implementation Requirements

1. **Replace TODO in Tool::call():**
   - Remove placeholder JSON response
   - Add call to NAPI `call_fspec_command` function
   - Create TypeScript callback that maps commands to module imports

2. **TypeScript Callback Logic:**
   - Parse command name (e.g., 'list-work-units' -> 'listWorkUnits')
   - Dynamic import from `src/commands/${commandFile}.ts`
   - Execute function with parsed args
   - Capture system reminders from console.error()
   - Return JSON response

3. **Error Handling:**
   - Convert TypeScript errors to ToolError format
   - Preserve original fspec error context
   - Handle unsupported commands (bootstrap, init)

4. **System Reminder Capture:**
   - Parse `<system-reminder>` tags from stderr/console output
   - Include in tool response for LLM workflow orchestration

## Next Steps for Implementation

1. Modify `codelet/tools/src/fspec.rs` Tool::call() method
2. Add NAPI integration with TypeScript callback
3. Implement command name mapping logic
4. Add system reminder parsing
5. Test with core commands (list-work-units, create-story)
6. Validate error handling and performance improvements