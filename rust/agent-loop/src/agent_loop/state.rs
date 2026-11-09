use crate::{dispatch::InputWithImages, terminal_overflow_recovery::RetryInput};

#[derive(Default)]
pub(super) struct LoopState {
    pub compaction_retry_input: Option<RetryInput>,
    pub automatic_recoveries: usize,
    pub loop_abort_retry_count: usize,
    pub turn_epoch: u64,
}

pub(super) struct PendingInput {
    pub input: InputWithImages,
    pub is_retry_input: bool,
    pub is_fresh_user_input: bool,
    pub is_compaction_retry: bool,
    pub manual_compaction_epoch: Option<u64>,
    /// BUG-516: a queued manual /compact request (never a conversation
    /// message); runs the compactor sub-agent round instead of a turn.
    pub is_manual_compaction: bool,
    pub interrupt_epoch: u64,
    pub cancelled_at_submission: bool,
    pub submission: Option<uuid::Uuid>,
}
