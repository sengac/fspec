# TOOL-020: Command Mapping — `_` Positional Args → Named Keys

## Purpose

This document maps every `_` positional argument pattern in `fspec_workflow_guidance.rs` to the correct named-key format expected by Rust fspec-core commands.

## Mapping Table

### Work Unit Management

| Command | Guidance `_` Format | Rust Named Keys |
|---------|---------------------|-----------------|
| `show-work-unit` | `{"_": ["AUTH-001"]}` | `{"workUnitId": "AUTH-001"}` |
| `update-work-unit-status` | `{"_": ["AUTH-001", "specifying"]}` | `{"workUnitId": "AUTH-001", "status": "specifying"}` |
| `update-work-unit` | `{"_": ["AUTH-001"], "title": "..."}` | `{"workUnitId": "AUTH-001", "title": "..."}` |
| `update-work-unit-estimate` | `{"_": ["AUTH-001", "5"]}` | `{"workUnitId": "AUTH-001", "estimate": "5"}` |
| `delete-work-unit` | `{"_": ["AUTH-001"]}` | `{"workUnitId": "AUTH-001"}` |
| `prioritize-work-unit` | `{"_": ["AUTH-003"], "position": "top"}` | `{"workUnitId": "AUTH-003", "position": "top"}` |
| `compact-work-unit` | `{"_": ["AUTH-001"]}` | `{"workUnitId": "AUTH-001"}` |
| `show-deleted` | `{"_": ["AUTH-001"]}` | `{"workUnitId": "AUTH-001"}` |
| `workflow-automation` | `{"_": ["AUTH-001"]}` | `{"workUnitId": "AUTH-001"}` |

### Work Unit Creation

| Command | Guidance `_` Format | Rust Named Keys |
|---------|---------------------|-----------------|
| `create-story` | `{"_": ["AUTH", "User Login"]}` | `{"prefix": "AUTH", "title": "User Login"}` |
| `create-bug` | `{"_": ["AUTH", "Login fails"]}` | `{"prefix": "AUTH", "title": "Login fails"}` |
| `create-task` | `{"_": ["INFRA", "Setup CI"]}` | `{"prefix": "INFRA", "title": "Setup CI"}` |

### Example Mapping

| Command | Guidance `_` Format | Rust Named Keys |
|---------|---------------------|-----------------|
| `set-user-story` | `{"_": ["AUTH-001"], "role": "..."}` | `{"workUnitId": "AUTH-001", "role": "..."}` |
| `add-rule` | `{"_": ["AUTH-001", "rule text"]}` | `{"workUnitId": "AUTH-001", "rule": "rule text"}` |
| `remove-rule` | `{"_": ["AUTH-001", "0"]}` | `{"workUnitId": "AUTH-001", "id": "0"}` |
| `restore-rule` | `{"_": ["AUTH-001", "2"]}` | `{"workUnitId": "AUTH-001", "id": "2"}` |
| `add-example` | `{"_": ["AUTH-001", "example text"]}` | `{"workUnitId": "AUTH-001", "example": "example text"}` |
| `remove-example` | `{"_": ["AUTH-001", "0"]}` | `{"workUnitId": "AUTH-001", "id": "0"}` |
| `restore-example` | `{"_": ["AUTH-001", "3"]}` | `{"workUnitId": "AUTH-001", "id": "3"}` |
| `add-question` | `{"_": ["AUTH-001", "question"]}` | `{"workUnitId": "AUTH-001", "question": "question"}` |
| `answer-question` | `{"_": ["AUTH-001", "0"], "answer": "..."}` | `{"workUnitId": "AUTH-001", "id": "0", "answer": "..."}` |
| `remove-question` | `{"_": ["AUTH-001", "0"]}` | `{"workUnitId": "AUTH-001", "id": "0"}` |
| `restore-question` | `{"_": ["AUTH-001", "0"]}` | `{"workUnitId": "AUTH-001", "id": "0"}` |
| `add-architecture-note` | `{"_": ["AUTH-001", "note"]}` | `{"workUnitId": "AUTH-001", "note": "note"}` |
| `remove-architecture-note` | `{"_": ["AUTH-001", "0"]}` | `{"workUnitId": "AUTH-001", "id": "0"}` |
| `restore-architecture-note` | `{"_": ["AUTH-001", "1"]}` | `{"workUnitId": "AUTH-001", "id": "1"}` |
| `add-assumption` | `{"_": ["AUTH-001", "assumption"]}` | `{"workUnitId": "AUTH-001", "assumption": "assumption"}` |

