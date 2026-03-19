# SCHED-009: AST Research — Schedule Tool Patterns

## Handler-Delegated Tool Pattern (4 layers)

### Layer 1: Tool Definition (codelet-tools)
- ScheduleTool struct with session_id: Uuid
- impl Tool for ScheduleTool — call() delegates to execute_schedule_command()
- ScheduleArgs: action, name, cron, timezone, job_type, role, prompt, command, overlap_policy

### Layer 2: Handler Registry (codelet-tools)
- ScheduleHandler type alias: Arc<dyn Fn(ScheduleRequest) -> ScheduleResult + Send + Sync>
- static SCHEDULE_HANDLERS: Lazy<RwLock<HashMap<Uuid, ScheduleHandler>>>
- Functions: set_schedule_handler, execute_schedule_command, has_schedule_handler, clear_all

### Layer 3: Handler Implementation (codelet-napi)
- codelet/napi/src/schedule_handler.rs — create_handler(project: String) -> ScheduleHandler
- Handler closure reads/writes spec/schedules.json, dispatches on action

### Layer 4: Registration (session_manager.rs + providers)
- Setup: set_schedule_handler(session.id, Some(handler)) before agent run
- Teardown: set_schedule_handler(session.id, None) after agent run

## Provider Registration Points

| Provider | Import line | Tool chain line |
|----------|------------|-----------------|
| claude.rs | 502-506 | 535 |
| openai.rs | 312-316 | 343 |
| gemini.rs | 114 | 176 |
| zai.rs | 203 | 251 |

## lib.rs Exports
- Module declaration: after line 39
- Re-export: after line 158

## Facade Decision
- If uniform schema across providers → no facade needed, direct .tool() registration
- If provider-specific schemas → add to codelet/tools/src/facade/registry.rs after line 58
