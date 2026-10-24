//! Feature: spec/features/provider-facade-parity.feature

use codelet_fspec_tui::{Action, App, ChunkKind};
use codelet_rpc_types::{SessionId, StreamChunk, ToolCallInfo, ToolResultInfo};
use ratatui::style::Color;
use serde_json::json;
use std::sync::Arc;

mod common;

fn patch_result(is_error: bool) -> App {
    let mut app = App::new(Arc::new(common::MockBackend::new()));
    let session = SessionId::new("patch-session");
    app.dispatch(Action::SessionCreated(session.clone()));
    app.dispatch(Action::ChunkReceived(session.clone(), StreamChunk::tool_call(ToolCallInfo {
        id: "patch-call".into(),
        name: "apply_patch".into(),
        input: json!({"patch": "*** Begin Patch\n*** Add File: /tmp/new.rs\n+created\n*** Update File: /tmp/change.rs\n@@\n-old\n+new\n*** Delete File: /tmp/deleted.rs\n*** End Patch"}).to_string(),
    })));
    app.dispatch(Action::ChunkReceived(
        session,
        StreamChunk::tool_result(ToolResultInfo {
            tool_call_id: "patch-call".into(),
            content: if is_error {
                "Context mismatch"
            } else {
                "Updated files"
            }
            .into(),
            is_error,
        }),
    ));
    app
}

#[test]
fn patch_changes_use_the_shared_colored_diff_renderer() -> Result<(), Box<dyn std::error::Error>> {
    let app = patch_result(false);
    let context = app
        .agent_view_store()
        .session_context_for(&SessionId::new("patch-session"))
        .ok_or("missing session")?;
    let card = context.scrollback.chunks().iter().find(|chunk| chunk.source.as_ref().is_some_and(|source| matches!(&source.kind, ChunkKind::ToolCall { tool_call_id, .. } if tool_call_id == "patch-call"))).ok_or("missing card")?;
    let source = card.source.as_ref().ok_or("missing source")?;
    assert!(matches!(
        source.kind,
        ChunkKind::ToolCall { is_diff: true, .. }
    ));
    let full = source.full_text.as_ref().ok_or("missing expanded diff")?;
    for path in ["/tmp/new.rs", "/tmp/change.rs", "/tmp/deleted.rs"] {
        assert!(full.contains(path), "missing affected file {path}");
    }
    for color in [Color::Rgb(139, 0, 0), Color::Rgb(0, 100, 0)] {
        assert!(
            card.lines
                .iter()
                .flat_map(|line| &line.spans)
                .any(|span| span.style.bg == Some(color)),
            "missing shared diff color {color:?}"
        );
    }
    Ok(())
}

#[test]
fn rejected_patches_retain_their_error_output() -> Result<(), Box<dyn std::error::Error>> {
    let app = patch_result(true);
    let context = app
        .agent_view_store()
        .session_context_for(&SessionId::new("patch-session"))
        .ok_or("missing session")?;
    let card = context.scrollback.chunks().iter().find(|chunk| chunk.source.as_ref().is_some_and(|source| matches!(&source.kind, ChunkKind::ToolCall { tool_call_id, .. } if tool_call_id == "patch-call"))).ok_or("missing card")?;
    let source = card.source.as_ref().ok_or("missing source")?;
    assert!(source.text.contains("Context mismatch"));
    assert!(matches!(
        source.kind,
        ChunkKind::ToolCall {
            is_error: true,
            is_diff: false,
            ..
        }
    ));
    Ok(())
}
