# AST Research: SessionSearch Turn Range — Code Points to Modify

## SessionSearchAction Enum (types.rs:16)
- `Show` variant (lines 49-60): Add `start_turn: Option<usize>` and `end_turn: Option<usize>`
- `Search` variant (lines 23-48): Add `start_turn: Option<usize>` and `end_turn: Option<usize>`

## Tool Definition Schema (mod.rs:68-141)
- `definition()` method returns JSON schema — add `start_turn` and `end_turn` params

## Handler Functions (session_search_handler.rs)
- `handle_show()` (line 283): Add start_turn/end_turn params, filter messages before max_turns
- `handle_search()` (line 128): Add start_turn/end_turn params, filter in message iteration loop
- `create_handler()` (line 38): Pass start_turn/end_turn through to handle_show and handle_search

## Existing Infrastructure
- `resolve_message_content()` — already handles blob resolution (line 446)
- `ConditionalTrimmer` — stateful tool correlation, must process all messages in order (line 691)
- `build_context_turns()` — context around matches, should work with filtered turn ranges (line 610)
