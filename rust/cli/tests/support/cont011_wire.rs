use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

#[derive(Clone, Copy, Debug)]
pub enum Kind {
    Claude,
    OAuth,
    Gemini,
    Zai,
}
impl Kind {
    pub const ALL: [Self; 4] = [Self::Claude, Self::OAuth, Self::Gemini, Self::Zai];
}
#[derive(Clone, Copy)]
pub enum Scenario {
    Clean,
    Goal,
    Malformed,
    Cancel,
    Unsettled,
    MalformedAfterTool,
    Safety,
    Budget,
    Denied,
}

pub async fn server(
    kind: Kind,
    scenario: Scenario,
    effect: PathBuf,
    session: uuid::Uuid,
    rejections: Arc<Mutex<Vec<u32>>>,
) -> MockServer {
    let server = MockServer::start().await;
    let count = AtomicUsize::new(0);
    Mock::given(method("POST"))
        .respond_with(move |_: &wiremock::Request| {
            let number = count.fetch_add(1, Ordering::SeqCst);
            if matches!(scenario, Scenario::Denied) && number > 0 {
                return ResponseTemplate::new(400).set_body_json(json!({"error":{"type":"invalid_request_error","code":"fixture_blocked_work","message":"blocked synthetic work cannot complete"}}));
            }
            rejections
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(codelet_tools::done_rejection_count(session));
            if matches!(scenario, Scenario::Malformed) {
                return ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string("data: {invalid-native-json\n\n");
            }
            let write_name = if matches!(kind, Kind::Claude | Kind::OAuth) { "Write" } else { "write_file" };
            let write = |path: &std::path::Path| {
                (
                    write_name.to_owned(),
                    json!({"file_path":path,"content":"native-effect","work_unit_id":"NONE"}),
                )
            };
            let done = |valid: bool| {
                let mut args = json!({"summary":"native complete","work_unit_id":"NONE"});
                if valid {
                    args["evidence"] = json!(["synthetic local verification"]);
                    args["goal_assessment"] =
                        json!("The local goal is satisfied by verified synthetic evidence.");
                }
                ("Done".to_owned(), args)
            };
            let (text, calls) = match scenario {
                Scenario::Clean if number == 0 => ("native progress", vec![]),
                Scenario::Clean if number == 1 => ("", vec![write(&effect)]),
                Scenario::Denied => ("", vec![write(&effect)]),
                Scenario::Goal if number == 0 => ("", vec![done(false)]),
                Scenario::Cancel => (
                    "partial native",
                    vec![done(true), write(&effect.with_file_name("trailing"))],
                ),
                _ => (
                    "",
                    vec![done(true), write(&effect.with_file_name("trailing"))],
                ),
            };
            let mut body = match kind {
                Kind::Claude | Kind::OAuth => claude(text, &calls, number),
                Kind::Gemini => gemini(text, &calls, scenario),
                Kind::Zai => chat(text, &calls, number),
            };
            if matches!(kind, Kind::Claude | Kind::OAuth) && matches!(scenario, Scenario::Unsettled | Scenario::MalformedAfterTool) {
                if let Some(delta) = body.find("\"type\":\"message_delta\"") {
                    let start = body[..delta].rfind("data:").unwrap_or(body.len());
                    body.truncate(start);
                }
                if matches!(scenario, Scenario::MalformedAfterTool) {
                    body.push_str(&event(json!({"type":"message_delta","delta":{"stop_reason":"tool_use","stop_sequence":null},"usage":{"output_tokens":"malformed"}})));
                }
            }
            if matches!(kind, Kind::Gemini) && matches!(scenario, Scenario::MalformedAfterTool) {
                body.push_str("data: {invalid-native-terminal\n\n");
            }
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(body)
        })
        .mount(&server)
        .await;
    server
}

fn event(value: Value) -> String {
    format!("data: {value}\n\n")
}

fn claude(text: &str, calls: &[(String, Value)], number: usize) -> String {
    let mut body = event(
        json!({"type":"message_start","message":{"id":format!("native-{number}"),"role":"assistant","content":[],"model":"claude-sonnet-4-5","stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":12,"output_tokens":0}}}),
    );
    if !text.is_empty() {
        body.push_str(&event(json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}})));
        body.push_str(&event(json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":text}})));
        body.push_str(&event(json!({"type":"content_block_stop","index":0})));
    }
    for (index, (name, arguments)) in calls.iter().enumerate() {
        body.push_str(&event(json!({"type":"content_block_start","index":index,"content_block":{"type":"tool_use","id":format!("native-{number}-{index}"),"name":name,"input":{}}})));
        body.push_str(&event(json!({"type":"content_block_delta","index":index,"delta":{"type":"input_json_delta","partial_json":arguments.to_string()}})));
        body.push_str(&event(json!({"type":"content_block_stop","index":index})));
    }
    body.push_str(&event(json!({"type":"message_delta","delta":{"stop_reason":if calls.is_empty() {"end_turn"} else {"tool_use"},"stop_sequence":null},"usage":{"output_tokens":3}})));
    body.push_str(&event(json!({"type":"message_stop"})));
    body
}

fn gemini(text: &str, calls: &[(String, Value)], scenario: Scenario) -> String {
    let mut parts = Vec::new();
    if !text.is_empty() {
        parts.push(json!({"text":text}));
    }
    parts.extend(calls.iter().map(|(name, args)| json!({"functionCall":{"name":name,"args":args},"thoughtSignature":"native-owned-signature"})));
    let reason = match scenario {
        Scenario::Unsettled | Scenario::MalformedAfterTool => Value::Null,
        Scenario::Safety => json!("SAFETY"),
        Scenario::Budget => json!("MAX_TOKENS"),
        _ => json!("STOP"),
    };
    event(
        json!({"candidates":[{"content":{"role":"model","parts":parts},"finishReason":reason,"index":0}],"usageMetadata":{"promptTokenCount":12,"candidatesTokenCount":3,"totalTokenCount":15}}),
    )
}

fn chat(text: &str, calls: &[(String, Value)], number: usize) -> String {
    let delta = if calls.is_empty() {
        json!({"role":"assistant","content":text})
    } else {
        json!({"role":"assistant","content":text,"tool_calls":calls.iter().enumerate().map(|(index,(name,args))|json!({"index":index,"id":format!("native-{number}-{index}"),"type":"function","function":{"name":name,"arguments":args.to_string()}})).collect::<Vec<_>>()})
    };
    let frame = json!({"id":format!("native-{number}"),"object":"chat.completion.chunk","created":1,"model":"glm-4.7","choices":[{"index":0,"delta":delta,"finish_reason":if calls.is_empty() {"stop"} else {"tool_calls"}}],"usage":{"prompt_tokens":12,"completion_tokens":3,"total_tokens":15}});
    format!("{}data: [DONE]\n\n", event(frame))
}
