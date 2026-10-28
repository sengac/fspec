use rig::message::{AssistantContent, Message};

#[derive(Debug, Clone)]
pub struct ReplayInstallation {
    pub revision: u64,
    pub runtime_prefix_len: usize,
    pub replay_len: usize,
    prefix_fingerprint: String,
    replay_fingerprint: String,
}

fn canonical_messages(messages: &[Message]) -> Vec<Message> {
    let mut canonical = messages.to_vec();
    for message in &mut canonical {
        if let Message::Assistant { content, .. } = message {
            for block in content.iter_mut() {
                if let AssistantContent::ToolCall(call) = block {
                    if call.signature.as_deref()
                        == Some(codelet_core::gemini_history_hook::SYNTHETIC_THOUGHT_SIGNATURE)
                    {
                        call.signature = None;
                    }
                }
            }
        }
    }
    canonical
}

/// Gemini's documented placeholder is transport decoration, not signed provider
/// evidence. All genuine signatures, call identities and tool inputs remain exact.
pub fn same_replay_messages(left: &[Message], right: &[Message]) -> bool {
    canonical_messages(left) == canonical_messages(right)
}

fn fingerprint(messages: &[Message]) -> Result<String, String> {
    serde_json::to_vec(&canonical_messages(messages))
        .map(|bytes| codelet_core::persistence::compute_hash(&bytes))
        .map_err(|error| error.to_string())
}

impl ReplayInstallation {
    pub fn new(revision: u64, prefix: &[Message], replay: &[Message]) -> Result<Self, String> {
        Ok(Self {
            revision,
            runtime_prefix_len: prefix.len(),
            replay_len: replay.len(),
            prefix_fingerprint: fingerprint(prefix)?,
            replay_fingerprint: fingerprint(replay)?,
        })
    }

    /// Locate the exact previously installed replay when runtime reminders have
    /// shifted. An authoritative incoming prefix must match the saved SHA-256
    /// identity; missing or duplicate matches are conflicts, never fallback.
    pub fn locate(
        &self,
        runtime: &[Message],
        incoming: &[Message],
    ) -> Result<(usize, usize), String> {
        let end = self
            .runtime_prefix_len
            .checked_add(self.replay_len)
            .ok_or("installed replay range overflow")?;
        if let (Some(prefix), Some(replay)) = (
            runtime.get(..self.runtime_prefix_len),
            runtime.get(self.runtime_prefix_len..end),
        ) {
            if fingerprint(prefix)? == self.prefix_fingerprint
                && fingerprint(replay)? == self.replay_fingerprint
            {
                return Ok((self.runtime_prefix_len, end));
            }
        }
        if self.replay_len == 0 {
            return Err("runtime prefix changed without an installed replay anchor".into());
        }
        let anchor = incoming
            .get(..self.replay_len)
            .ok_or("rewritten replay cannot identify the installed history")?;
        if fingerprint(anchor)? != self.replay_fingerprint {
            return Err("rewritten replay cannot identify the installed history".into());
        }
        let runtime = canonical_messages(runtime);
        let anchor = canonical_messages(anchor);
        let mut matches = runtime
            .windows(self.replay_len)
            .enumerate()
            .filter(|(_, window)| *window == anchor.as_slice());
        let (start, _) = matches
            .next()
            .ok_or("installed replay anchor no longer exists")?;
        if matches.next().is_some() {
            return Err("installed replay anchor is ambiguous in runtime history".into());
        }
        Ok((start, start + self.replay_len))
    }
}