### Feature Management

| Command | Guidance `_` Format | Rust Named Keys |
|---------|---------------------|-----------------|
| `create-feature` | `{"_": ["User Authentication"]}` | `{"name": "User Authentication"}` |
| `add-scenario` | `{"_": ["feature", "scenario"]}` | `{"feature": "feature", "scenario": "scenario"}` |
| `update-scenario` | `{"_": ["feature", "old", "new"]}` | `{"feature": "feature", "oldName": "old", "newName": "new"}` |
| `delete-scenario` | `{"_": ["feature", "scenario"]}` | `{"feature": "feature", "scenario": "scenario"}` |
| `add-step` | `{"_": ["feature", "scenario", "given", "text"]}` | `{"feature": "feature", "scenario": "scenario", "keyword": "given", "text": "text"}` |
| `update-step` | `{"_": ["feature", "scenario", "old"], ...}` | `{"feature": "feature", "scenario": "scenario", "oldText": "old", ...}` |
| `delete-step` | `{"_": ["feature", "scenario", "text"]}` | `{"feature": "feature", "scenario": "scenario", "text": "text"}` |
| `add-background` | `{"_": ["feature", "text"]}` | `{"feature": "feature", "text": "text"}` |
| `add-architecture` | `{"_": ["feature", "text"]}` | `{"feature": "feature", "text": "text"}` |
| `show-feature` | `{"_": ["feature"]}` | `{"feature": "feature"}` |
| `generate-scenarios` | `{"_": ["AUTH-001"]}` | `{"workUnitId": "AUTH-001"}` |

### Feature Tags

| Command | Guidance `_` Format | Rust Named Keys |
|---------|---------------------|-----------------|
| `add-tag-to-feature` | `{"_": ["file", "@tag"]}` | `{"feature": "file", "tag": "@tag"}` |
| `remove-tag-from-feature` | `{"_": ["file", "@tag"]}` | `{"feature": "file", "tag": "@tag"}` |
| `list-feature-tags` | `{"_": ["file"]}` | `{"feature": "file"}` |
| `add-tag-to-scenario` | `{"_": ["feature", "scenario", "@tag"]}` | `{"feature": "feature", "scenario": "scenario", "tag": "@tag"}` |
| `remove-tag-from-scenario` | `{"_": ["feature", "scenario", "@tag"]}` | `{"feature": "feature", "scenario": "scenario", "tag": "@tag"}` |
| `list-scenario-tags` | `{"_": ["feature", "scenario"]}` | `{"feature": "feature", "scenario": "scenario"}` |

### Coverage

| Command | Guidance `_` Format | Rust Named Keys |
|---------|---------------------|-----------------|
| `link-coverage` | `{"_": ["feature"], ...}` | `{"feature": "feature", ...}` |
| `unlink-coverage` | `{"_": ["feature"], ...}` | `{"feature": "feature", ...}` |
| `show-coverage` | `{"_": ["feature"]}` | `{"feature": "feature"}` |
| `audit-coverage` | `{"_": ["feature"]}` | `{"feature": "feature"}` |

### Dependencies

| Command | Guidance `_` Format | Rust Named Keys |
|---------|---------------------|-----------------|
| `add-dependency` | `{"_": ["AUTH-002", "AUTH-001"]}` | `{"workUnitId": "AUTH-002", "dependsOn": "AUTH-001"}` |
| `add-dependencies` | `{"_": ["DASH-001", "AUTH-001", "AUTH-002"]}` | `{"workUnitId": "DASH-001", "dependsOn": ["AUTH-001", "AUTH-002"]}` |
| `remove-dependency` | `{"_": ["AUTH-002", "AUTH-001"]}` | `{"workUnitId": "AUTH-002", "target": "AUTH-001"}` |
| `clear-dependencies` | `{"_": ["AUTH-002"]}` | `{"workUnitId": "AUTH-002"}` |
| `dependencies` | `{"_": ["AUTH-002"]}` | `{"workUnitId": "AUTH-002"}` |

