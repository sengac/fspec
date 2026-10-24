use serde_json::{json, Value};

pub fn response() -> Value {
    json!({
        "id": "resp_test", "object": "response", "created_at": 1,
        "status": "completed", "model": "shared-model",
        "output": [{"type": "message", "id": "msg_test", "status": "completed",
            "role": "assistant", "content": [{"type": "output_text", "text": "Hello",
            "annotations": []}]}],
        "usage": {"input_tokens": 12, "output_tokens": 8, "total_tokens": 20,
            "input_tokens_details": {"cached_tokens": 3},
            "output_tokens_details": {"reasoning_tokens": 4}}
    })
}

pub fn stream_body() -> String {
    let events = [
        json!({"type": "response.output_text.delta", "output_index": 0,
            "content_index": 0, "sequence_number": 1, "delta": "Hello"}),
        json!({"type": "response.reasoning_summary_text.delta", "output_index": 1,
            "summary_index": 0, "sequence_number": 2, "delta": "Thinking"}),
        json!({"type": "response.output_item.done", "output_index": 2,
            "sequence_number": 3, "item": {"type": "function_call", "id": "fc_test",
                "call_id": "call_test", "name": "inspect", "arguments": "{\"path\":\".\"}",
                "status": "completed"}}),
        json!({"type": "response.completed", "sequence_number": 4, "response": response()}),
    ];
    events
        .iter()
        .map(|event| format!("data: {event}\n\n"))
        .collect()
}
