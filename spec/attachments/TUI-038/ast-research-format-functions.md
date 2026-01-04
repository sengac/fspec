# AST Research: TUI-038 Diff View for Write/Edit Tool Output

## Key Locations Found

### Format Functions (AgentModal.tsx)
```
src/tui/components/AgentModal.tsx:406:1:const formatCollapsedOutput
src/tui/components/AgentModal.tsx:395:1:const formatWithTreeConnectors
```

These are the primary formatting functions that need to be extended for diff coloring.

### Tool Result Handling (AgentModal.tsx)
```
src/tui/components/AgentModal.tsx:2377:23:toolResult
src/tui/components/AgentModal.tsx:2378:21:toolResult
src/tui/components/AgentModal.tsx:2379:44:toolResult
```

Tool result processing occurs in the conversation lines memo around line 2377.

### Diff Parser (diff-parser.ts)
The parseDiff function and DiffLine interface are in src/git/diff-parser.ts.
Used by FileDiffViewer for git change visualization.

## Implementation Plan

1. **New Function: formatDiffOutput**
   - Detect Edit/Write tool results
   - For Edit: parse old_string and new_string to generate diff
   - For Write: treat all lines as additions
   - Apply colored formatting with backgroundColor

2. **Color Constants**
   - Removed: #8B0000 (dark red)
   - Added: #006400 (dark green)

3. **Integration Point**
   - Modify tool result formatting in conversationLines memo
   - Check tool name (Edit/Write) and apply diff formatting

4. **Maintain Existing Patterns**
   - Tree connector (L prefix on first line)
   - Collapse long output with expand indicator
