use codelet_rpc_types::StreamChunk;
use codelet_sessions::background_session::{BackgroundSession, BridgeImageData, IncomingMessage};
use codelet_tools::tool_pause::PauseResponse;
use std::sync::Arc;

pub(super) fn register_bridge_handlers(session: &Arc<BackgroundSession>) {
    let session_for_bridge = session.clone();
    let session_id_for_bridge = session.id;
    let runtime_handle = tokio::runtime::Handle::current();
    let supervisor_broadcast_sender = session_for_bridge.supervisor_broadcast.clone();
    let broadcast_rx_factory: codelet_tools::BroadcastReceiverFactory = Arc::new(move || {
        let mut stream_rx = supervisor_broadcast_sender.subscribe();
        let (json_tx, json_rx) = tokio::sync::broadcast::channel::<serde_json::Value>(256);
        let json_tx_clone = json_tx.clone();
        tokio::spawn(async move {
            loop {
                match stream_rx.recv().await {
                    Ok(chunk) => {
                        let json_value =
                            crate::stream_chunk_json::stream_chunk_to_json_value(&chunk);
                        let _ = json_tx_clone.send(json_value);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                        tracing::warn!("Bridge adapter lagged {} messages", count);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::debug!("Bridge adapter: source broadcast closed");
                        break;
                    }
                }
            }
        });
        json_rx
    });
    let session_for_injector = session_for_bridge.clone();
    let input_injector: codelet_tools::InputInjector =
        Arc::new(move |input: codelet_tools::InjectedInput| {
            let bridge_images = input.images.map(|imgs| {
                imgs.into_iter()
                    .map(|img| BridgeImageData {
                        data: img.data,
                        media_type: img.media_type,
                    })
                    .collect()
            });
            let supervisor_input = if input.message.is_empty() && bridge_images.is_some() {
                IncomingMessage {
                    source_session_id: "bridge".to_string(),
                    role_name: "bridge".to_string(),
                    message: String::new(),
                    images: bridge_images,
                    interrupt_epoch: None,
                }
            } else {
                IncomingMessage {
                    source_session_id: "bridge".to_string(),
                    role_name: "bridge".to_string(),
                    message: input.message.clone(),
                    images: bridge_images,
                    interrupt_epoch: None,
                }
            };
            match session_for_injector.receive_incoming_message(supervisor_input) {
                Ok(()) => {
                    tracing::debug!(
                        "Bridge input injected successfully: {}",
                        input.message.chars().take(50).collect::<String>()
                    );
                }
                Err(error) => {
                    tracing::warn!("Failed to inject bridge input: {}", error);
                }
            }
        });
    let session_for_control = session.clone();
    let control_handler: codelet_tools::ControlHandler =
        Arc::new(move |action: &str, response: Option<&str>| match action {
            "interrupt" => {
                tracing::info!("Bridge control: interrupting session");
                session_for_control.interrupt();
            }
            "clear" => {
                tracing::info!("Bridge control: clearing session");
                tokio::task::block_in_place(|| {
                    crate::bridge_control::clear_history(&session_for_control);
                });
            }
            "pause_response" => {
                if let Some(resp) = response {
                    tracing::info!(response = %resp, "Bridge control: pause_response received");
                    let pause_resp = match resp {
                        "allow_once" => PauseResponse::AllowOnce,
                        "allow_session" => PauseResponse::AllowSession,
                        "deny" => PauseResponse::Denied,
                        _ => {
                            tracing::warn!("Unknown pause response: {}, defaulting to deny", resp);
                            PauseResponse::Denied
                        }
                    };
                    session_for_control.send_pause_response(pause_resp);
                } else {
                    tracing::warn!("pause_response action received without response value");
                }
            }
            _ => {
                tracing::warn!("Bridge control: unknown action '{}'", action);
            }
        });
    let session_for_command = session.clone();
    let command_emitter: codelet_tools::CommandEmitter =
        Arc::new(move |command, args_json, project_root, tool_call_id| {
            if !crate::is_global_chunk_callback_registered() {
                tracing::warn!("Cannot emit FspecCommandRequest - no global chunk callback");
                return;
            }
            let fspec_request = codelet_rpc_types::FspecRequest {
                command,
                args_json,
                project_root,
                tool_call_id,
            };
            session_for_command.handle_output(StreamChunk::fspec_command_request(fspec_request));
        });
    codelet_tools::set_bridge_session_context(
        session_id_for_bridge,
        broadcast_rx_factory,
        input_injector,
        Some(control_handler),
        Some(command_emitter),
    );
    let bridge_handler: codelet_tools::BridgeHandler =
        Arc::new(move |request: codelet_tools::BridgeRequest| {
            tokio::task::block_in_place(|| {
                runtime_handle.block_on(async {
                    match codelet_tools::handle_bridge_action(request.session_id, request.action)
                        .await
                    {
                        Ok(result) => result,
                        Err(error) => codelet_tools::BridgeResult {
                            success: false,
                            message: format!("Bridge action failed: {}", error),
                            connections: None,
                        },
                    }
                })
            })
        });
    codelet_tools::set_bridge_handler(session.id, Some(bridge_handler));
}

pub(super) fn clear_bridge_handlers(session: &BackgroundSession) {
    codelet_tools::set_bridge_handler(session.id, None);
    codelet_tools::remove_bridge_session_context(session.id);
}
