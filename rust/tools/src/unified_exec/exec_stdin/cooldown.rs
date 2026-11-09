//! BUG-387: the exec-stdin detector's clock — the re-fire cooldown on the
//! monotonic clock and the request's epoch timestamp (moved out of
//! `exec_stdin.rs` to keep it under 300 lines).

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use once_cell::sync::Lazy;

use super::EXEC_STDIN_COOLDOWN_SECS;

/// Cooldown tracker — per-exec-session last-fire instants on the monotonic
/// clock (BUG-387: no wall-clock error path). The detector task is the
/// sole writer.
pub(super) type CooldownMap = Lazy<std::sync::RwLock<std::collections::HashMap<String, Instant>>>;
pub(super) static LAST_FIRE_MS: CooldownMap =
    Lazy::new(|| std::sync::RwLock::new(std::collections::HashMap::new()));

/// Unix epoch milliseconds of `at` for a request's `ts_ms`. BUG-387: a
/// clock before the epoch is an error, never a 0 timestamp.
pub(super) fn epoch_ms(at: SystemTime) -> Result<u64, String> {
    let since = at
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("the system clock is before the Unix epoch: {error}"))?;
    u64::try_from(since.as_millis())
        .map_err(|error| format!("the system clock is beyond u64 milliseconds: {error}"))
}

/// True when a fire at `fired` is at least the cooldown old at `now`.
pub(super) fn cooldown_has_elapsed(fired: Instant, now: Instant) -> bool {
    now.saturating_duration_since(fired) >= Duration::from_secs(EXEC_STDIN_COOLDOWN_SECS)
}

/// BUG-376: a panic while the cooldown map was locked poisons it. The map
/// only holds last-fire times, so the poisoned guard is recovered, the
/// poison cleared and the recovery logged; the cooldown keeps working.
pub(super) fn report_cooldown_poison() {
    tracing::error!(
        "exec-stdin cooldown lock was poisoned by a panic; recovering its last-fire map"
    );
    LAST_FIRE_MS.clear_poison();
}

/// Read access to the cooldown map (poison recovered, see above).
pub(super) fn cooldown_read(
) -> std::sync::RwLockReadGuard<'static, std::collections::HashMap<String, Instant>> {
    match LAST_FIRE_MS.read() {
        Ok(guard) => guard,
        Err(poisoned) => {
            report_cooldown_poison();
            poisoned.into_inner()
        }
    }
}

/// Write access to the cooldown map (poison recovered, see above).
pub(super) fn cooldown_write(
) -> std::sync::RwLockWriteGuard<'static, std::collections::HashMap<String, Instant>> {
    match LAST_FIRE_MS.write() {
        Ok(guard) => guard,
        Err(poisoned) => {
            report_cooldown_poison();
            poisoned.into_inner()
        }
    }
}

/// True when `exec_id` may fire again (cooldown elapsed or first fire).
pub(super) fn cooldown_elapsed(exec_id: &str) -> bool {
    let now = Instant::now();
    cooldown_read()
        .get(exec_id)
        .is_none_or(|&fired| cooldown_has_elapsed(fired, now))
}

/// Record a fire for `exec_id`.
pub(super) fn record_fire(exec_id: &str) {
    cooldown_write().insert(exec_id.to_string(), Instant::now());
}

/// BUG-171 rule [5]: reset the per-exec-session re-fire cooldown by
/// removing the last-fire record, so the next quiet window can fire
/// IMMEDIATELY without waiting out the previous fire's 30s window.
/// Called on non-exit clears (output resumption): the command resumed,
/// so the previous fire's cooldown no longer guards a fresh prompt.
/// The 30s window still applies within ONE continuous quiet period —
/// a non-exit clear never happens while the session stays quiet, so
/// the cooldown entry only survives continuous quiet.
pub(super) fn reset_cooldown(exec_id: &str) {
    cooldown_write().remove(exec_id);
}
