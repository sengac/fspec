use super::*;
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn content(
    parts: serde_json::Value,
    usage: serde_json::Value,
) -> Result<super::super::completion::gemini_api_types::Content, Box<dyn std::error::Error>> {
    let response: StreamGenerateContentResponse = serde_json::from_value(json!({
        "candidates": [{"content":{"parts":parts,"role":"model"},"finishReason":"STOP","index":0}],
        "usageMetadata": usage
    }))?;
    assert_eq!(response.candidates.len(), 1);
    response
        .candidates
        .into_iter()
        .next()
        .and_then(|candidate| candidate.content)
        .ok_or_else(|| "candidate should contain content".into())
}

#[test]
fn test_deserialize_stream_response_with_single_text_part() -> TestResult {
    let content = content(
        json!([{"text":"Hello, world!"}]),
        json!({"promptTokenCount":10,"candidatesTokenCount":5,"totalTokenCount":15}),
    )?;
    assert_eq!(content.parts.len(), 1);
    assert!(matches!(&content.parts[0].part, PartKind::Text(text) if text == "Hello, world!"));
    Ok(())
}

#[test]
fn test_deserialize_stream_response_with_multiple_text_parts() -> TestResult {
    let content = content(
        json!([{"text":"Hello, "},{"text":"world!"},{"text":" How are you?"}]),
        json!({"promptTokenCount":10,"candidatesTokenCount":8,"totalTokenCount":18}),
    )?;
    assert_eq!(content.parts.len(), 3);
    for (part, expected) in content
        .parts
        .iter()
        .zip(["Hello, ", "world!", " How are you?"])
    {
        assert!(matches!(&part.part, PartKind::Text(text) if text == expected));
    }
    Ok(())
}

#[test]
fn test_deserialize_stream_response_with_multiple_tool_calls() -> TestResult {
    let content = content(
        json!([
            {"functionCall":{"name":"get_weather","args":{"city":"San Francisco"}}},
            {"functionCall":{"name":"get_temperature","args":{"location":"New York"}}}
        ]),
        json!({"promptTokenCount":50,"candidatesTokenCount":20,"totalTokenCount":70}),
    )?;
    assert_eq!(content.parts.len(), 2);
    for (part, expected) in content.parts.iter().zip(["get_weather", "get_temperature"]) {
        assert!(matches!(&part.part, PartKind::FunctionCall(call) if call.name == expected));
    }
    Ok(())
}

#[test]
fn test_deserialize_stream_response_with_mixed_parts() -> TestResult {
    let content = content(
        json!([
            {"text":"Let me think about this...","thought":true},
            {"text":"Here's my response: "},
            {"functionCall":{"name":"search","args":{"query":"rust async"}}},
            {"text":"I found the answer!"}
        ]),
        json!({"promptTokenCount":100,"candidatesTokenCount":50,"thoughtsTokenCount":15,"totalTokenCount":165}),
    )?;
    let parts = content.parts;
    assert_eq!(parts.len(), 4);
    assert_eq!(parts[0].thought, Some(true));
    assert!(matches!(&parts[0].part, PartKind::Text(text) if text == "Let me think about this..."));
    assert!(matches!(&parts[1].part, PartKind::Text(text) if text == "Here's my response: "));
    assert!(parts[1].thought.is_none() || parts[1].thought == Some(false));
    assert!(matches!(&parts[2].part, PartKind::FunctionCall(call) if call.name == "search"));
    assert!(matches!(&parts[3].part, PartKind::Text(text) if text == "I found the answer!"));
    Ok(())
}

#[test]
fn test_deserialize_stream_response_with_empty_parts() -> TestResult {
    let content = content(
        json!([]),
        json!({"promptTokenCount":10,"candidatesTokenCount":0,"totalTokenCount":10}),
    )?;
    assert!(content.parts.is_empty());
    Ok(())
}
