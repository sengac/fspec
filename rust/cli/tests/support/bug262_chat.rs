use rig::{completion::ToolDefinition, tool::Tool};
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::Write as _;

#[derive(Deserialize)]
pub struct Arguments {
    index: usize,
}

pub struct Write(pub std::path::PathBuf);

impl Tool for Write {
    const NAME: &'static str = "WriteOrdered";
    type Error = std::io::Error;
    type Args = Arguments;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.into(),
            description: "Record ordered work in an isolated file".into(),
            parameters: json!({"type":"object","properties":{"index":{"type":"integer"}},"required":["index"]}),
        }
    }

    async fn call(&self, arguments: Arguments) -> Result<String, std::io::Error> {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.0)?;
        writeln!(file, "{}", arguments.index)?;
        Ok("recorded".into())
    }
}

fn call(index: usize) -> Value {
    let (name, arguments) = if index == 32 {
        (
            "Done",
            json!({"summary":"ordered task complete","work_unit_id":"NONE"}),
        )
    } else {
        ("WriteOrdered", json!({"index":index,"work_unit_id":"NONE"}))
    };
    json!({"id":format!("call-{index}"),"type":"function",
        "function":{"name":name,"arguments":arguments.to_string()}})
}

pub fn response() -> Value {
    json!({"id":"local-response","object":"chat.completion","created":0,"model":"local-model",
        "choices":[{"index":0,"message":{"role":"assistant","content":null,
            "tool_calls":(0..34).map(call).collect::<Vec<_>>()},"finish_reason":"tool_calls"}],
        "usage":{"prompt_tokens":20,"completion_tokens":30,"total_tokens":50}})
}

pub fn sse(reason: &str) -> String {
    let mut frames = String::new();
    for suffix in [false, true] {
        for index in (0..34).rev() {
            let call = call(index);
            let arguments = call["function"]["arguments"].as_str().unwrap_or_default();
            let boundary = arguments.len() / 2;
            let fragment = if suffix {
                &arguments[boundary..]
            } else {
                &arguments[..boundary]
            };
            let delta = json!({"index":index,
                "id":if suffix { Value::Null } else { call["id"].clone() },
                "function":{"name":if suffix { Value::Null } else { call["function"]["name"].clone() },
                    "arguments":fragment}});
            frames.push_str(&format!(
                "data: {}\n\n",
                json!({"choices":[{"index":0,"delta":{"tool_calls":[delta]},"finish_reason":null}]})
            ));
        }
    }
    frames.push_str(&format!(
        "data: {}\n\n",
        json!({"choices":[{"index":0,"delta":{},"finish_reason":reason}],
        "usage":{"prompt_tokens":20,"completion_tokens":30,"total_tokens":50}})
    ));
    frames.push_str("data: [DONE]\n\n");
    frames
}
