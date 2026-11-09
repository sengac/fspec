use crate::session::Session;
use codelet_core::compaction::annotation_detector::{
    detect_annotations, ToolCallInfo, TurnContext,
};

pub(super) fn process_turn_annotations(
    session: &mut Session,
    current: &mut Vec<ToolCallInfo>,
    previous: &mut Vec<ToolCallInfo>,
) {
    if current.is_empty() {
        return;
    }
    let ctx = TurnContext {
        current_tool_calls: current,
        previous_tool_calls: if previous.is_empty() {
            None
        } else {
            Some(previous)
        },
    };
    let annotations = detect_annotations(&ctx);
    if !annotations.is_empty() {
        let msg_idx = session.messages.len().saturating_sub(1);
        session.annotations.insert(msg_idx, annotations);
    }
    *previous = std::mem::take(current);
}
