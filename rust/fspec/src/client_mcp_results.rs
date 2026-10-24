//! Immutable response snapshots: continuation never repeats an RPC.
use serde_json::{json, Value};
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};
const CHUNK: usize = 4096;
const MAX_BYTES: usize = 32 * 1024 * 1024;
struct Entry {
    id: String,
    text: String,
    created: Instant,
}
#[derive(Default)]
pub(crate) struct Results {
    entries: VecDeque<Entry>,
}
impl Results {
    fn expire(&mut self) {
        self.entries
            .retain(|e| e.created.elapsed() < Duration::from_secs(300));
    }
    pub(crate) fn insert(&mut self, value: &Value) -> Result<Value, &'static str> {
        self.expire();
        let text = serde_json::to_string(value).map_err(|_| "Result serialization failed")?;
        if text.len() > MAX_BYTES {
            return Err("Execution returned; result exceeds 32 MiB retention limit. Do not repeat a mutation to recover output.");
        }
        if self.entries.len() >= 8
            || self.entries.iter().map(|e| e.text.len()).sum::<usize>() + text.len() > MAX_BYTES
        {
            return Err("Execution returned; result cache is full. Existing continuations preserved; do not repeat a mutation.");
        }
        let id = uuid::Uuid::new_v4().to_string();
        self.entries.push_back(Entry {
            id: id.clone(),
            text,
            created: Instant::now(),
        });
        let mut page = self.page(&id, 0)?;
        if page["has_more"] == false {
            self.entries.pop_back();
            page["result_id"] = Value::Null;
            page["retention_seconds"] = json!(0);
        }
        Ok(page)
    }
    pub(crate) fn page(&mut self, id: &str, offset: usize) -> Result<Value, &'static str> {
        self.expire();
        let entry = self
            .entries
            .iter()
            .find(|e| e.id == id)
            .ok_or("Unknown or expired result; RPC was not repeated")?;
        let total = entry.text.chars().count();
        if offset > total {
            return Err("Offset exceeds result length");
        }
        let text: String = entry.text.chars().skip(offset).take(CHUNK).collect();
        let end = offset + text.chars().count();
        Ok(
            json!({"result_id":id,"text":text,"offset":offset,"next_offset":if end < total {Some(end)} else {None},
            "returned_chars":end-offset,"total_chars":total,"partial":offset>0 || end<total,"has_more":end<total,
            "offset_unit":"unicode_scalar","encoding":"json_text","retention_seconds":300,
            "backend_fetch":"rpc_result","execution":"returned","retry_performed":false}),
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_continuation_is_lossless_and_stable() -> Result<(), &'static str> {
        let mut results = Results::default();
        let value = json!({"large":"🦀".repeat(10000)});
        let mut page = results.insert(&value)?;
        let id = page["result_id"].as_str().ok_or("id")?.to_owned();
        let mut joined = String::new();
        loop {
            joined.push_str(page["text"].as_str().ok_or("text")?);
            assert!(serde_json::to_vec(&page).map_err(|_| "json")?.len() < 256 * 1024);
            match page["next_offset"].as_u64() {
                Some(n) => page = results.page(&id, n as usize)?,
                None => break,
            }
        }
        assert_eq!(
            serde_json::from_str::<Value>(&joined).map_err(|_| "parse")?,
            value
        );
        assert!(results.page(&id, usize::MAX).is_err());
        assert!(results.page("unknown", 0).is_err());
        results.entries[0].created = Instant::now() - Duration::from_secs(301);
        assert!(results.page(&id, 0).is_err());
        Ok(())
    }
}
