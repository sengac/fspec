//! Agent streaming loop with interruption support.
//!
//! The driver owns one stream and delegates typed event, recovery and settlement
//! operations to focused modules. CLI polling and background notification share
//! the same cancellation, rollback, retry and finalization order.

mod annotations;
mod assistant;
mod driver;
mod final_completion;
mod final_response;
mod final_thinking;
mod finish;
mod interrupted;
mod manual_compaction;
mod poll;
mod preflight;
mod recovery;
mod recovery_compaction;
mod recovery_network;
mod recovery_truncation;
mod restart;
mod state;
mod steer;
mod stream_end;
mod telemetry;
mod terminal_error;
mod tool_result;
mod uncompacted;
mod usage;

use super::{multimodal::BridgeImage, output::StreamOutput};
use crate::session::Session;
use codelet_core::RigAgent;
use rig::{
    completion::{CompletionModel, GetTokenUsage},
    wasm_compat::WasmCompatSend,
};
use std::sync::{atomic::AtomicBool, Arc};
use tokio::sync::Notify;

#[allow(clippy::too_many_arguments)]
pub async fn run_agent_stream<M, O>(
    agent: RigAgent<M>,
    prompt: &str,
    session: &mut Session,
    is_interrupted: Arc<AtomicBool>,
    compaction_in_progress: Arc<AtomicBool>,
    interrupt_notify: Arc<Notify>,
    output: &O,
    session_id: uuid::Uuid,
) -> anyhow::Result<()>
where
    M: CompletionModel,
    M::StreamingResponse: WasmCompatSend + GetTokenUsage,
    O: StreamOutput,
{
    driver::run::<M, O>(
        agent,
        prompt,
        None,
        session,
        None,
        is_interrupted,
        compaction_in_progress,
        Some(interrupt_notify),
        output,
        session_id,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn run_agent_stream_with_images<M, O>(
    agent: RigAgent<M>,
    prompt: &str,
    images: Option<Vec<BridgeImage>>,
    session: &mut Session,
    is_interrupted: Arc<AtomicBool>,
    compaction_in_progress: Arc<AtomicBool>,
    interrupt_notify: Arc<Notify>,
    output: &O,
    session_id: uuid::Uuid,
) -> anyhow::Result<()>
where
    M: CompletionModel,
    M::StreamingResponse: WasmCompatSend + GetTokenUsage,
    O: StreamOutput,
{
    driver::run::<M, O>(
        agent,
        prompt,
        images,
        session,
        None,
        is_interrupted,
        compaction_in_progress,
        Some(interrupt_notify),
        output,
        session_id,
    )
    .await
}
