//! Feature: spec/features/codex-refreshed-reminders.feature

use codelet_providers::codex::history::CodexHistory;
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn reminder(text: &str) -> Value {
    json!({"type":"message","role":"user","content":[{
        "type":"input_text","text":format!("<system-reminder>{text}</system-reminder>")
    }]})
}

#[test]
fn refreshed_standalone_reminders_retain_typed_continuation() -> TestResult {
    for refresh in [true, false] {
        // @step Given a Codex conversation with a standalone runtime reminder
        let history = CodexHistory::default();
        let original = reminder("old runtime context");
        let current = reminder("current runtime context");
        let user = json!({"type":"message","role":"user","content":"inspect"});
        let request = json!({"model":"test","input":[original,user]});
        // @step And saved tool calls
        let output =
            json!({"type":"function_call","call_id":"call_1","name":"inspect","arguments":"{}"});
        history.commit(&request, std::slice::from_ref(&output))?;
        let result =
            json!({"type":"function_call_output","call_id":"call_1","output":"real result"});
        let mut incoming = vec![user];
        if refresh {
            incoming.insert(0, current.clone());
        }
        incoming.push(result.clone());
        let mut next = json!({"model":"test","input":incoming});
        // @step When the reminder changes without replacing ordinary user history
        history.prepare(&mut next)?;
        // @step Then typed continuation is retained with the current reminder
        assert!(history.snapshot()?.is_some());
        let mut expected = vec![request["input"][1].clone(), output];
        if refresh {
            expected.insert(0, current);
        }
        expected.push(result);
        // @step And each real tool output keeps its matching call
        assert_eq!(next["input"], json!(expected));
    }
    Ok(())
}

#[test]
fn compaction_only_history_discards_old_typed_continuation() -> TestResult {
    // @step Given a Codex conversation containing only standalone runtime reminders
    let history = CodexHistory::default();
    let initial = json!({"model":"test","input":[reminder("runtime context")]});
    // @step And saved opaque reasoning
    history.commit(
        &initial,
        &[json!({"type":"reasoning","summary":[],"encrypted_content":"old opaque state"})],
    )?;
    let mut next =
        json!({"model":"test","input":[reminder("\n<!-- type:compaction-dag -->\nsummary")]});
    let original = next.clone();
    let previous = history.snapshot()?.ok_or("saved checkpoint missing")?;
    // @step When local compaction commits an advancing boundary before replacing the history with a compaction DAG
    history.transition_generation(previous.generation + 1)?;
    history.prepare(&mut next)?;
    // @step Then the old typed continuation is discarded
    let current = history.snapshot()?.ok_or("committed boundary missing")?;
    assert_eq!(current.generation, previous.generation + 1);
    assert!(current.items.is_empty());
    // @step And only the replacement DAG is replayed
    assert_eq!(next, original);
    Ok(())
}
