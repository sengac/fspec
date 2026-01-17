# NAPI-009: Background Session Management Design

## Overview

Enable AI agent sessions to run in the background with attach/detach capability, similar to tmux/screen for terminal sessions.

## Domain Model (Event Storming)

### Bounded Context: Session Management

**Aggregates:**
- `BackgroundSession` - Core aggregate owning session state and agent execution

**Commands:**
- `CreateSession` → spawns background agent task
- `AttachToSession` → subscribes UI to output stream
- `DetachFromSession` → unsubscribes UI, session continues
- `SendInput` → queues prompt for background agent
- `InterruptSession` → stops current agent execution
- `DestroySession` → terminates background task

**Domain Events:**
- `SessionCreated` - new background session spawned
- `SessionAttached` - UI subscribed to session output
- `SessionDetached` - UI unsubscribed, session continues running
- `InputReceived` - prompt queued for processing
- `OutputProduced` - agent generated text/tool call/result
- `SessionCompleted` - agent finished processing current prompt
- `SessionDestroyed` - background task terminated

**Policies:**
- WHEN `OutputProduced` THEN buffer output AND if attached, forward to UI
- WHEN `SessionDetached` THEN continue running, buffer all output
- WHEN `SessionAttached` THEN replay buffered output, then stream live

## Architecture

```
┌─────────────────────────────────────────────────────┐
│              SessionManager (Singleton)              │
│  - sessions: HashMap<Uuid, Arc<BackgroundSession>>  │
│  - tokio runtime (multi-threaded)                   │
└──────────────────────┬──────────────────────────────┘
                       │
        ┌──────────────┼──────────────┐
        ▼              ▼              ▼
   ┌─────────┐    ┌─────────┐    ┌─────────┐
   │Session A│    │Session B│    │Session C│
   │(attached)│   │(detached)│   │(detached)│
   └─────────┘    └─────────┘    └─────────┘
        │              │              │
        ▼         (running)      (running)
   [AgentView]
```

## Key Components

### 1. SessionManager (Rust)
```rust
lazy_static! {
    static ref SESSION_MANAGER: SessionManager = SessionManager::new();
}

pub struct SessionManager {
    sessions: RwLock<HashMap<Uuid, Arc<BackgroundSession>>>,
}
```

### 2. BackgroundSession (Rust)
```rust
pub struct BackgroundSession {
    id: Uuid,
    inner: Mutex<Session>,
    status: AtomicU8,  // Idle, Running, Interrupted
    
    // Communication
    input_tx: mpsc::Sender<String>,
    output_buffer: RwLock<VecDeque<StreamChunk>>,
    attached_callback: RwLock<Option<ThreadsafeFunction>>,
    
    // Interrupt
    is_interrupted: Arc<AtomicBool>,
    interrupt_notify: Arc<Notify>,
}
```

### 3. NAPI Bindings
```typescript
// Lifecycle
sessionCreate(model: string, project: string): string;
sessionDestroy(sessionId: string): void;
sessionList(): SessionInfo[];

// Attach/Detach
sessionAttach(sessionId: string, callback: (chunk: StreamChunk) => void): void;
sessionDetach(sessionId: string): void;

// Interaction
sessionSendInput(sessionId: string, input: string): void;
sessionInterrupt(sessionId: string): void;
sessionGetStatus(sessionId: string): SessionStatus;
sessionGetBufferedOutput(sessionId: string, limit: number): StreamChunk[];
```

### 4. AgentView.tsx Changes
- Remove direct `CodeletSession` ownership
- Add session selector UI (list running sessions)
- Attach/detach via NAPI calls
- Hydrate conversation from buffered output on attach

## SOLID Principles Applied

- **S**: SessionManager only manages session lifecycle; BackgroundSession only handles single session
- **O**: New session types can extend BackgroundSession without modifying core
- **L**: All sessions implement same interface regardless of attached state
- **I**: Separate interfaces for lifecycle (create/destroy) vs interaction (attach/send)
- **D**: AgentView depends on NAPI abstractions, not concrete Rust types

## Implementation Phases

1. **Phase 1**: SessionManager + BackgroundSession in Rust
2. **Phase 2**: NAPI bindings for all operations
3. **Phase 3**: AgentView refactor to use attach/detach
4. **Phase 4**: Session list UI and status indicators
