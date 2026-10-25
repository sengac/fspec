#![allow(clippy::too_many_arguments)]

mod bridge;
mod finalization;
mod handlers;
mod idle_guard;
mod input;
mod lifecycle;
mod prompt;
mod provider_custom;
mod provider_openai;
mod providers;
mod reconciliation;
mod routing;
mod services;
mod state;
mod thinking;

use crate::background_output::{BackgroundOutput, LoopDetectionWiring};
use crate::dispatch::InputWithImages;
use codelet_rpc_types::{SessionStatus, StreamChunk};
use codelet_sessions::background_session::{BackgroundSession, PromptInput};
use codelet_tools::McpInjection;
use idle_guard::IdleOnDropGuard;
use state::{LoopState, PendingInput};
use std::sync::Arc;
use tokio::sync::mpsc;

pub async fn agent_loop(
    session: Arc<BackgroundSession>,
    mut input_rx: mpsc::Receiver<PromptInput>,
    mut mcp_injection_rx: mpsc::Receiver<McpInjection>,
) {
    let mut state = LoopState::default();
    let mut mcp_channel_open = true;
    lifecycle::start_session_hooks(&session).await;
    loop {
        let pending = if state.compaction_retry_input.is_some() {
            input::next_retry_input(&session, &mut state)
        } else {
            let mut is_fresh_user_input = false;
            let mut manual_compaction_epoch = None;
            let mut supervisor_rx = session.incoming_message_rx.lock().await;
            let input_to_process = tokio::select! {
                biased;
                result = input_rx.recv() => {
                    match result {
                        Some(prompt_input) => {
                            manual_compaction_epoch = prompt_input.compaction_epoch;
                            is_fresh_user_input = manual_compaction_epoch.is_none();
                            Some(InputWithImages {
                                text: prompt_input.input,
                                thinking_config: prompt_input.thinking_config,
                                images: None,
                            })
                        }
                        None => {
                            drop(supervisor_rx);
                            lifecycle::end_session_hooks(&session).await;
                            break;
                        }
                    }
                }
                result = supervisor_rx.recv() => {
                    result.map(|message| input::supervisor_input(&session, message))
                }
                result = mcp_injection_rx.recv(), if mcp_channel_open => {
                    match result {
                        Some(McpInjection::Notification(text)) => {
                            tracing::info!(input_bytes = text.len(), input = %text, "[MCP] agent_loop received notification");
                            session.handle_output(StreamChunk::incoming_message(text.clone()));
                            Some(InputWithImages { text, thinking_config: None, images: None })
                        }
                        Some(McpInjection::SamplingRequest { params, response_tx }) => {
                            tracing::info!(
                                "[MCP] agent_loop received sampling/createMessage request ({} messages, maxTokens={})",
                                params.messages.len(), params.max_tokens,
                            );
                            let _ = response_tx.send(Err(
                                "sampling/createMessage not yet supported — V2 feature".to_string(),
                            ));
                            tracing::debug!("[MCP] sampling/createMessage rejected (V2 feature)");
                            None
                        }
                        None => {
                            tracing::info!("[MCP] injection channel closed for session {}", session.id);
                            mcp_channel_open = false;
                            None
                        }
                    }
                }
            };
            drop(supervisor_rx);
            input_to_process.map(|input| PendingInput {
                input,
                is_retry_input: false,
                is_fresh_user_input,
                is_compaction_retry: manual_compaction_epoch.is_some(),
                manual_compaction_epoch,
            })
        };
        let Some(pending) = pending else {
            continue;
        };
        if !prompt::admit_prompt(&session, &pending, &mut state).await {
            continue;
        }
        let input = &pending.input.text;
        let (current_provider, current_model, thinking_model_key) =
            routing::resolve_turn_routing(&session).await;
        let thinking_config_value = thinking::resolve_turn_thinking(
            &session,
            &pending.input,
            &current_provider,
            current_model.as_deref(),
            thinking_model_key.as_deref(),
        );
        let mut inner_session = session.inner.lock().await;
        if let Err(error) = session.refresh_runtime_settings_for_turn(&mut inner_session) {
            session.handle_output(StreamChunk::error(error));
            session.set_status(SessionStatus::Idle);
            session.handle_output(StreamChunk::done());
            continue;
        }
        if pending.is_fresh_user_input {
            session.sync_completion_contract_for_user_turn(&mut inner_session);
        } else {
            session.sync_completion_contract_for_recovery(&mut inner_session);
        }
        let session_for_output = session.clone();
        let loop_wiring = {
            let resolved = codelet_sessions::model_resolution::resolve_profile_loop_detection(
                inner_session.provider_manager(),
            );
            LoopDetectionWiring {
                enabled: resolved.enabled.unwrap_or(true),
                window: resolved.window.unwrap_or(160),
                max_repeats: resolved.max_repeats.unwrap_or(10),
                max_retries: resolved.max_retries.unwrap_or(10),
            }
        };
        if let Err(error) = inner_session.bind_provider_persistence(session.id) {
            session.handle_output(StreamChunk::error(error.to_string()));
            session.set_status(SessionStatus::Idle);
            session.handle_output(StreamChunk::done());
            continue;
        }
        inner_session.begin_provider_turn();
        session.prepare_provider_status(
            &inner_session.provider_session,
            inner_session.current_provider_name(),
        );
        let output = BackgroundOutput::with_provider(
            session_for_output,
            current_provider.clone(),
            Some(loop_wiring),
        );
        output.reset_turn_loop_detectors();
        if let Some(loop_abort_note) = session.take_pending_loop_abort_note() {
            tracing::info!(
                "[RIG-014] injecting loop-abort corrective note into turn context (session={})",
                session.id
            );
            inner_session.messages.push(rig::message::Message::User {
                content: rig::OneOrMany::one(rig::message::UserContent::text(loop_abort_note)),
            });
        }
        handlers::register_core_handlers(&session);
        let (turn_compaction, compactor_registration) =
            services::register_service_handlers(&session, &inner_session);
        bridge::register_bridge_handlers(&session);
        let bridge_images = pending.input.images.map(|imgs| {
            imgs.into_iter()
                .map(|img| codelet_cli::interactive::BridgeImage {
                    data: img.data,
                    media_type: img.media_type,
                })
                .collect()
        });
        let mut idle_guard = IdleOnDropGuard {
            session: session.clone(),
            armed: true,
        };
        let builder_retry = if pending.is_fresh_user_input {
            codelet_cli::compactor_sub_agent::retry_failed_compactor_for_user_input(
                &mut inner_session,
                session.id,
                &session.compaction_in_progress,
                &session.is_interrupted,
                &session.interrupt_notify,
                &output,
            )
            .await
        } else {
            Ok(false)
        };
        let builder_retry_failed = builder_retry.is_err();
        let mut result = providers::run_selected_provider(
            &mut inner_session,
            input,
            bridge_images,
            &session,
            &output,
            thinking_config_value,
            &current_provider,
            current_model.clone(),
            builder_retry,
        )
        .await;
        let applied = reconciliation::reconcile_turn(
            &session,
            &mut inner_session,
            &turn_compaction,
            &mut result,
            &mut state,
            current_provider,
            current_model,
        );
        handlers::clear_core_handlers(&session);
        services::clear_service_handlers(&session);
        handlers::clear_input_handlers(&session);
        bridge::clear_bridge_handlers(&session);
        idle_guard.armed = false;
        finalization::settle_turn_result(
            &session,
            &mut inner_session,
            result,
            applied,
            builder_retry_failed,
            &mut state,
            &output,
        )
        .await;
        drop(compactor_registration);
        finalization::schedule_loop_retry(&session, &output, &mut state);
    }
}
