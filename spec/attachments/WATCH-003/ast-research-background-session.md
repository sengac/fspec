# AST Research: BackgroundSession for Broadcast Channel Integration

## Search: BackgroundSession struct location
```
fspec research --tool=ast --pattern="pub struct BackgroundSession" --lang=rust --path=codelet/napi/src/
```

### Result:
```
codelet/napi/src/session_manager.rs:106:1:pub struct BackgroundSession {
```

## Analysis

### Current BackgroundSession fields (lines 106-150):
- `id: Uuid` - Session identifier
- `name: RwLock<String>` - Session name
- `project: String` - Project path
- `provider_id: RwLock<Option<String>>` - Provider ID
- `model_id: RwLock<Option<String>>` - Model ID
- `cached_input_tokens: AtomicU32` - Token counts
- `cached_output_tokens: AtomicU32` - Token counts
- `inner: Arc<Mutex<codelet_cli::session::Session>>` - Inner session
- `status: AtomicU8` - Session status
- `is_attached: AtomicBool` - Whether UI attached
- `input_tx: mpsc::Sender<PromptInput>` - Input channel
- `output_buffer: RwLock<Vec<StreamChunk>>` - Buffered output
- `attached_callback: RwLock<Option<ThreadsafeFunction<StreamChunk>>>` - UI callback
- `is_interrupted: Arc<AtomicBool>` - Interrupt flag
- `interrupt_notify: Arc<Notify>` - Interrupt notify
- `is_debug_enabled: AtomicBool` - Debug flag
- `pending_input: RwLock<Option<String>>` - Pending input

### handle_output() method (lines 239-255):
```rust
pub fn handle_output(&self, chunk: StreamChunk) {
    // Always buffer (unbounded)
    {
        let mut buffer = self.output_buffer.write().expect("output buffer lock poisoned");
        buffer.push(chunk.clone());
    }
    
    // If attached, forward to callback
    if self.is_attached() {
        if let Some(cb) = self.attached_callback.read().expect("callback lock poisoned").as_ref() {
            let _ = cb.call(Ok(chunk), ThreadsafeFunctionCallMode::NonBlocking);
        }
    }
}
```

### Integration Points:
1. Add `watcher_broadcast: broadcast::Sender<StreamChunk>` field after `attached_callback`
2. Initialize in `BackgroundSession::new()` with `broadcast::channel(256)`
3. Add `subscribe_to_stream()` method returning `self.watcher_broadcast.subscribe()`
4. Modify `handle_output()` to call `let _ = self.watcher_broadcast.send(chunk.clone());` after buffering

### Dependencies:
- `tokio::sync::broadcast` - Already available via tokio dependency
- `StreamChunk` - Already implements Clone (line 244 shows chunk.clone())
