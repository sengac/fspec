use super::SessionContext;
use crate::views::agent::{ChunkKind, ChunkSource};

#[derive(Debug)]
pub(super) struct ResponseAttemptCheckpoint {
    next_seq: u64,
    assistant_seq: Option<u64>,
    thinking_seq: Option<u64>,
    sources: Vec<(u64, ChunkSource)>,
}

pub(super) fn start(ctx: &mut SessionContext) {
    let chunks = ctx.scrollback.chunks();
    let assistant = ctx.in_flight_assistant.and_then(|index| chunks.get(index));
    let thinking = ctx.in_flight_thinking.and_then(|index| chunks.get(index));
    ctx.response_attempt = Some(ResponseAttemptCheckpoint {
        next_seq: ctx.scrollback_next_seq,
        assistant_seq: assistant.map(|chunk| chunk.seq),
        thinking_seq: thinking.map(|chunk| chunk.seq),
        sources: assistant
            .into_iter()
            .chain(thinking)
            .filter_map(|chunk| chunk.source.clone().map(|source| (chunk.seq, source)))
            .collect(),
    });
}

pub(super) fn abort(ctx: &mut SessionContext) {
    let Some(checkpoint) = ctx.response_attempt.take() else {
        return;
    };
    ctx.scrollback.chunks_mut().retain(|chunk| {
        chunk.seq < checkpoint.next_seq
            || !chunk.source.as_ref().is_some_and(|source| {
                matches!(source.kind, ChunkKind::AssistantText | ChunkKind::Thinking)
            })
    });
    for (seq, source) in checkpoint.sources {
        if let Some(index) = ctx
            .scrollback
            .chunks()
            .iter()
            .position(|chunk| chunk.seq == seq)
        {
            ctx.scrollback.chunks_mut()[index].source = Some(source);
            ctx.scrollback.rewrap_at(index);
        }
    }
    let locate = |seq| {
        ctx.scrollback
            .chunks()
            .iter()
            .position(|chunk| Some(chunk.seq) == seq)
    };
    ctx.in_flight_assistant = locate(checkpoint.assistant_seq);
    ctx.in_flight_thinking = locate(checkpoint.thinking_seq);
    ctx.scrollback.refresh_after_rollback();
}
