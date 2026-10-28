use rig::agent::{CancelSignal, StreamingPromptHook};
use rig::completion::{CompletionModel, ToolDefinition};
use rig::tool::Tool;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

#[path = "prov148.rs"]
pub mod previous;
pub use previous::{event, Server, TestResult};

#[derive(Clone)]
pub struct ReadTool(pub Arc<Mutex<Vec<Value>>>);

impl Tool for ReadTool {
    const NAME: &'static str = "read_file";
    type Error = std::io::Error;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.into(),
            description: "Read-only execution recorder".into(),
            parameters: json!({"type":"object"}),
        }
    }

    async fn call(&self, args: Value) -> Result<Value, Self::Error> {
        self.0
            .lock()
            .map_err(|_| std::io::Error::other("record lock"))?
            .push(args.clone());
        Ok(args)
    }
}

#[derive(Clone, Default)]
pub struct Gate {
    pub cancel: bool,
    pub calls: Arc<Mutex<usize>>,
    pub results: Arc<Mutex<usize>>,
}

impl<Model: CompletionModel> StreamingPromptHook<Model> for Gate {
    async fn on_tool_call(&self, _: &str, _: Option<String>, _: &str, signal: CancelSignal) {
        let Ok(mut calls) = self.calls.lock() else {
            signal.cancel();
            return;
        };
        *calls += 1;
        if self.cancel {
            signal.cancel();
        }
    }

    async fn on_tool_result(
        &self,
        _: &str,
        _: Option<String>,
        _: &str,
        _: &str,
        signal: CancelSignal,
    ) {
        let Ok(mut results) = self.results.lock() else {
            signal.cancel();
            return;
        };
        *results += 1;
    }
}

pub fn item(id: &str, call: &str, name: &str, arguments: &str) -> Value {
    json!({"type":"function_call","id":id,"call_id":call,"name":name,
        "arguments":arguments,"status":"completed"})
}

pub fn done(item: Value) -> String {
    event(json!({"type":"response.output_item.done","output_index":0,"item":item}))
}

pub fn completed(items: Vec<Value>) -> String {
    event(json!({"type":"response.completed","response":{
        "id":"resp_tools","status":"completed","end_turn":true,"output":items,
        "usage":previous::baseline::response()["usage"]}}))
}
