use serde_json::{json, Value};

pub fn rich_record() -> Value {
    json!({
        "id": "AUTH-001", "title": "Complete canonical card", "type": "story", "status": "backlog",
        "description": format!("CANONICAL-FIRST\n{}\nCANONICAL-LAST", "日本語 café 🦀\n".repeat(4096)),
        "createdAt": "2026-01-01T00:00:00Z", "updatedAt": "2026-10-28T00:00:00Z",
        "updated": "LEGACY-UPDATED", "estimate": 8, "epic": "AUTH",
        "userStory": {"role": "developer", "action": "inspect everything", "benefit": "complete context"},
        "dependsOn": ["AUTH-002"], "blockedBy": ["AUTH-003"], "blocks": ["AUTH-004"],
        "relatesTo": ["AUTH-005"], "parent": "AUTH-010", "children": ["AUTH-011"],
        "rules": [{"id": 0, "text": "active rule", "deleted": false}, {"id": 1, "text": "historical rule", "deleted": true}],
        "examples": [{"id": 0, "text": "example", "deleted": false}],
        "questions": [{"id": 0, "text": "question", "selected": true, "answered": true, "answer": "answer"}],
        "architectureNotes": [{"id": 0, "text": "architecture", "deleted": false, "createdAt": "2026-01-02T00:00:00Z"}],
        "assumptions": ["assumption"],
        "stateHistory": [{"state": "testing", "timestamp": "2026-01-03T00:00:00Z", "reason": "current state history"}],
        "statusHistory": [{"from": "backlog", "to": "specifying", "timestamp": "2026-01-02T00:00:00Z", "reason": "legacy status history"}],
        "blockedReason": "dependency pending",
        "eventStorm": {"level": "process_modeling", "nextItemId": 2, "items": [
            {"id": 0, "type": "event", "color": "orange", "text": "event", "deleted": false, "createdAt": "2026-01-02T00:00:00Z"},
            {"id": 1, "type": "event", "color": "orange", "text": "historical event", "deleted": true, "createdAt": "2026-01-02T00:00:00Z"}
        ]},
        "virtualHooks": [{"name": "hook", "event": "pre-testing", "command": "true"}],
        "iterations": 7, "actualTokens": 12345,
        "attachments": ["spec/attachments/A/a 日本語 #%.md", "spec/attachments/B/a 日本語 #%.md"],
        "futureField": {"nested": "unknown field", "nullable": null, "list": [false, 42, {"extra": "preserved"}]},
        "legacyCompatibility": {"architectureNotes": ["unusual notes"], "statusHistory": [{"status": "legacy status"}], "eventStorm": {"events": ["legacy events"]}}
    })
}
