//! Feature: spec/features/board-card-enter-chooser-and-complete-details.feature

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::{EmbeddedFspecBackend, WebSocketFspecBackend};
use codelet_rpc::SharedFspecService;
use serde_json::{json, Value};

use super::*;

pub struct Canonical {
    pub app: App,
    pub root: tempfile::TempDir,
    pub record: Value,
    listener: Option<tokio::task::JoinHandle<()>>,
}

impl Canonical {
    pub async fn new(websocket: bool) -> Result<Self, Box<dyn std::error::Error>> {
        let root = codelet_test_helpers::setup_temp_directory()?;
        std::fs::create_dir_all(root.path().join("spec/attachments/A"))?;
        let record = rich_record();
        write_store(root.path(), &record)?;
        let watcher = Arc::new(WorkUnitsWatcher::new(root.path())?);
        let service =
            Arc::new(SharedFspecService::new(watcher).with_cwd(root.path().to_path_buf()));
        let (backend, listener): (Arc<dyn FspecBackend>, _) = if websocket {
            let (address, listener) = common::start_ws_server(service).await;
            let backend = WebSocketFspecBackend::connect(common::ws_url(address)).await?;
            (Arc::new(backend), Some(listener))
        } else {
            (
                Arc::new(EmbeddedFspecBackend::new(
                    tokio::runtime::Handle::current(),
                    service,
                )),
                None,
            )
        };
        Ok(Self {
            app: app_with(backend),
            root,
            record,
            listener,
        })
    }

    pub fn save(&self) -> TestResult {
        write_store(self.root.path(), &self.record)
    }
}

impl Drop for Canonical {
    fn drop(&mut self) {
        if let Some(listener) = self.listener.take() {
            listener.abort();
        }
    }
}

fn write_store(root: &std::path::Path, record: &Value) -> TestResult {
    let other = json!({
        "id": "AUTH-002", "title": "Linked B", "type": "story", "status": "backlog",
        "description": "B-CANONICAL-DETAIL", "dependsOn": ["AUTH-001"],
        "createdAt": "2026-01-02T00:00:00Z", "updatedAt": "2026-01-02T00:00:00Z"
    });
    std::fs::write(
        root.join("spec/work-units.json"),
        serde_json::to_vec(&json!({
            "workUnits": {"AUTH-001": record, "AUTH-002": other}
        }))?,
    )?;
    Ok(())
}

fn rich_record() -> Value {
    json!({
        "id": "AUTH-001", "title": "Canonical A", "type": "story", "status": "backlog",
        "description": format!("FIRST-CANONICAL-LINE\n{}\nFINAL-CANONICAL-LINE", "日本語 café 🦀 wrapped words ".repeat(90)),
        "createdAt": "2026-01-01T00:00:00Z", "updatedAt": "2026-10-28T00:00:00Z",
        "updated": "LEGACY-UPDATED", "estimate": 5, "epic": "COMPLETE-EPIC",
        "userStory": {"role": "ROLE-MARKER", "action": "ACTION-MARKER", "benefit": "BENEFIT-MARKER"},
        "dependsOn": ["AUTH-002", "AUTH-404"], "blocks": ["BLOCK-333"],
        "blockedBy": ["BLOCKER-444"], "relatesTo": ["RELATED-555"],
        "parent": "PARENT-111", "children": ["CHILD-222"],
        "rules": [{"id": 0, "text": "ACTIVE-RULE", "deleted": false, "createdAt": "2026-01-02T00:00:00Z"}, {"id": 1, "text": "RETIRED-RULE", "deleted": true, "createdAt": "2026-01-02T00:00:00Z"}],
        "examples": [{"id": 0, "text": "EXAMPLE-MARKER", "deleted": false, "createdAt": "2026-01-02T00:00:00Z"}],
        "questions": [{"id": 0, "text": "ANSWERED-QUESTION", "selected": true, "deleted": false, "createdAt": "2026-01-02T00:00:00Z", "answer": "ANSWER-MARKER"}],
        "architectureNotes": [{"id": 0, "text": "ARCHITECTURE-MARKER", "deleted": false, "createdAt": "2026-01-02T00:00:00Z"}],
        "assumptions": ["ASSUMPTION-MARKER"],
        "stateHistory": [{"state": "testing", "timestamp": "2026-01-03T00:00:00Z", "reason": "STATE-HISTORY-MARKER"}],
        "statusHistory": [{"from": "backlog", "to": "specifying", "timestamp": "2026-01-02T00:00:00Z", "reason": "LEGACY-HISTORY-MARKER"}],
        "blockedReason": "BLOCKED-REASON-MARKER",
        "eventStorm": {"level": "process_modeling", "nextItemId": 2, "items": [
            {"id": 0, "type": "event", "color": "orange", "text": "EVENT-STORM-MARKER", "deleted": false, "createdAt": "2026-01-02T00:00:00Z"},
            {"id": 1, "type": "event", "color": "orange", "text": "RETIRED-EVENT-MARKER", "deleted": true, "createdAt": "2026-01-02T00:00:00Z"}
        ]},
        "virtualHooks": [{"name": "VIRTUAL-HOOK-MARKER", "event": "pre-testing", "command": "true", "blocking": true}],
        "iterations": 7, "actualTokens": 12345,
        "attachments": ["spec/attachments/A/a 日本語 #%.md", "spec/attachments/B/a 日本語 #%.md"],
        "futureField": {"nested": "UNKNOWN-FIELD-MARKER"},
        "legacyCompatibility": {"architectureNotes": ["ODD-NOTES-MARKER"], "statusHistory": [{"status": "ODD-HISTORY-MARKER"}], "eventStorm": {"events": ["ODD-EVENT-MARKER"]}}
    })
}
