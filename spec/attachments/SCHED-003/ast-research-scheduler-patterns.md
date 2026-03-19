# AST Research: Scheduler Engine Patterns — SCHED-003

## 1. tokio::spawn Usage in NAPI Module

Found 3 `tokio::spawn(async move { ... })` patterns in `session_manager.rs`:
- Line 3229: Session creation spawns background agent loop
- Line 3411: Compaction task spawn  
- Line 4400: Message queue processing

## 2. Reaper Pattern (Reference Architecture)

`codelet/tools/src/unified_exec/reaper.rs` — simple tokio loop pattern:
```rust
pub fn spawn_reaper(session_id: String) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            // Check conditions, break when done
        }
    });
}
```

Key: Uses `tokio::time::sleep` with a 2-second interval. Scheduler will use `tokio::time::interval` with 30-second ticks instead.

## 3. SessionManager Struct (Line 3061)

```
pub struct SessionManager {
    sessions: IndexMap<String, Arc<RwLock<BackgroundSession>>>,
    // ... other fields
}
```

Integration point: Add `scheduler_handle: RwLock<Option<JoinHandle<()>>>` field.

## 4. Work Units Watcher Pattern

`codelet/napi/src/work_units_watcher.rs` — file-watcher pattern with NAPI callbacks:
- Uses `lazy_static!` for global state
- `Arc<RwLock<>>` for thread-safe state
- `ThreadsafeFunction<StreamChunk>` for TS callbacks
- `#[napi]` exports for start/stop operations

## 5. Existing Dependencies

- ✅ `chrono` — workspace dependency with serde feature, `Utc::now()` used throughout
- ❌ `croner` — NOT present, need to add for cron expression parsing
- ❌ `chrono-tz` — NOT present, need to add for IANA timezone support

## 6. NAPI Struct Patterns

Found 90+ `pub struct` definitions in `codelet/napi/src/`. Key patterns:
- `#[napi(object)]` for JS-visible structs
- `pub(crate)` for internal-only structs
- Serde derives for serialization

## 7. Schedule Types (from SCHED-002)

`src/types/schedule.ts` defines TypeScript interfaces:
- `AgentScheduleEntry` / `ShellScheduleEntry`
- Cron, timezone, overlap_policy fields
- `last_run_at`, `last_run_status` tracking

## Architecture Decision

Scheduler module at `codelet/napi/src/scheduler/`:
- `mod.rs` — module root, re-exports
- `engine.rs` — core loop with `tokio::time::interval(Duration::from_secs(30))`
- `types.rs` — Rust-side schedule types matching TS types
- Follow reaper pattern but with interval instead of sleep
- Integrate into SessionManager with JoinHandle tracking
