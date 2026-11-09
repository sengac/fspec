//! CMPCT-084: chunked compactor synthesis.
//!
//! A synthesis response that stops at the output limit is truncated. Only
//! complete `<dag-node>`/`<dag-files>` blocks are kept; the kept DAG is resent
//! as the assistant turn and the model is asked for the remaining nodes. A
//! chunk that completes no node (reasoning consumed the budget) is asked to
//! write the next node immediately. Continuation is bounded by
//! [`MAX_SYNTHESIS_CHUNKS`], [`STALLED_CHUNK_LIMIT`] and the caller's
//! synthesis deadline.
//!
//! BUG-614 (Q8): every chunk's complete nodes are structure-checked before it
//! is continued; a malformed node ends the candidate at once so the builder
//! returns the exact error for correction. A response that ends inside an
//! unclosed but well-formed `<dag-node>` after complete nodes is continued
//! like a truncated one, even without an output-limit stop reason.

use rig::completion::CompletionRequest;
use rig::message::{AssistantContent, Message};
use rig::OneOrMany;
use serde_json::Value;
use std::future::Future;

pub(super) const MAX_SYNTHESIS_CHUNKS: usize = 8;
const STALLED_CHUNK_LIMIT: usize = 2;
const NODE_END: &str = "</dag-node>";
const FILES_END: &str = "</dag-files>";

/// One synthesis response: its text and the provider stop reason.
pub(super) struct Chunk {
    pub text: String,
    pub stop_reason: Option<String>,
}

pub(super) fn is_output_limit(stop_reason: Option<&str>) -> bool {
    stop_reason.is_some_and(|reason| {
        matches!(
            reason.to_ascii_lowercase().as_str(),
            "length" | "max_tokens" | "max_output_tokens"
        )
    })
}

/// Stop reason of a serialized non-streaming provider response
/// (Chat Completions, Anthropic, Gemini and Responses shapes).
pub(super) fn raw_stop_reason(raw: &Value) -> Option<String> {
    raw.pointer("/choices/0/finish_reason")
        .or_else(|| raw.get("stop_reason"))
        .or_else(|| raw.pointer("/candidates/0/finishReason"))
        .or_else(|| raw.pointer("/incomplete_details/reason"))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

/// Byte length of the prefix that ends with the last complete DAG block.
fn complete_prefix(text: &str) -> usize {
    [NODE_END, FILES_END]
        .iter()
        .filter_map(|end| text.rfind(end).map(|start| start + end.len()))
        .max()
        .unwrap_or(0)
}

fn join(kept: &str, text: &str) -> String {
    if kept.is_empty() {
        text.to_string()
    } else {
        format!("{kept}\n{}", text.trim_start())
    }
}

/// Whether `text` ends inside a well-formed `<dag-node>` that was cut off
/// after at least one complete block.
fn cut_off_node(text: &str) -> bool {
    let boundary = complete_prefix(text);
    let tail = text[boundary..].trim();
    if boundary == 0 || !tail.starts_with("<dag-node") || tail.contains("</") {
        return false;
    }
    match tail.find('>') {
        None => true,
        Some(end) => codelet_core::compaction::validate_dag_structure(&format!(
            "{}summary</dag-node>",
            &tail[..=end]
        ))
        .is_ok(),
    }
}

fn instruction(kept: &str, progressed: bool, limited: bool) -> String {
    let nodes = kept.matches(NODE_END).count();
    if kept.is_empty() {
        return "Your previous response reached the output token limit before completing a single \
                <dag-node>, so nothing was kept. Write the DAG immediately with minimal reasoning: \
                start with the first <dag-node> now. You may stop after a few complete nodes; you \
                will be asked to continue."
            .to_string();
    }
    let last = kept
        .rfind("<dag-node ")
        .and_then(|start| {
            kept[start..]
                .find('>')
                .map(|end| &kept[start..=start + end])
        })
        .unwrap_or("<dag-node>");
    let stopped = if limited {
        "reached the output token limit"
    } else {
        "ended inside an unclosed <dag-node>"
    };
    if progressed {
        format!(
            "Your previous response {stopped}. The DAG above keeps your {nodes} \
             complete <dag-node> blocks (the last is {last}); the unfinished tail was discarded. \
             Continue the SAME DAG: output only the remaining <dag-node> blocks after these, then the \
             <dag-files> section. Do not repeat kept nodes. Keep reasoning brief; you may stop after a \
             few complete nodes and you will be asked to continue."
        )
    } else {
        format!(
            "Your previous response {stopped} before completing another \
             <dag-node>. The DAG above keeps {nodes} complete nodes (the last is {last}). Write the \
             next <dag-node> immediately with minimal reasoning, then the remaining nodes and the \
             <dag-files> section. Do not repeat kept nodes."
        )
    }
}

fn continuation(
    base: &CompletionRequest,
    kept: &str,
    progressed: bool,
    limited: bool,
) -> Result<CompletionRequest, String> {
    let mut history: Vec<Message> = base.chat_history.iter().cloned().collect();
    if !kept.is_empty() {
        history.push(Message::Assistant {
            id: None,
            content: OneOrMany::one(AssistantContent::text(kept)),
        });
    }
    history.push(Message::user(instruction(kept, progressed, limited)));
    let mut request = base.clone();
    request.chat_history = OneOrMany::many(history).map_err(|error| error.to_string())?;
    Ok(request)
}

/// Run synthesis, continuing in chunks while responses stop at the output
/// limit or end inside a cut-off node. `first` is a response already
/// generated for `base` (the exploration's final answer).
pub(super) async fn synthesize_in_chunks<Dispatch, Response>(
    base: CompletionRequest,
    mut first: Option<Chunk>,
    mut send: Dispatch,
) -> Result<String, String>
where
    Dispatch: FnMut(CompletionRequest) -> Response,
    Response: Future<Output = Result<Chunk, String>>,
{
    let mut kept = String::new();
    let mut stalled = 0;
    let mut request = base.clone();
    for chunk in 1..=MAX_SYNTHESIS_CHUNKS {
        let response = match first.take() {
            Some(response) => response,
            None => send(request).await?,
        };
        let assembled = join(&kept, &response.text);
        let limited = is_output_limit(response.stop_reason.as_deref());
        if !limited && !cut_off_node(&assembled) {
            return Ok(assembled);
        }
        let boundary = complete_prefix(&assembled);
        if boundary > 0
            && codelet_core::compaction::validate_dag_structure(&assembled[..boundary]).is_err()
        {
            // The candidate is already invalid: hand it to validation so the
            // exact error returns for correction instead of continuing it.
            return Ok(assembled);
        }
        let progressed = boundary > kept.len();
        if progressed {
            kept = assembled[..boundary].to_string();
            stalled = 0;
            if kept.trim_end().ends_with(FILES_END) {
                return Ok(kept);
            }
        } else {
            stalled += 1;
            if stalled >= STALLED_CHUNK_LIMIT {
                return Err(format!(
                    "compactor synthesis reached the output limit {stalled} times in a row without \
                     completing a dag-node ({} complete nodes kept)",
                    kept.matches(NODE_END).count()
                ));
            }
        }
        tracing::info!(
            chunk,
            kept_chars = kept.len(),
            progressed,
            limited,
            "CMPCT-084: compactor synthesis stopped mid-DAG; requesting continuation"
        );
        request = continuation(&base, &kept, progressed, limited)?;
    }
    Err(format!(
        "compactor synthesis still reached the output limit after {MAX_SYNTHESIS_CHUNKS} chunks"
    ))
}
