use serde_json::Value;

pub(super) fn sanitize(text: &str, payloads: bool) -> String {
    let mut output = String::new();
    let mut remaining = text;
    let mut cursor = 0;
    while let Some(offset) = remaining[cursor..].find(['{', '[', '"']) {
        let start = cursor + offset;
        let candidate = &remaining[start..];
        let mut stream = serde_json::Deserializer::from_str(candidate).into_iter::<Value>();
        let parsed = stream
            .next()
            .and_then(Result::ok)
            .map(|value| (value, stream.byte_offset()))
            .or_else(|| debug_string(candidate));
        if let Some((mut value, consumed)) = parsed {
            if matches!(&value, Value::String(text) if !text.contains(['{', '['])) {
                cursor = start + consumed;
                continue;
            }
            output.push_str(&super::redact(&remaining[..start]));
            scrub(&mut value, payloads);
            output.push_str(
                &serde_json::to_string(&value).unwrap_or_else(|_| "[OMITTED JSON]".to_string()),
            );
            remaining = &candidate[consumed..];
            cursor = 0;
        } else {
            cursor = start + 1;
        }
    }
    output.push_str(&super::redact(remaining));
    output
}

fn debug_string(candidate: &str) -> Option<(Value, usize)> {
    candidate.strip_prefix('"')?;
    let mut encoded = String::from("\"");
    let mut characters = candidate.char_indices().skip(1).peekable();
    while let Some((index, character)) = characters.next() {
        match character {
            '"' => {
                encoded.push('"');
                return serde_json::from_str::<String>(&encoded)
                    .ok()
                    .map(|text| (Value::String(text), index + 1));
            }
            '\\' => {
                let (_, escaped) = characters.next()?;
                if escaped == 'u' && characters.peek().is_some_and(|(_, next)| *next == '{') {
                    characters.next()?;
                    let mut digits = String::new();
                    loop {
                        let (_, digit) = characters.next()?;
                        if digit == '}' {
                            break;
                        }
                        if !digit.is_ascii_hexdigit() || digits.len() >= 6 {
                            return None;
                        }
                        digits.push(digit);
                    }
                    let character = char::from_u32(u32::from_str_radix(&digits, 16).ok()?)?;
                    let replacement = serde_json::to_string(&character.to_string()).ok()?;
                    encoded.push_str(replacement.strip_prefix('"')?.strip_suffix('"')?);
                } else {
                    encoded.push('\\');
                    encoded.push(escaped);
                }
            }
            _ => encoded.push(character),
        }
    }
    None
}

fn scrub(value: &mut Value, payloads: bool) {
    match value {
        Value::Object(fields) => {
            for (name, value) in fields {
                if super::credential(name) {
                    *value = Value::String("[REDACTED]".to_string());
                } else if super::payload(name) && !payloads {
                    *value = Value::String("[OMITTED PAYLOAD]".to_string());
                } else {
                    scrub(value, payloads);
                    if super::payload(name) {
                        let text = match &*value {
                            Value::String(text) => text.clone(),
                            other => other.to_string(),
                        };
                        if text.chars().count() > 4096 {
                            *value = Value::String(format!(
                                "{}[TRUNCATED]",
                                text.chars().take(4096).collect::<String>()
                            ));
                        }
                    }
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                scrub(value, payloads);
            }
        }
        Value::String(text) => *text = sanitize(text, payloads),
        _ => {}
    }
}
