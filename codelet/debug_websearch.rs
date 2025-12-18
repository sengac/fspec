use serde_json;
use serde::{Serialize, Deserialize};
use codelet_common::web_search::WebSearchAction;

#[derive(Debug, Deserialize, Serialize)]
pub struct WebSearchRequest {
    pub action: WebSearchAction,
}

fn main() {
    let json_str = r#"
{
  "action": {
    "type": "search",
    "query": "test query"
  }
}
"#;

    println!("Testing JSON parsing...");
    
    match serde_json::from_str::<WebSearchRequest>(json_str) {
        Ok(req) => {
            println!("Success: {:?}", req);
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
    
    // Test direct WebSearchAction parsing
    let action_str = r#"
{
  "type": "search",
  "query": "test query"
}
"#;
    
    match serde_json::from_str::<WebSearchAction>(action_str) {
        Ok(action) => {
            println!("Action Success: {:?}", action);
        }
        Err(e) => {
            println!("Action Error: {}", e);
        }
    }
}
