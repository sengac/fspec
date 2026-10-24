#[derive(Clone, Default)]
pub(super) struct Redactor {
    secrets: Vec<String>,
}

impl Redactor {
    pub fn from_headers(headers: &http::HeaderMap) -> Self {
        let mut redactor = Self::default();
        for (name, value) in headers {
            if !value.is_sensitive()
                && !matches!(
                    name.as_str(),
                    "authorization"
                        | "proxy-authorization"
                        | "chatgpt-account-id"
                        | "x-api-key"
                        | "api-key"
                        | "cookie"
                )
            {
                continue;
            }
            let value = String::from_utf8_lossy(value.as_bytes());
            redactor.add(&value);
            let value = value.trim();
            redactor.add(value);
            if matches!(name.as_str(), "authorization" | "proxy-authorization") {
                if let Some((_, credential)) = value.split_once(char::is_whitespace) {
                    redactor.add(credential.trim());
                }
            } else if name == "cookie" {
                for pair in value.split(';') {
                    if let Some((_, credential)) = pair.split_once('=') {
                        redactor.add(credential.trim().trim_matches('"'));
                    }
                }
            }
        }
        redactor
    }

    pub fn add(&mut self, secret: &str) {
        if secret.is_empty() {
            return;
        }
        let encoded = serde_json::Value::String(secret.to_owned()).to_string();
        for value in [secret, &encoded[1..encoded.len() - 1]] {
            if !self.secrets.iter().any(|known| known == value) {
                self.secrets.push(value.to_owned());
            }
        }
        self.secrets
            .sort_by_key(|value| std::cmp::Reverse(value.len()));
    }

    pub fn clean(&self, value: &str, limit: usize) -> String {
        redact(value, self.spans(value, false), limit)
    }

    pub fn clean_fragment(&self, value: &str, limit: usize) -> String {
        self.clean_body(value, limit, true)
    }

    pub fn clean_body(&self, value: &str, limit: usize, partial: bool) -> String {
        let mut spans = self.spans(value, partial);
        let decoded = decode_json_escapes(value, partial);
        for span in self.spans(&decoded.value, partial) {
            let start = decoded.origins[span.start].start;
            let end = decoded.origins[span.end - 1].end;
            spans.push(start..end);
        }
        if let Some(tail) = decoded.incomplete {
            if !self.secrets.is_empty() {
                spans.push(tail.clone());
            }
            for secret in &self.secrets {
                for (start, _) in decoded.value.char_indices() {
                    if secret.starts_with(&decoded.value[start..]) {
                        spans.push(decoded.origins[start].start..tail.end);
                    }
                }
            }
        }
        redact(value, spans, limit)
    }

    fn spans(&self, value: &str, partial: bool) -> Vec<Range<usize>> {
        let mut spans = Vec::new();
        for secret in &self.secrets {
            for (start, _) in value.char_indices() {
                let suffix = &value[start..];
                if suffix.starts_with(secret) {
                    spans.push(start..start + secret.len());
                } else if partial && secret.starts_with(suffix) {
                    spans.push(start..value.len());
                }
            }
        }
        spans
    }
}

fn redact(value: &str, mut spans: Vec<Range<usize>>, limit: usize) -> String {
    spans.sort_by_key(|span| (span.start, span.end));
    let mut merged: Vec<Range<usize>> = Vec::new();
    for span in spans {
        if let Some(previous) = merged
            .last_mut()
            .filter(|previous| previous.end >= span.start)
        {
            previous.end = previous.end.max(span.end);
        } else {
            merged.push(span);
        }
    }
    let mut sanitized = String::new();
    let mut cursor = 0;
    for span in merged {
        sanitized.push_str(&value[cursor..span.start]);
        sanitized.push_str("[redacted]");
        cursor = span.end;
    }
    sanitized.push_str(&value[cursor..]);
    sanitized.truncate(sanitized.floor_char_boundary(limit.min(sanitized.len())));
    sanitized
}

struct Decoded {
    value: String,
    origins: Vec<Range<usize>>,
    incomplete: Option<Range<usize>>,
}

fn decode_json_escapes(value: &str, partial: bool) -> Decoded {
    let mut decoded = Decoded {
        value: String::with_capacity(value.len()),
        origins: Vec::with_capacity(value.len()),
        incomplete: None,
    };
    let mut offset = 0;
    while let Some(character) = value[offset..].chars().next() {
        let remaining = &value[offset..];
        let escape = (character == '\\')
            .then(|| {
                [2, 6, 12].into_iter().find_map(|width| {
                    let raw = remaining.get(..width)?;
                    let decoded = serde_json::from_str::<String>(&format!("\"{raw}\"")).ok()?;
                    Some((width, decoded))
                })
            })
            .flatten();
        if let Some((width, escape)) = escape {
            decoded.value.push_str(&escape);
            decoded
                .origins
                .extend(std::iter::repeat_n(offset..offset + width, escape.len()));
            offset += width;
        } else if partial && incomplete_escape(remaining) {
            decoded.incomplete = Some(offset..value.len());
            break;
        } else {
            let end = offset + character.len_utf8();
            decoded.value.push(character);
            decoded
                .origins
                .extend(std::iter::repeat_n(offset..end, character.len_utf8()));
            offset = end;
        }
    }
    decoded
}

fn incomplete_escape(value: &str) -> bool {
    if value == "\\" {
        return true;
    }
    let Some(digits) = value.strip_prefix("\\u") else {
        return false;
    };
    if digits.len() < 4 {
        return digits.bytes().all(|digit| digit.is_ascii_hexdigit());
    }
    let Some(high) = digits
        .get(..4)
        .and_then(|high| u16::from_str_radix(high, 16).ok())
    else {
        return false;
    };
    (0xd800..=0xdbff).contains(&high) && (digits.len() == 4 || incomplete_escape(&digits[4..]))
}
use std::ops::Range;
