#![allow(clippy::too_many_arguments)]

mod admission;
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
pub mod thinking;

use crate::background_output::{BackgroundOutput, LoopDetectionWiring};
use crate::dispatch::InputWithImages;
use codelet_core::persistence::input_receipts::InputDisposition::Refused;
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
    let _continuations = admission::ContinuationRegistration::new(session.id);
    loop {
        let pending = if state.compaction_retry_input.is_some() {
            input::next_retry_input(&session, &mut state)
        } else {
            let mut is_fresh_user_input = false;
            let mut manual_compaction_epoch = None;
            let mut interrupt_epoch = session.compaction_epoch();
            let mut cancelled_at_submission = false;
            let mut submission = None;
            let mut supervisor_rx = session.incoming_message_rx.lock().await;
            let input_to_process = tokio::select! {
                biased;
                result = input_rx.recv() => {
                    match result {
                        Some(prompt_input) => {
                            manual_compaction_epoch = prompt_input.compaction_epoch;
                            interrupt_epoch = prompt_input.interrupt_epoch;
                            is_fresh_user_input = manual_compaction_epoch.is_none();
                            submission = prompt_input.submission;
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
                    result.map(|message| {
                        cancelled_at_submission = message.interrupt_epoch.is_none();
                        submission = message.submission_id;
                        interrupt_epoch = message.interrupt_epoch.unwrap_or(interrupt_epoch);
                        input::supervisor_input(&session, message)
                    })
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
                is_manual_compaction: manual_compaction_epoch.is_some(),
                interrupt_epoch,
                cancelled_at_submission,
                submission,
            })
        };
        let Some(mut pending) = pending else {
            continue;
        };
        if !prompt::admit_prompt(&session, &mut pending, &mut state).await {
            continue;
        }
        let input = &pending.input.text;
        let (current_provider, current_model, thinking_target) =
            routing::resolve_turn_routing(&session).await;
        // BUG-439 / MODEL-010: a level the provider or model cannot carry
        // refuses the turn.
        let thinking_config_value = match thinking::resolve_turn_thinking(
            &session,
            pending.input.thinking_config.as_deref(),
            &thinking_target,
        ) {
            Ok(config) => config,
            Err(reason) => {
                admission::settle(
                    &session,
                    &pending,
                    Refused {
                        reason: reason.clone(),
                    },
                )
                .await;
                session.handle_output(StreamChunk::error(reason));
                session.set_status(SessionStatus::Idle);
                session.handle_output(StreamChunk::done());
                continue;
            }
        };
        let mut inner_session = session.inner.lock().await;
        if prompt::cancel_pending(&session, &pending, false).await {
            continue;
        }
        if let Err(error) = session.refresh_runtime_settings_for_turn(&mut inner_session) {
            admission::settle(
                &session,
                &pending,
                Refused {
                    reason: error.clone(),
                },
            )
            .await;
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
            LoopDetectionWiring::from(resolved)
        };
        if let Err(error) = inner_session.bind_provider_persistence(session.id) {
            let reason = error.to_string();
            admission::settle(&session, &pending, Refused { reason }).await;
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
        if let Some(loop_abort_note) = (!pending.is_manual_compaction)
            .then(|| session.take_pending_loop_abort_note())
            .flatten()
        {
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
        let bridge_images = pending.input.images.clone().map(|imgs| {
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
        // BUG-614 (Q4): any next input, including background and supervisor
        // messages, retries a failed builder with a fresh budget. Internal
        // retries are not new input; a manual /compact restarts it itself.
        let builder_retry = if !pending.is_retry_input && !pending.is_manual_compaction {
            codelet_cli::compactor_sub_agent::retry_failed_compactor_for_next_input(
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
        // BUG-516: a manual /compact runs the compactor sub-agent round in
        // the provider's agent stream; it is not an admitted conversation
        // input, so it skips input admission.
        let mut result = if pending.is_manual_compaction {
            inner_session.request_manual_compaction();
            providers::run_selected_provider(
                &mut inner_session,
                "",
                None,
                &session,
                &output,
                thinking_config_value,
                &current_provider,
                current_model.clone(),
                builder_retry,
            )
            .await
        } else {
            match admission::admit_turn(&session, &mut inner_session, &pending).await {
                admission::Gate::Cancelled => Ok(()),
                admission::Gate::Refused(error) => Err(error),
                admission::Gate::Dispatch(submission) => {
                    // MODEL-015: plan the turn's Claude effort before dispatch.
                    let planned = crate::claude_effort::plan_turn_effort(
                        session.id,
                        &mut inner_session,
                        &thinking_target,
                        thinking_config_value.as_ref(),
                        submission,
                        pending.is_fresh_user_input,
                    )
                    .await;
                    let result = match planned {
                        Ok(()) => {
                            providers::run_selected_provider(
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
                            .await
                        }
                        Err(reason) => Err(anyhow::anyhow!(reason)),
                    };
                    admission::settle_dispatch(&session, &mut inner_session, submission, &result)
                        .await;
                    result
                }
            }
        };
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
        finalization::start_queued_input(&session, &state);
    }
}
