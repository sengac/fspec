use serde_json;
use serde::{Serialize, Deserialize};
use codelet_common::web_search::WebSearchAction;

#[derive(Debug, Deserialize, Serialize)]
pub struct WebSearchRequest {
    #[serde(deserialize_with = "deserialize_web_search_action")]
    pub action: WebSearchAction,
}

/// Custom deserializer that handles both JSON object and JSON string formats
/// 
/// This handles the case where Claude passes the action as a JSON string instead of an object
fn deserialize_web_search_action<'de, D>(deserializer: D) -> Result<WebSearchAction, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    use serde_json::Value;
    
    let value = Value::deserialize(deserializer)?;
    
    match value {
        // If it's already an object, deserialize directly
        Value::Object(_) => {
            WebSearchAction::deserialize(value).map_err(D::Error::custom)
        }
        // If it's a string, parse it as JSON first
        Value::String(s) => {
            let parsed: Value = serde_json::from_str(&s).map_err(D::Error::custom)?;
            WebSearchAction::deserialize(parsed).map_err(D::Error::custom)
        }
        _ => Err(D::Error::custom("Expected object or string for action"))
    }
}

fn main() {
    // Test what serde expects for the internally tagged enum
    let action = WebSearchAction::Search {
        query: Some("test query".to_string()),
    };
    
    let action_json = serde_json::to_string_pretty(&action).unwrap();
    println!("WebSearchAction JSON:\n{}", action_json);
    
    let request = WebSearchRequest { action };
    let request_json = serde_json::to_string_pretty(&request).unwrap();
    println!("\nWebSearchRequest JSON:\n{}", request_json);
    
    // Test what format Claude is probably sending
    let claude_format = r#"{"action": "{\"type\": \"search\", \"query\": \"test query\"}"}"#;
    println!("\nClaude format (string): {}", claude_format);
    
    match serde_json::from_str::<WebSearchRequest>(claude_format) {
        Ok(req) => println!("Claude format SUCCESS: {:?}", req),
        Err(e) => println!("Claude format ERROR: {}", e),
    }
}
