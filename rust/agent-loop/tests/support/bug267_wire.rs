use serde_json::json;
use wiremock::ResponseTemplate;

pub const SCRIPT: &str = r#"
fn build_url(config) { config.base_url + "/script" }
fn build_headers(config) { #{ "Content-Type": "application/json", "X-Script-Model": config.model } }
fn build_request(request) { #{ script_marker: "selected-script", messages: request.messages, tools: request.tools } }
fn build_stream_request(request) { #{ script_marker: "selected-script", messages: request.messages, tools: request.tools, stream: true } }
fn parse_response(raw) { #{ content: "", stop_reason: "end_turn" } }
fn map_error(status, body) { #{ type: "api", message: body } }
fn define_tools(config) { [] }
fn parse_stream_chunk(config, data) {
    if data == "call" { return #{ kind: "tool_call_delta", index: 0, id: "selected-done",
        name: "Done", arguments: "{\"summary\":\"selected script complete\",\"work_unit_id\":\"NONE\"}" }; }
    if data == "stop" { return #{ kind: "stop", reason: "tool_use" }; }
    if data == "usage" { return #{ kind: "usage", input_tokens: 12, output_tokens: 3 }; }
    throw "unexpected script payload";
}
"#;

pub fn response(body: &serde_json::Value) -> ResponseTemplate {
    if body.get("script_marker").is_some() {
        return ResponseTemplate::new(200)
            .insert_header("content-type", "text/event-stream")
            .set_body_string("data: call\n\ndata: stop\n\ndata: usage\n\ndata: [DONE]\n\n");
    }
    let call = json!({"id":"selected-done","type":"function","function":{
        "name":"Done","arguments":json!({"summary":"selected native complete","work_unit_id":"NONE"}).to_string()}});
    let usage = json!({"prompt_tokens":12,"completion_tokens":3,"total_tokens":15});
    if body["stream"] == true {
        let mut call = call;
        call["index"] = json!(0);
        let frame = json!({"id":"local-native","object":"chat.completion.chunk","created":1,"model":"wire-model",
            "choices":[{"index":0,"delta":{"tool_calls":[call]},"finish_reason":"tool_calls"}],"usage":usage});
        ResponseTemplate::new(200)
            .insert_header("content-type", "text/event-stream")
            .set_body_string(format!("data: {frame}\n\ndata: [DONE]\n\n"))
    } else {
        ResponseTemplate::new(200).set_body_json(json!({"id":"local-native","object":"chat.completion","created":1,"model":"wire-model",
            "choices":[{"index":0,"message":{"role":"assistant","tool_calls":[call]},"finish_reason":"tool_calls"}],"usage":usage}))
    }
}
