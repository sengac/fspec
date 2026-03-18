# SCHED-002: Schedule Persistence & Schema — Implementation Guide

## Overview

Define and implement `spec/schedules.json` as the single file that stores both schedule definitions and runtime state (last-run timestamps). Follow the established fspec pattern for project-level JSON config files.

## File Schema

```json
{
  "version": "1.0.0",
  "schedules": {
    "nightly-review": {
      "name": "nightly-review",
      "cron": "0 2 * * *",
      "timezone": "Australia/Brisbane",
      "jobType": "agent",
      "role": "Code reviewer",
      "prompt": "Review all open PRs and summarize findings",
      "overlapPolicy": "skip",
      "status": "active",
      "lastRunAt": "2026-03-17T16:00:00.000Z",
      "lastRunStatus": "completed",
      "createdAt": "2026-03-10T00:00:00.000Z"
    },
    "daily-sync": {
      "name": "daily-sync",
      "cron": "0 9 * * 1-5",
      "timezone": "UTC",
      "jobType": "shell",
      "command": "npm run sync",
      "overlapPolicy": "skip",
      "status": "active",
      "lastRunAt": null,
      "lastRunStatus": null,
      "createdAt": "2026-03-10T00:00:00.000Z"
    }
  }
}
```

### Schedule Entry Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Unique schedule identifier |
| `cron` | string | Yes | Cron expression (5-field standard) |
| `timezone` | string | Yes | IANA timezone (e.g., `Australia/Brisbane`, `UTC`) |
| `jobType` | `"agent"` \| `"shell"` | Yes | Job type discriminant |
| `role` | string | Agent only | Agent session role |
| `prompt` | string | Agent only | Initial prompt sent to agent |
| `command` | string | Shell only | Shell command to execute |
| `overlapPolicy` | `"skip"` \| `"queue"` | Yes | What to do if previous run is still active |
| `status` | `"active"` \| `"paused"` | Yes | Whether the schedule triggers |
| `lastRunAt` | ISO8601 \| null | No | Timestamp of last completed run |
| `lastRunStatus` | `"completed"` \| `"failed"` \| `"skipped"` \| null | No | Status of last run |
| `createdAt` | ISO8601 | Yes | Creation timestamp |

## Existing Patterns to Follow

### File Management — `LockedFileManager`

All spec/ JSON files use the `LockedFileManager` from `src/utils/file-manager.ts`. Two key operations:

1. **`readJSON<T>(filePath, defaultData)`** — Read with auto-creation if missing
2. **`transaction<T>(filePath, callback)`** — Exclusive read-modify-write

```typescript
// Pattern from ensure-files.ts
export async function ensureSchedulesFile(cwd: string): Promise<Schedules> {
  const specPath = await findOrCreateSpecDirectory(cwd);
  const filePath = join(specPath, 'schedules.json');
  const initialData: Schedules = { version: '1.0.0', schedules: {} };
  return await fileManager.readJSON(filePath, initialData);
}
```

### Schema Validation — Ajv

Follow the pattern from `src/validators/json-schema.ts`:

```typescript
import Ajv from 'ajv';
import addFormats from 'ajv-formats';
import scheduleSchema from '../schemas/schedule.schema.json';

const ajv = new Ajv({ allErrors: true });
addFormats(ajv);
const validate = ajv.compile(scheduleSchema);
```

Create `src/schemas/schedule.schema.json` with the full JSON Schema definition.

### Cron Expression Validation

Validate cron expressions at write time, not just at evaluation time. The TypeScript layer should validate the cron expression is syntactically correct before persisting. Consider using `cron-validator` or a simple regex for 5-field cron syntax.

### Timezone Validation

Validate timezone strings against the IANA timezone database. Use `Intl.supportedValuesOf('timeZone')` (available in Node.js 18+) to get the list of valid timezones.

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `src/schemas/schedule.schema.json` | Create | JSON Schema for schedules.json |
| `src/types/schedule.ts` | Create | TypeScript interfaces for schedule entries |
| `src/utils/ensure-files.ts` | Modify | Add `ensureSchedulesFile()` |
| `src/validators/schedule-validator.ts` | Create | Cron and timezone validation |
| `src/commands/schedule/` | Create | Directory for schedule CRUD commands |
| `src/commands/schedule/add-schedule.ts` | Create | Add schedule (agent or shell) |
| `src/commands/schedule/remove-schedule.ts` | Create | Remove schedule |
| `src/commands/schedule/pause-schedule.ts` | Create | Pause/resume schedule |
| `src/commands/schedule/list-schedules.ts` | Create | List all schedules |

## Key Constraints

- File uses `LockedFileManager.transaction()` for all writes — concurrent fspec instances must not corrupt the file
- Cron expressions must be valid 5-field standard cron syntax
- Timezone must be a valid IANA timezone string
- Schedule names must be unique within the file
- Schedule names should follow slug format (lowercase, hyphens, no spaces)
- The `version` field enables future migrations using the same pattern as `work-units.json`

## Rust Side

The scheduler engine (SCHED-003) reads this file from the Rust layer. Define matching Rust structs:

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct SchedulesFile {
    pub version: String,
    pub schedules: HashMap<String, ScheduleEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScheduleEntry {
    pub name: String,
    pub cron: String,
    pub timezone: String,
    pub job_type: JobType,
    // ... etc
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "jobType")]
pub enum JobType {
    #[serde(rename = "agent")]
    Agent { role: String, prompt: String },
    #[serde(rename = "shell")]
    Shell { command: String },
}
```

The Rust side reads the file directly via `tokio::fs::read_to_string` + `serde_json::from_str`. It does NOT need LockedFileManager (that's TypeScript-only). For timestamp updates from Rust, consider using a lightweight file lock or coordinating via the N-API bridge.
