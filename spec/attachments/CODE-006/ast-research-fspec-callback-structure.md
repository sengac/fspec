# AST Research: fspec-callback.ts Structure Analysis for CODE-006

## Research Command
```bash
ast-grep --pattern "function $NAME" --lang typescript src/utils/fspec-callback.ts
ast-grep --pattern "JSON.stringify($ARG)" --lang typescript src/utils/fspec-callback.ts
ast-grep --pattern "console.error($$$ARGS)" --lang typescript src/utils/fspec-callback.ts
```

## Key Findings

### 1. Main Function Structure
**Pattern:** `function fspecCallback`
**Location:** `src/utils/fspec-callback.ts:6:8`

The main callback function is structured to handle command routing and return JSON responses.

### 2. JSON Response Points (8 locations)
**Pattern:** `JSON.stringify($ARG)`
**Locations:**
- Line 23: Main response structure
- Line 32: Empty workUnits response
- Line 37, 45, 54, 64, 76: Various command responses  
- Line 88: Error response

**Key Insight:** All responses go through JSON.stringify, making it the perfect injection point for adding systemReminders field.

### 3. No console.error Usage
**Pattern:** `console.error($$$ARGS)`  
**Result:** No matches found

**Critical Finding:** The callback currently doesn't capture or emit any console.error output, which is needed for system reminder preservation.

## Integration Points for CODE-006

### 1. Response Enhancement Location
**Target:** All `JSON.stringify({...})` calls need to include systemReminders field
**Approach:** Create wrapper function that adds systemReminders to all responses

### 2. stderr Capture Implementation
**Target:** Function body of fspecCallback  
**Approach:** Override process.stderr.write during command execution to capture console.error output

### 3. System Reminder Parsing
**Target:** Between command execution and JSON.stringify calls
**Approach:** Parse captured stderr for both result.systemReminder and <system-reminder> tags

## Implementation Strategy

1. **Wrap stderr capture around command execution**
2. **Parse captured output for system reminders** 
3. **Enhance all JSON responses with systemReminders field**
4. **Maintain backward compatibility with existing response structure**

This analysis confirms the fspecCallback is the correct integration point for system reminder preservation.