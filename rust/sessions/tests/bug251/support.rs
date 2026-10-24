#![allow(dead_code)]
#[path = "../cmpct060/support.rs"]
pub mod replay;

use base64::{engine::general_purpose::STANDARD, Engine};
use codelet_core::persistence::*;
use codelet_rpc_types::SessionId;
use codelet_tools::session_search::originals::{OriginalPage, OriginalsRequest};
pub use replay::TestResult;
use serde_json::{json, Value};

pub fn originals(id: &SessionId) -> Result<OriginalPage, Box<dyn std::error::Error>> {
    Ok(originals::retrieve_originals(OriginalsRequest {
        session_id: id.value.clone(),
        start_turn: Some(0),
        end_turn: Some(0),
        next_turn: None,
        limit: Some(1),
        max_bytes: Some(61_440),
    })?)
}

pub fn images() -> Result<Value, base64::DecodeError> {
    let mut png = STANDARD.decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/l9sAAAAASUVORK5CYII=")?;
    png.resize(10_000, 0);
    Ok(json!([
        {"type":"text","text":"原文 α"},
        {"type":"image","source":{"type":"base64","media_type":"image/png","data":STANDARD.encode(png)}},
        {"type":"image","source":{"type":"base64","media_type":"image/gif","data":"R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7"}}
    ]))
}

pub fn oversized_image() -> Result<Value, Box<dyn std::error::Error>> {
    let images = images()?;
    let mut bytes = STANDARD.decode(images[1]["source"]["data"].as_str().ok_or("PNG data")?)?;
    bytes[16..20].copy_from_slice(&30_000_u32.to_be_bytes());
    Ok(
        json!({"type":"image","source":{"type":"base64","media_type":"image/png","data":STANDARD.encode(bytes)}}),
    )
}

pub fn archive(id: &SessionId, content: Value) -> TestResult {
    codelet_agent_loop::persist::persist_user_content(
        &uuid::Uuid::parse_str(&id.value)?,
        serde_json::from_value(content)?,
    )?;
    Ok(())
}

pub fn media_blocks(wire: &Value) -> Vec<Value> {
    wire["messages"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|message| message["content"].as_array().into_iter().flatten())
        .filter(|block| matches!(block["type"].as_str(), Some("image" | "document")))
        .cloned()
        .collect()
}