### Epic & Prefix Management

| Command | Guidance `_` Format | Rust Named Keys |
|---------|---------------------|-----------------|
| `create-epic` | `{"_": ["id", "prefix", "desc"]}` | `{"id": "id", "prefix": "prefix", "description": "desc"}` |
| `show-epic` | `{"_": ["id"]}` | `{"id": "id"}` |
| `delete-epic` | `{"_": ["id"]}` | `{"id": "id"}` |
| `create-prefix` | `{"_": ["AUTH", "desc"]}` | `{"prefix": "AUTH", "description": "desc"}` |
| `update-prefix` | `{"_": ["AUTH", "desc"]}` | `{"prefix": "AUTH", "description": "desc"}` |

### Foundation

| Command | Guidance `_` Format | Rust Named Keys |
|---------|---------------------|-----------------|
| `update-foundation` | `{"_": ["key", "value"]}` | `{"key": "key", "value": "value"}` |
| `add-capability` | `{"_": ["name", "desc"]}` | `{"capability": "name", "description": "desc"}` |
| `remove-capability` | `{"_": ["name"]}` | `{"capability": "name"}` |
| `add-persona` | `{"_": ["name"], ...}` | `{"name": "name", ...}` |
| `remove-persona` | `{"_": ["name"]}` | `{"name": "name"}` |
| `add-foundation-bounded-context` | `{"_": ["name"]}` | `{"name": "name"}` |
| `add-aggregate-to-foundation` | `{"_": ["context", "name"]}` | `{"context": "context", "aggregate": "name"}` |
| `add-domain-event-to-foundation` | `{"_": ["context", "event"]}` | `{"context": "context", "event": "event"}` |
| `add-command-to-foundation` | `{"_": ["context", "command"]}` | `{"context": "context", "command": "command"}` |

### Event Storm

| Command | Guidance `_` Format | Rust Named Keys |
|---------|---------------------|-----------------|
| `discover-event-storm` | `{"_": ["AUTH-001"]}` | `{"workUnitId": "AUTH-001"}` |
| `add-domain-event` | `{"_": ["AUTH-001", "event"]}` | `{"workUnitId": "AUTH-001", "event": "event"}` |
| `add-command` | `{"_": ["AUTH-001", "command"]}` | `{"workUnitId": "AUTH-001", "command": "command"}` |
| `add-policy` | `{"_": ["AUTH-001", "policy"], ...}` | `{"workUnitId": "AUTH-001", "policy": "policy", ...}` |
| `add-hotspot` | `{"_": ["AUTH-001", "hotspot"], ...}` | `{"workUnitId": "AUTH-001", "hotspot": "hotspot", ...}` |
| `add-aggregate` | `{"_": ["AUTH-001", "name"], ...}` | `{"workUnitId": "AUTH-001", "name": "name", ...}` |
| `add-bounded-context` | `{"_": ["AUTH-001", "name"]}` | `{"workUnitId": "AUTH-001", "name": "name"}` |
| `add-external-system` | `{"_": ["AUTH-001", "name"], ...}` | `{"workUnitId": "AUTH-001", "name": "name", ...}` |
| `show-event-storm` | `{"_": ["AUTH-001"]}` | `{"workUnitId": "AUTH-001"}` |
| `generate-example-mapping-from-event-storm` | `{"_": ["AUTH-001"]}` | `{"workUnitId": "AUTH-001"}` |

### Import/Export

| Command | Guidance `_` Format | Rust Named Keys |
|---------|---------------------|-----------------|
| `export-example-map` | `{"_": ["AUTH-001", "file"]}` | `{"workUnitId": "AUTH-001", "output": "file"}` |
| `import-example-map` | `{"_": ["AUTH-001", "file"]}` | `{"workUnitId": "AUTH-001", "input": "file"}` |

### Attachments

