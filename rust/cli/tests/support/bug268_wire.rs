use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

pub async fn server(sequence: bool, file: &str) -> MockServer {
    let server = MockServer::start().await;
    let count = Arc::new(AtomicUsize::new(0));
    let file = file.to_owned();
    Mock::given(method("POST")).respond_with(move |request: &wiremock::Request| {
        let number = count.fetch_add(1, Ordering::SeqCst);
        let body: Value = serde_json::from_slice(&request.body).unwrap_or(Value::Null);
        let call = if sequence && number != 1 {
            Some(if number == 0 {
                ("Read", json!({"file_path":file,"work_unit_id":"NONE"}))
            } else {
                ("Done", json!({"summary":"copilot complete","work_unit_id":"NONE"}))
            })
        } else { None };
        let usage = json!({"input_tokens":12,"output_tokens":3,"total_tokens":15});
        if request.url.path() == "/responses" {
            let output = match call {
                Some((name, arguments)) => vec![json!({"type":"function_call","id":"item-native","call_id":"call-native","name":name,"arguments":arguments.to_string(),"status":"completed"})],
                None => vec![json!({"type":"message","role":"assistant","id":"answer","status":"completed","content":[{"type":"output_text","text":"answer","annotations":[]}]})],
            };
            let response = json!({"id":"local-response","object":"response","created_at":1,"model":"local","status":"completed","output":output,"usage":usage});
            if body["stream"] == true {
                let mut events = String::new();
                for (index,item) in output.iter().enumerate() {
                    if item["type"] == "function_call" {
                        events.push_str(&format!("data: {}\n\n",json!({"type":"response.output_item.done","sequence_number":index,"output_index":index,"item":item})));
                    } else {
                        events.push_str(&format!("data: {}\n\n",json!({"type":"response.output_text.delta","sequence_number":index,"output_index":index,"content_index":0,"item_id":"answer","delta":"answer"})));
                    }
                }
                events.push_str(&format!("data: {}\n\n",json!({"type":"response.completed","sequence_number":3,"response":response})));
                ResponseTemplate::new(200).insert_header("content-type","text/event-stream").set_body_string(events)
            } else { ResponseTemplate::new(200).set_body_json(response) }
        } else {
            let message = match call {
                Some((name,arguments)) => json!({"role":"assistant","tool_calls":[{"id":"call-native","type":"function","function":{"name":name,"arguments":arguments.to_string()}}]}),
                None => json!({"role":"assistant","content":"answer"}),
            };
            let reason = if message.get("tool_calls").is_some() { "tool_calls" } else { "stop" };
            if body["stream"] == true {
                let mut delta=message;
                if let Some(calls)=delta.get_mut("tool_calls").and_then(Value::as_array_mut) {
                    for (index,call) in calls.iter_mut().enumerate() { call["index"]=json!(index); }
                }
                let frame=json!({"id":"local-chat","object":"chat.completion.chunk","created":1,"model":"local",
                    "choices":[{"index":0,"delta":delta,"finish_reason":reason}],
                    "usage":{"prompt_tokens":12,"completion_tokens":3,"total_tokens":15}});
                return ResponseTemplate::new(200).insert_header("content-type","text/event-stream")
                    .set_body_string(format!("data: {frame}\n\ndata: [DONE]\n\n"));
            }
            ResponseTemplate::new(200).set_body_json(json!({"id":"local-chat","object":"chat.completion","created":1,"model":"local",
                "choices":[{"index":0,"message":message,"finish_reason":reason}],
                "usage":{"prompt_tokens":12,"completion_tokens":3,"total_tokens":15}}))
        }
    }).up_to_n_times(if sequence { 3 } else { 1 }).mount(&server).await;
    server
}
