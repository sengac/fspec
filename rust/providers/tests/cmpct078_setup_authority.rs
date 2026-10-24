use codelet_providers::{
    codex::{
        history::CodexHistory, refreshing_client::RefreshingCodexClient,
        websocket::CodexWebSocketHandle, CodexResponsesModel,
    },
    request_admission::AdmissionLimits,
};
use futures::{SinkExt, StreamExt};
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

#[path = "fixtures/prov148.rs"]
mod fixture;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn model(
    client: RefreshingCodexClient,
    url: &str,
    history: &CodexHistory,
) -> Result<CodexResponsesModel, Box<dyn std::error::Error>> {
    let client = rig::providers::openai::Client::<RefreshingCodexClient>::builder()
        .api_key("test-key")
        .base_url(url)
        .http_client(client)
        .build()?;
    Ok(CodexResponsesModel::new(client, "shared-model")
        .with_history(history.clone())
        .with_request_admission(AdmissionLimits {
            context_capacity: Some(10000),
            output_reservation: Some(100),
        }))
}

fn consume(model: CodexResponsesModel) -> tokio::task::JoinHandle<(Vec<String>, usize)> {
    tokio::spawn(async move {
        let result = model
            .stream(
                CompletionRequestBuilder::new(model.clone(), "old generation")
                    .max_tokens(100)
                    .build(),
            )
            .await;
        let mut stream = match result {
            Ok(stream) => stream,
            Err(error) => return (vec![error.to_string()], 0),
        };
        let mut errors = Vec::new();
        let mut finals = 0;
        while let Some(event) = stream.next().await {
            match event {
                Err(error) => errors.push(error.to_string()),
                Ok(rig::streaming::StreamedAssistantContent::Final(_)) => finals += 1,
                _ => {}
            }
        }
        (errors, finals)
    })
}

#[tokio::test]
async fn reset_during_socket_setup_prevents_the_response_create_message() -> TestResult {
    // @step Given a Codex stream is waiting for its socket handshake
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let url = format!("http://{}", listener.local_addr()?);
    let started = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Semaphore::new(0));
    let messages = Arc::new(AtomicUsize::new(0));
    let server = tokio::spawn({
        let started = started.clone();
        let release = release.clone();
        let messages = messages.clone();
        async move {
            let (connection, _) = listener.accept().await?;
            started.notify_one();
            let permit = release.acquire().await?;
            permit.forget();
            let mut socket = tokio_tungstenite::accept_async(connection).await?;
            if let Some(Ok(tokio_tungstenite::tungstenite::Message::Text(_))) = socket.next().await
            {
                messages.fetch_add(1, Ordering::SeqCst);
                socket.send(tokio_tungstenite::tungstenite::Message::Text(
                    serde_json::json!({"type":"response.completed","response":fixture::baseline::response()}).to_string().into()
                )).await?;
            }
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        }
    });
    let history = CodexHistory::default();
    let task = consume(model(
        RefreshingCodexClient::new_api_key().with_websocket(CodexWebSocketHandle::default()),
        &url,
        &history,
    )?);
    tokio::time::timeout(std::time::Duration::from_secs(4), started.notified()).await?;
    // @step When its generation is reset before the handshake is released
    history.reset()?;
    let reset = history.snapshot()?;
    release.add_permits(1);
    let (errors, finals) = tokio::time::timeout(std::time::Duration::from_secs(4), task).await??;
    let server_result = tokio::time::timeout(std::time::Duration::from_secs(4), server).await??;
    assert!(server_result.is_ok(), "{server_result:?}");
    // @step Then the stream fails without a response create message
    assert_eq!(messages.load(Ordering::SeqCst), 0, "{errors:?}");
    assert_eq!(errors.len(), 1);
    assert_eq!(finals, 0);
    // @step And the reset checkpoint remains unchanged
    assert_eq!(history.snapshot()?, reset);
    Ok(())
}