| Command | Guidance `_` Format | Rust Named Keys |
|---------|---------------------|-----------------|
| `add-attachment` | `{"_": ["AUTH-001", "file.png"]}` | `{"workUnitId": "AUTH-001", "filePath": "file.png"}` |
| `list-attachments` | `{"_": ["AUTH-001"]}` | `{"workUnitId": "AUTH-001"}` |
| `remove-attachment` | `{"_": ["AUTH-001", "file.png"]}` | `{"workUnitId": "AUTH-001", "filePath": "file.png"}` |

### Diagrams

| Command | Guidance `_` Format | Rust Named Keys |
|---------|---------------------|-----------------|
| `add-diagram` | `{"_": ["name", "title", "content"]}` | `{"category": "name", "title": "title", "content": "content"}` |
| `delete-diagram` | `{"_": ["name", "title"]}` | `{"category": "name", "title": "title"}` |

### Hooks

| Command | Guidance `_` Format | Rust Named Keys |
|---------|---------------------|-----------------|
| `add-hook` | `{"_": ["event", "name"], ...}` | `{"event": "event", "name": "name", ...}` |
| `remove-hook` | `{"_": ["event", "name"]}` | `{"event": "event", "name": "name"}` |
| `add-virtual-hook` | `{"_": ["AUTH-001", "event", "cmd"]}` | `{"workUnitId": "AUTH-001", "event": "event", "command": "cmd"}` |
| `list-virtual-hooks` | `{"_": ["AUTH-001"]}` | `{"workUnitId": "AUTH-001"}` |
| `remove-virtual-hook` | `{"_": ["AUTH-001", "name"]}` | `{"workUnitId": "AUTH-001", "name": "name"}` |
| `clear-virtual-hooks` | `{"_": ["AUTH-001"]}` | `{"workUnitId": "AUTH-001"}` |

### Checkpoints

| Command | Guidance `_` Format | Rust Named Keys |
|---------|---------------------|-----------------|
| `checkpoint` | `{"_": ["AUTH-001", "name"]}` | `{"workUnitId": "AUTH-001", "name": "name"}` |
| `list-checkpoints` | `{"_": ["AUTH-001"]}` | `{"workUnitId": "AUTH-001"}` |
| `restore-checkpoint` | `{"_": ["AUTH-001", "name"]}` | `{"workUnitId": "AUTH-001", "name": "name"}` |
| `cleanup-checkpoints` | `{"_": ["AUTH-001"], "keepLast": 5}` | `{"workUnitId": "AUTH-001", "keepLast": 5}` |

### Tag Management

| Command | Guidance `_` Format | Rust Named Keys |
|---------|---------------------|-----------------|
| `register-tag` | `{"_": ["@tag", "category", "desc"]}` | `{"tag": "@tag", "category": "category", "description": "desc"}` |
| `update-tag` | `{"_": ["@tag"], ...}` | `{"tag": "@tag", ...}` |
| `delete-tag` | `{"_": ["@tag"], ...}` | `{"tag": "@tag", ...}` |

### Metrics

| Command | Guidance `_` Format | Rust Named Keys |
|---------|---------------------|-----------------|
| `record-iteration` | `{"_": ["AUTH-001"]}` | `{"workUnitId": "AUTH-001"}` |
| `query-estimation-guide` | `{"_": ["AUTH-001"]}` | `{"workUnitId": "AUTH-001"}` |

### Validation

| Command | Guidance `_` Format | Rust Named Keys |
|---------|---------------------|-----------------|
| `validate` | `{"_": ["file"]}` | `{"file": "file"}` |
| `format` | `{"_": ["file"]}` | `{"file": "file"}` |

### Version Sync

| Command | Guidance `_` Format | Rust Named Keys |
|---------|---------------------|-----------------|
| `--sync-version` | `{"_": ["0.9.3"]}` | `{"version": "0.9.3"}` |

## Implementation Notes

1. **No code changes needed** — this is purely a documentation update to `fspec_workflow_guidance.rs`
2. **TypeScript callback still supports `_`** — the transformation layer in `fspec-callback.ts` converts `_` → CLI argv → Commander.js. The guidance change makes the format work for BOTH paths.
3. **Named keys are more explicit** — LLMs don't need to remember argument order.
4. **Named keys provide type safety** — serde deserialization validates field names at parse time.
