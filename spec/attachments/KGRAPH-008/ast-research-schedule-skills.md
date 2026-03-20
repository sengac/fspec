# AST Research: Skills File Parsing & Schedule Integration

## Existing Schedule tool (codelet/napi/src/scheduler/)
- Schedule engine with cron-based job execution
- Agent jobs and shell jobs supported
- Schedule tool registered in tool handler map

## Skills file format
- Markdown with embedded JSON code blocks
- Pattern: ```json ... ``` fenced blocks

## IndexState (from merge.rs)
- Already has `sessions: HashMap<String, SessionWatermark>`
- `SessionWatermark.last_indexed_turn: u32`
- Can be reused for incremental indexing watermark tracking
