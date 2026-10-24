fn page_error(message: &str, _: Option<()>) -> String {
    message.to_owned()
}
use serde_json::{json, Value};
use std::hash::{Hash, Hasher};

pub struct Page {
    limit: usize,
    cursor: Option<String>,
    revision: Option<String>,
}

fn invalid_page() -> String {
    page_error(
        "Use limit 1..50 and an existing ID cursor of 1..512 bytes; no other arguments",
        None,
    )
}

impl Page {
    pub fn parse(args: Option<&serde_json::Map<String, Value>>) -> Result<Self, String> {
        let mut page = Self {
            limit: 20,
            cursor: None,
            revision: None,
        };
        if let Some(args) = args {
            for (key, value) in args {
                match key.as_str() {
                    "limit" => {
                        let n = value
                            .as_u64()
                            .filter(|n| (1..=50).contains(n))
                            .ok_or_else(invalid_page)?;
                        page.limit = n as usize;
                    }
                    "cursor" => {
                        let id = value
                            .as_str()
                            .filter(|s| !s.is_empty() && s.len() <= 512)
                            .ok_or_else(invalid_page)?;
                        page.cursor = Some(id.to_owned());
                    }
                    "revision" => {
                        page.revision = Some(
                            value
                                .as_str()
                                .filter(|s| s.len() == 16)
                                .ok_or_else(invalid_page)?
                                .to_owned(),
                        )
                    }
                    _ => return Err(invalid_page()),
                }
            }
        }
        if page.cursor.is_some() && page.revision.is_none() {
            return Err(invalid_page());
        }
        Ok(page)
    }

    pub fn apply(&self, value: Value) -> Result<Value, String> {
        let Some(items) = value.as_array() else {
            return Ok(Value::Null);
        };
        let mut rows = Vec::with_capacity(items.len());
        for item in items {
            let Some(id) = item["id"]
                .as_str()
                .filter(|s| !s.is_empty() && s.len() <= 512)
            else {
                return Ok(Value::Null);
            };
            rows.push((id, item));
        }
        rows.sort_unstable_by(|a, b| a.0.cmp(b.0));
        if rows.windows(2).any(|w| w[0].0 == w[1].0) {
            return Ok(Value::Null);
        }
        let revision = revision(
            &serde_json::to_string(&rows.iter().map(|r| r.1).collect::<Vec<_>>())
                .map_err(|_| invalid_page())?,
        );
        if self.revision.as_ref().is_some_and(|r| r != &revision) {
            return Err(page_error(
                "Workspace changed; restart pagination without cursor/revision",
                None,
            ));
        }
        let start = match &self.cursor {
            Some(cursor) => rows
                .binary_search_by(|row| row.0.cmp(cursor.as_str()))
                .map(|index| index + 1)
                .map_err(|_| invalid_page())?,
            None => 0,
        };
        let end = (start + self.limit).min(rows.len());
        let summaries: Vec<Value> = rows[start..end]
            .iter()
            .map(|(id, item)| {
                let bounded = |key: &str, max: usize| {
                    item[key]
                        .as_str()
                        .unwrap_or_default()
                        .chars()
                        .take(max)
                        .collect::<String>()
                };
                json!({"id": id, "title": bounded("title", 256),
                "status": bounded("status", 64), "workType": bounded("workType", 64),
                    "truncated_fields": ([ ("title",256), ("status",64), ("workType",64) ].iter()
                        .filter(|(key,max)| item[*key].as_str().is_some_and(|s| s.chars().count() > *max))
                        .map(|(key,_)| *key).collect::<Vec<_>>())})
            })
            .collect();
        let next = if end < rows.len() {
            Some(rows[end - 1].0)
        } else {
            None
        };
        Ok(
            json!({"items": summaries, "total": rows.len(), "next_cursor": next, "returned_count": summaries.len(), "has_more": next.is_some(), "revision": revision, "summary_only": true, "omitted_fields": ["description", "estimate", "epic", "attachments", "lastStateChangeAt"], "backend_fetch": "full_list"}),
        )
    }
}

// Change detection only, not a security token. Restart after server/version changes.
fn revision(text: &str) -> String {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hash);
    format!("{:016x}", hash.finish())
}

