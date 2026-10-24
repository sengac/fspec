use futures::StreamExt;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use rig::streaming::StreamedAssistantContent;
use serde_json::{json, Value};

#[path = "prov148.rs"]
pub mod transport;
pub use transport::{event, terminal, text, Server, TestResult};

#[derive(Default, Debug)]
pub struct Frontend {
    pub thinking: Vec<(String, String)>,
    pub answer: String,
    pub order: Vec<String>,
    pub errors: Vec<String>,
}

pub async fn consume<Model: CompletionModel<StreamingResponse = transport::Final>>(
    model: Model,
) -> Result<Frontend, Box<dyn std::error::Error>> {
    let request = CompletionRequestBuilder::new(model.clone(), "Hello")
        .preamble("Test instructions".to_owned())
        .build();
    let mut stream = model.stream(request).await?;
    let mut frontend = Frontend::default();
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(StreamedAssistantContent::ReasoningDelta { id, reasoning }) => {
                frontend.order.push(format!("thinking:{reasoning}"));
                frontend.thinking.push((id.unwrap_or_default(), reasoning));
            }
            Ok(StreamedAssistantContent::Text(text)) => {
                frontend.order.push(format!("answer:{}", text.text));
                frontend.answer.push_str(&text.text);
            }
            Err(error) => frontend.errors.push(error.to_string()),
            _ => {}
        }
    }
    Ok(frontend)
}

pub async fn run(events: Vec<Value>) -> Result<Frontend, Box<dyn std::error::Error>> {
    let body = events.into_iter().map(event).collect::<String>()
        + text().as_str()
        + terminal("completed", Some(true), None).as_str();
    let server = Server::start(vec![body], false).await?;
    consume(server.models()?.0).await
}

pub fn summary(kind: &str, item: &str, index: u64, text: &str) -> Value {
    let mut value = json!({"type": format!("response.reasoning_summary_text.{kind}"),
        "item_id": item, "summary_index": index, "output_index": 0, "sequence_number": 1});
    value[if kind == "delta" { "delta" } else { "text" }] = json!(text);
    value
}

pub fn item(item: &str, summaries: &[&str]) -> Value {
    json!({"type":"response.output_item.done", "output_index":0,"sequence_number":2,
        "item":{"type":"reasoning","id":item,"encrypted_content":"opaque-secret",
        "summary":summaries.iter().map(|text|json!({"type":"summary_text","text":text})).collect::<Vec<_>>()}})
}
