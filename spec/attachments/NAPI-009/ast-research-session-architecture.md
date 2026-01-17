# AST Research for NAPI-009: Background Session Management

## Research Date
2026-01-17

## Purpose
Analyze existing codebase structure to understand integration points for background session management.

## Findings

### 1. Rust NAPI Structs (codelet/napi/src/)

Key structs that will be extended or integrated with:

```
codelet/napi/src/session.rs:36 - pub struct CodeletSession
codelet/napi/src/types.rs:118 - pub struct StreamChunk
codelet/napi/src/output.rs:80 - pub struct NapiOutput
codelet/napi/src/persistence/types.rs:129 - pub struct SessionManifest
codelet/napi/src/persistence/storage.rs:172 - pub struct SessionStore
```

### 2. Existing Singleton Patterns (lazy_static)

```
codelet/napi/src/lib.rs:58 - TOKIO_RUNTIME singleton
codelet/napi/src/persistence/mod.rs:43 - MESSAGE_STORE, SESSION_STORE, BLOB_STORE, HISTORY_STORE singletons
```

New `SESSION_MANAGER` will follow this established pattern.

### 3. CodeletSession Implementation

Location: `codelet/napi/src/session.rs:47`

Current structure:
- `inner: Arc<Mutex<Session>>` - holds CLI session
- `is_interrupted: Arc<AtomicBool>` - interrupt flag
- `interrupt_notify: Arc<Notify>` - tokio notify for immediate wake

This will be wrapped by `BackgroundSession` which adds:
- Input channel (mpsc)
- Output buffer (VecDeque)
- Attached callback (ThreadsafeFunction)

### 4. AgentView.tsx

Location: `src/tui/components/AgentView.tsx:593`

Export: `export const AgentView: React.FC<AgentViewProps>`

Current session ownership:
- Line ~595: `const [session, setSession] = useState<CodeletSessionType | null>(null)`

Will be refactored to use session ID and NAPI bindings instead of direct ownership.

### 5. Persistence Integration

Existing stores in `codelet/napi/src/persistence/mod.rs`:
- `SessionStore` - manages session manifests
- `MessageStore` - stores message content
- `BlobStore` - large content storage

Background sessions will use these existing stores for persistence.

## Integration Points

1. **New file**: `codelet/napi/src/session_manager.rs` - SessionManager singleton
2. **Modify**: `codelet/napi/src/lib.rs` - expose new NAPI bindings
3. **Modify**: `codelet/napi/src/session.rs` - BackgroundSession wraps CodeletSession
4. **Modify**: `src/tui/components/AgentView.tsx` - use session bindings instead of direct ownership

## Conclusion

The codebase already has established patterns for:
- Singletons via lazy_static
- Async session management with Arc<Mutex<>>
- Interrupt handling with AtomicBool + Notify
- Persistence with multiple stores

Background session management will follow these patterns and integrate with existing infrastructure.
