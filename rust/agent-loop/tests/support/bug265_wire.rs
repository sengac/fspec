use serde_json::json;
use wiremock::ResponseTemplate;

#[derive(Clone, Copy)]
pub enum Reply {
    Text,
    RejectDone,
    Overflow,
    Loop,
    Failure,
}

fn event(delta: serde_json::Value, reason: serde_json::Value) -> String {
    format!(
        "data: {}\n\n",
        json!({
            "id":"accounting", "object":"chat.completion.chunk", "created":1, "model":"o3",
            "choices":[{"index":0,"delta":delta,"finish_reason":reason}]
        })
    )
}

pub fn response(reply: Reply, request: usize) -> ResponseTemplate {
    if matches!(reply, Reply::Overflow | Reply::Failure) {
        let (code, message) = if matches!(reply, Reply::Overflow) {
            (
                "context_length_exceeded",
                "request exceeds the context window",
            )
        } else {
            ("bad_request", "synthetic terminal fixture failure")
        };
        return ResponseTemplate::new(400)
            .set_body_json(json!({"error":{"code":code,"message":message}}));
    }
    let mut frames = match reply {
        Reply::RejectDone => event(
            json!({"role":"assistant","tool_calls":[{
                "index":0,"id":format!("rejected-{request}"),"type":"function",
                "function":{"name":"Done","arguments":json!({
                    "summary":"unsupported completion claim","work_unit_id":"NONE"
                }).to_string()}
            }]}),
            json!(null),
        ),
        Reply::Loop => {
            let prefix = "The architecture of the streaming loop detector relies on word level tokenization with a bounded sliding window that evaluates four distinct repetition signals in order of specificity before escalating";
            let text = format!("{prefix} {}", "the model thinks that ".repeat(100));
            text.split_whitespace()
                .map(|word| event(json!({"content":format!("{word} ")}), json!(null)))
                .collect()
        }
        _ => event(
            json!({"role":"assistant","content":"Task remains unfinished"}),
            json!(null),
        ),
    };
    if !matches!(reply, Reply::Loop) {
        frames.push_str(&event(
            json!({}),
            json!(if matches!(reply, Reply::RejectDone) {
                "tool_calls"
            } else {
                "stop"
            }),
        ));
        frames.push_str(&format!(
            "data: {}\n\n",
            json!({"choices":[],"usage":{"prompt_tokens":100,"completion_tokens":10,"total_tokens":110}})
        ));
    }
    frames.push_str("data: [DONE]\n\n");
    ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_string(frames)
}
