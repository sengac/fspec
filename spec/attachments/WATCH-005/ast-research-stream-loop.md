# AST Research: Stream Loop Architecture for Watcher Agent

## Search: Existing agent stream functions
```
fspec research --tool=ast --pattern="pub.*fn.*stream|pub.*fn.*loop" --lang=rust --path=codelet/cli/src/interactive
```

### Result:
```
codelet/cli/src/interactive/stream_loop.rs:74:pub(super) async fn run_agent_stream_with_interruption
codelet/cli/src/interactive/stream_loop.rs:108:pub async fn run_agent_stream
codelet/cli/src/interactive/repl_loop.rs:16:pub(super) async fn repl_loop
```

## Key Patterns Found:

1. **Dual-mode support**: CLI mode uses event_stream, NAPI mode uses is_interrupted flag
2. **tokio::select!** is used to multiplex between:
   - `stream.next()` - agent response chunks
   - `es.next()` - TUI events (Esc key)
   - `si.tick()` - status updates
   - `interrupt_fut` - NAPI interrupt notification

3. **run_agent_stream** signature (NAPI entry point):
   ```rust
   pub async fn run_agent_stream<M, O>(
       agent: RigAgent<M>,
       prompt: &str,
       session: &mut Session,
       is_interrupted: Arc<AtomicBool>,
       interrupt_notify: Arc<Notify>,
       output: &O,
   ) -> Result<()>
   ```

4. **StreamChunk types** to observe:
   - Text, Thinking - accumulate
   - ToolUse - tool execution started
   - ToolResult - natural breakpoint
   - TurnComplete - natural breakpoint
   - UserInput - user sent input

## Watcher Loop Design:

The watcher loop should use a similar pattern but select! on:
1. `user_input_rx.recv()` - user prompts (priority)
2. `parent_broadcast_rx.recv()` - parent observations  
3. `silence_timeout.tick()` - configurable timeout

When a breakpoint is detected, format an evaluation prompt and run the agent.
