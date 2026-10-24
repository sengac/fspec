use super::RequestAdmissionError;
use serde_json::Value;

const INLINE_IMAGE_TOKEN_RESERVATION: u64 = 10_000;

#[derive(Default)]
pub(crate) struct MediaEstimate {
    pub tokens: u64,
}

impl MediaEstimate {
    fn add(&mut self, other: Self) -> Result<(), RequestAdmissionError> {
        self.tokens = self
            .tokens
            .checked_add(other.tokens)
            .ok_or(RequestAdmissionError::OccupancyOverflow)?;
        Ok(())
    }
}

fn inline_image(value: &mut Value) -> Result<MediaEstimate, RequestAdmissionError> {
    let object = value
        .as_object()
        .ok_or(RequestAdmissionError::UnknownMediaOccupancy)?;
    if object
        .keys()
        .any(|key| !matches!(key.as_str(), "type" | "image_url" | "detail"))
        || object.get("detail").is_some_and(|detail| {
            !matches!(detail.as_str(), Some("auto" | "low" | "high" | "original"))
        })
    {
        return Err(RequestAdmissionError::UnknownMediaOccupancy);
    }
    let url = value["image_url"]
        .as_str()
        .ok_or(RequestAdmissionError::UnknownMediaOccupancy)?;
    let (prefix, payload) = url
        .split_once(',')
        .ok_or(RequestAdmissionError::UnknownMediaOccupancy)?;
    if !matches!(
        prefix,
        "data:image/png;base64"
            | "data:image/jpeg;base64"
            | "data:image/gif;base64"
            | "data:image/webp;base64"
    ) || payload.is_empty()
        || !payload.len().is_multiple_of(4)
        || !payload
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'='))
    {
        return Err(RequestAdmissionError::UnknownMediaOccupancy);
    }
    value["image_url"] = Value::String(String::new());
    Ok(MediaEstimate {
        tokens: INLINE_IMAGE_TOKEN_RESERVATION,
    })
}

pub(crate) fn estimate(
    value: &mut Value,
    request_root: bool,
) -> Result<MediaEstimate, RequestAdmissionError> {
    let mut total = MediaEstimate::default();
    match value {
        Value::Object(object) => {
            if object.get("type").and_then(Value::as_str) == Some("input_image") {
                return inline_image(value);
            }
            if object.keys().any(|key| {
                matches!(
                    key.as_str(),
                    "image_url"
                        | "image"
                        | "audio"
                        | "input_audio"
                        | "video"
                        | "inlineData"
                        | "inline_data"
                        | "fileData"
                        | "file_data"
                        | "source"
                        | "file_id"
                )
            }) || object
                .get("type")
                .and_then(Value::as_str)
                .is_some_and(|kind| {
                    matches!(
                        kind,
                        "image"
                            | "image_url"
                            | "input_audio"
                            | "audio"
                            | "video"
                            | "document"
                            | "input_file"
                            | "file"
                    )
                })
            {
                return Err(RequestAdmissionError::UnknownMediaOccupancy);
            }
            for (key, nested) in object {
                if request_root && matches!(key.as_str(), "tools" | "functions") {
                    continue;
                }
                total.add(estimate(nested, false)?)?;
            }
        }
        Value::Array(array) => {
            for nested in array {
                total.add(estimate(nested, false)?)?;
            }
        }
        _ => {}
    }
    Ok(total)
}
