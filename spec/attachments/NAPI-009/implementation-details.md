# NAPI-009: Implementation Details

## Agent Loop Design

The background agent runs in a tokio task with this loop:

```rust
async fn agent_loop(session: Arc<BackgroundSession>) {
    loop {
        // 1. Wait for input (blocks until prompt received)
        let input = match session.input_rx.recv().await {
            Some(input) => input,
            None => break, // Channel closed, exit loop
        };
        
        // 2. Set status to Running
        session.status.store(Status::Running);
        session.is_interrupted.store(false);
        
        // 3. Run agent stream with output handler
        let result = run_agent_stream(
            &mut session.inner.lock().await,
            &input,
            session.is_interrupted.clone(),
            session.interrupt_notify.clone(),
            |chunk| session.handle_output(chunk),
        ).await;
        
        // 4. Set status back to Idle
        session.status.store(Status::Idle);
        
        // 5. Persist state
        session.persist_to_disk();
    }
}
```

## Output Handling

```rust
impl BackgroundSession {
    fn handle_output(&self, chunk: StreamChunk) {
        // Always buffer (ring buffer, keep last N chunks)
        let mut buffer = self.output_buffer.write();
        if buffer.len() >= MAX_BUFFER_SIZE {
            buffer.pop_front();
        }
        buffer.push_back(chunk.clone());
        
        // If attached, also send to callback
        if let Some(cb) = self.attached_callback.read().as_ref() {
            cb.call(chunk, ThreadsafeFunctionCallMode::NonBlocking);
        }
    }
}
```

## Attach Flow

```
User calls sessionAttach(id, callback)
           │
           ▼
┌─────────────────────────────┐
│ 1. Store callback reference │
│ 2. Get buffered output      │
│ 3. Replay buffer to callback│
│ 4. Set is_attached = true   │
└─────────────────────────────┘
           │
           ▼
    Live streaming begins
    (new chunks go to callback)
```

## Detach Flow

```
User calls sessionDetach(id)
           │
           ▼
┌─────────────────────────────┐
│ 1. Set is_attached = false  │
│ 2. Clear callback reference │
│ 3. Session keeps running    │
│ 4. Output continues to buffer│
└─────────────────────────────┘
```

## State Persistence

Each session persists to existing persistence system:

- **On input**: Store user message envelope
- **On output complete**: Store assistant message envelope  
- **On attach**: Load from persistence if buffer empty
- **Session manifest**: Track running/idle status

## Thread Safety

| Component | Synchronization |
|-----------|-----------------|
| `sessions` HashMap | `RwLock` (read-heavy) |
| `inner` Session | `Mutex` (exclusive access during prompt) |
| `output_buffer` | `RwLock` (write from agent, read on attach) |
| `attached_callback` | `RwLock` (rarely changes) |
| `status` | `AtomicU8` (lock-free) |
| `is_interrupted` | `AtomicBool` (lock-free) |

## Error Handling

- **Agent error**: Log, set status to Idle, wait for next input
- **Channel closed**: Exit agent loop, mark session as terminated
- **Callback error**: Log warning, continue buffering
- **Persistence error**: Log error, continue (non-fatal)

## Memory Limits

- Buffer size: 10,000 chunks (~10MB estimated)
- Max sessions: 10 concurrent (configurable)
- Chunk TTL: None (buffer is ring, old chunks evicted)
