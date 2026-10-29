use codelet_rpc::mcp_dispatch::InvokeError;
use serde_json::{json, Value};

use super::{fixture::Fixture, TestResult};

pub struct Case {
    pub name: &'static str,
    pub id: &'static str,
    pub attach_workspace: bool,
    mutation: Mutation,
    category: &'static [&'static str],
    context: &'static str,
}

enum Mutation {
    None,
    Remove,
    Directory,
    Bytes(&'static [u8]),
    Json(Value),
}

impl Case {
    pub fn prepare(&self, fixture: &Fixture) -> TestResult {
        match &self.mutation {
            Mutation::None => {}
            Mutation::Remove => std::fs::remove_file(fixture.store())?,
            Mutation::Directory => {
                std::fs::remove_file(fixture.store())?;
                std::fs::create_dir(fixture.store())?;
            }
            Mutation::Bytes(bytes) => std::fs::write(fixture.store(), bytes)?,
            Mutation::Json(value) => std::fs::write(fixture.store(), serde_json::to_vec(value)?)?,
        }
        Ok(())
    }

    pub fn check(&self, result: Result<Value, InvokeError>) -> TestResult {
        let message = match result {
            Err(InvokeError::Daemon(message)) => message,
            other => {
                return Err(
                    format!("expected explicit daemon loader error, received {other:?}").into(),
                )
            }
        };
        let lower = message.to_lowercase();
        for category in self.category {
            if !lower.contains(*category) {
                return Err(format!("error lacks category {category}: {message}").into());
            }
        }
        if !message.contains(self.context) {
            return Err(format!("error lacks target {}: {message}", self.context).into());
        }
        if !self.id.is_empty() && !message.contains(self.id) {
            return Err(format!("error lacks requested card {}: {message}", self.id).into());
        }
        Ok(())
    }
}

pub fn cases() -> Vec<Case> {
    vec![
        Case {
            name: "missing workspace",
            id: "AUTH-001",
            attach_workspace: false,
            mutation: Mutation::None,
            category: &["workspace", "unavailable"],
            context: "AUTH-001",
        },
        Case {
            name: "missing card",
            id: "AUTH-404",
            attach_workspace: true,
            mutation: Mutation::None,
            category: &["work unit", "missing"],
            context: "AUTH-404",
        },
        Case {
            name: "missing store",
            id: "AUTH-001",
            attach_workspace: true,
            mutation: Mutation::Remove,
            category: &["read"],
            context: "work-units.json",
        },
        Case {
            name: "directory store",
            id: "AUTH-001",
            attach_workspace: true,
            mutation: Mutation::Directory,
            category: &["read"],
            context: "work-units.json",
        },
        Case {
            name: "malformed JSON",
            id: "AUTH-001",
            attach_workspace: true,
            mutation: Mutation::Bytes(b"{"),
            category: &["json"],
            context: "work-units.json",
        },
        Case {
            name: "non-object root",
            id: "AUTH-001",
            attach_workspace: true,
            mutation: Mutation::Json(json!([])),
            category: &["invalid store"],
            context: "work-units.json",
        },
        Case {
            name: "missing workUnits",
            id: "AUTH-001",
            attach_workspace: true,
            mutation: Mutation::Json(json!({})),
            category: &["invalid store"],
            context: "work-units.json",
        },
        Case {
            name: "non-object workUnits",
            id: "AUTH-001",
            attach_workspace: true,
            mutation: Mutation::Json(json!({"workUnits": []})),
            category: &["invalid store"],
            context: "work-units.json",
        },
        Case {
            name: "non-object record",
            id: "AUTH-001",
            attach_workspace: true,
            mutation: Mutation::Json(json!({"workUnits": {"AUTH-001": null}})),
            category: &["invalid store"],
            context: "AUTH-001",
        },
        Case {
            name: "empty identifier",
            id: "",
            attach_workspace: true,
            mutation: Mutation::None,
            category: &["id", "empty"],
            context: "empty",
        },
    ]
}
