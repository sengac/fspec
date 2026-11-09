//! BOARD-024 — readable labels and scalar text for canonical JSON values.

use serde_json::Value;

/// Turn `camelCase` / `snake_case` keys into readable `Title Words`.
pub fn humanize(key: &str) -> String {
    let mut words: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut previous_lower = false;
    for character in key.chars() {
        if character == '_' || character == '-' || character == ' ' {
            words.push(std::mem::take(&mut current));
            previous_lower = false;
            continue;
        }
        if character.is_uppercase() && previous_lower {
            words.push(std::mem::take(&mut current));
        }
        previous_lower = character.is_lowercase() || character.is_ascii_digit();
        current.push(character);
    }
    words.push(current);
    words
        .into_iter()
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect::<String>()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn scalar(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Null => Some("null".to_owned()),
        Value::Bool(flag) => Some(flag.to_string()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn humanizes_camel_and_snake_keys() {
        assert_eq!(humanize("dependsOn"), "Depends On");
        assert_eq!(humanize("nextItemId"), "Next Item Id");
        assert_eq!(humanize("bounded_context"), "Bounded Context");
        assert_eq!(scalar(&Value::Null).as_deref(), Some("null"));
    }
}