pub struct Detail {
    id: String,
    offset: usize,
    revision: Option<String>,
}
impl Detail {
    pub fn parse(args: Option<&serde_json::Map<String, Value>>) -> Result<Self, String> {
        let args = args.ok_or_else(invalid_page)?;
        if args
            .keys()
            .any(|k| !["id", "offset", "revision"].contains(&k.as_str()))
        {
            return Err(invalid_page());
        }
        let id = args
            .get("id")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty() && s.len() <= 512)
            .ok_or_else(invalid_page)?
            .to_owned();
        let offset = match args.get("offset") {
            None => 0,
            Some(v) => v
                .as_u64()
                .and_then(|n| usize::try_from(n).ok())
                .ok_or_else(invalid_page)?,
        };
        let rev = match args.get("revision") {
            None => None,
            Some(v) => Some(
                v.as_str()
                    .filter(|s| s.len() == 16)
                    .ok_or_else(invalid_page)?
                    .to_owned(),
            ),
        };
        if offset > 0 && rev.is_none() {
            return Err(invalid_page());
        }
        Ok(Self {
            id,
            offset,
            revision: rev,
        })
    }
    pub fn apply(&self, value: Value) -> Result<Value, String> {
        let items = value.as_array().ok_or_else(invalid_page)?;
        let matches: Vec<_> = items
            .iter()
            .filter(|item| item["id"].as_str() == Some(&self.id))
            .collect();
        if matches.len() != 1 {
            return Err(page_error("Work unit missing or ID ambiguous", None));
        }
        let text = serde_json::to_string(matches[0]).map_err(|_| invalid_page())?;
        let revision = revision(&text);
        if self.revision.as_ref().is_some_and(|r| r != &revision) {
            return Err(page_error(
                "Record changed; restart detail at offset zero without revision",
                None,
            ));
        }
        let total = text.chars().count();
        if self.offset > total {
            return Err(invalid_page());
        }
        let chunk: String = text.chars().skip(self.offset).take(4096).collect();
        let end = self.offset + chunk.chars().count();
        Ok(
            json!({"id": self.id, "encoding": "json_text", "offset_unit": "unicode_scalar",
            "offset": self.offset, "text": chunk, "total_chars": total, "revision": revision,
            "partial": self.offset > 0 || end < total,
            "next_offset": if end < total { Some(end) } else { None },
            "backend_fetch": "full_list"}),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detail_chunks_and_changed_records() -> Result<(), String> {
        let data = json!([{"id":"A", "description": "😀\n\"".repeat(10000)}]);
        let mut args = json!({"id":"A"});
        let mut joined = String::new();
        loop {
            let page = Detail::parse(args.as_object())?.apply(data.clone())?;
            assert!(
                serde_json::to_string(&page)
                    .map_err(|_| invalid_page())?
                    .len()
                    < 256 * 1024
            );
            joined.push_str(page["text"].as_str().ok_or_else(invalid_page)?);
            if page["next_offset"].is_null() {
                break;
            }
            args["offset"] = page["next_offset"].clone();
            args["revision"] = page["revision"].clone();
        }
        assert_eq!(
            serde_json::from_str::<Value>(&joined).map_err(|_| invalid_page())?,
            data[0]
        );
        assert!(Detail::parse(args.as_object())?
            .apply(json!([{"id":"A", "description":"changed"}]))
            .is_err());
        for args in [
            json!({"id":"A","offset":1}),
            json!({"id":"A","offset":-1}),
            json!({"id":"A","offset":1.5}),
            json!({"id":"A","unknown":1}),
            json!({}),
        ] {
            assert!(Detail::parse(args.as_object()).is_err());
        }
        assert!(Detail::parse(json!({"id":"missing"}).as_object())?
            .apply(data)
            .is_err());
        Ok(())
    }
    #[test]
    fn changed_lists_reject_continuation() -> Result<(), String> {
        let first =
            Page::parse(json!({"limit":1}).as_object())?.apply(json!([{"id":"B"}, {"id":"A"}]))?;
        let args = json!({"limit":1,"cursor":first["next_cursor"],"revision":first["revision"]});
        let request = Page::parse(args.as_object())?;
        assert!(request
            .apply(json!([{"id":"A"},{"id":"B"},{"id":"C"}]))
            .is_err());
        assert!(request.apply(json!([{"id":"B"}])).is_err());
        assert!(request
            .apply(json!([{"id":"A","title":"changed"},{"id":"B"}]))
            .is_err());
        let last = request.apply(json!([{"id":"A"},{"id":"B"}]))?;
        assert_eq!(last["items"][0]["id"], "B");
        assert!(last["next_cursor"].is_null());
        assert_eq!(Page::parse(None)?.apply(json!([]))?["returned_count"], 0);
        assert!(Page::parse(None)?
            .apply(json!([{"id":"A"},{"id":"A"}]))?
            .is_null());
        Ok(())
    }
}
