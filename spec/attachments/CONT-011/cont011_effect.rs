use rig::{completion::ToolDefinition, tool::Tool};
use serde::Deserialize;
use serde_json::json;

pub struct RecordedEffect(pub std::path::PathBuf);

#[derive(Deserialize)]
pub struct Arguments {
    trailing: bool,
    work_unit_id: String,
}

impl Tool for RecordedEffect {
    const NAME: &'static str = "RecordFixtureEvidence";
    type Args = Arguments;
    type Output = String;
    type Error = std::io::Error;

    async fn definition(&self, _: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.into(),
            description: "Record an isolated synthetic test effect in the captured temporary root".into(),
            parameters: json!({"type":"object","properties":{"trailing":{"type":"boolean"},"work_unit_id":{"type":"string"}},"required":["trailing","work_unit_id"],"additionalProperties":false}),
        }
    }

    async fn call(&self, args: Arguments) -> Result<String, std::io::Error> {
        if args.work_unit_id != "NONE" {
            return Err(std::io::Error::other("fixture requires explicit NONE attribution"));
        }
        let path = if args.trailing { self.0.with_file_name("trailing") } else { self.0.clone() };
        tokio::fs::write(path, "native-effect").await?;
        Ok("native-effect".into())
    }
}
