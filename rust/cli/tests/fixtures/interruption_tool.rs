use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Notify;

#[derive(Clone)]
pub struct Execution {
    pub completed: Arc<AtomicUsize>,
    pub started: Arc<AtomicUsize>,
    pub interrupted: Arc<AtomicBool>,
    pub notify: Arc<Notify>,
    pub interrupt_at: Option<usize>,
}

pub struct RecordingTool {
    pub name: &'static str,
    pub execution: Execution,
}

impl Tool for RecordingTool {
    const NAME: &'static str = "inspect";
    type Error = std::io::Error;
    type Args = Value;
    type Output = Value;

    fn name(&self) -> String {
        self.name.into()
    }

    async fn definition(&self, _: String) -> ToolDefinition {
        ToolDefinition {
            name: self.name.into(),
            description: "Record completed tool executions".into(),
            parameters: json!({"type":"object"}),
        }
    }

    async fn call(&self, _: Value) -> Result<Value, Self::Error> {
        let index = self.execution.started.fetch_add(1, Ordering::SeqCst);
        if self.execution.interrupt_at == Some(index) {
            self.execution.interrupted.store(true, Ordering::Release);
            self.execution.notify.notify_one();
            std::future::pending::<()>().await;
        }
        self.execution.completed.fetch_add(1, Ordering::SeqCst);
        Ok(json!({"completed":true}))
    }
}
