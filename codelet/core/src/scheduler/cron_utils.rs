//! Cron utility functions — shared by engine and catch_up modules
//!
//! Provides timezone-aware cron evaluation helpers extracted from engine.rs
//! to avoid duplication across scheduler modules.

use chrono::Utc;
use chrono_tz::Tz;
use croner::Cron;
use tracing::error;

/// Maximum session limit for agent jobs.
pub const MAX_SESSIONS: usize = 10;

/// Find the most recent cron trigger time before the given datetime.
///
/// Uses a 48-hour lookback window, which is sufficient for most cron expressions
/// (hourly, daily, etc.). Returns None if no trigger is found in the window.
pub fn find_previous_trigger(
    cron: &Cron,
    now: chrono::DateTime<chrono_tz::Tz>,
) -> Option<chrono::DateTime<chrono_tz::Tz>> {
    let lookback = now - chrono::Duration::hours(48);

    let mut prev = None;
    let iter = cron.clone().iter_from(lookback);
    for trigger_time in iter {
        if trigger_time >= now {
            break;
        }
        prev = Some(trigger_time);
    }
    prev
}

/// Parse a cron expression, returning an error string on failure.
pub fn parse_cron(expr: &str, context: &str) -> Result<Cron, String> {
    Cron::new(expr).parse().map_err(|e| {
        error!("Invalid cron '{expr}' for {context}: {e}");
        format!("Invalid cron: {e}")
    })
}

/// Parse an IANA timezone string, returning an error string on failure.
pub fn parse_timezone(tz_str: &str, context: &str) -> Result<Tz, String> {
    tz_str.parse::<Tz>().map_err(|_| {
        error!("Invalid timezone '{tz_str}' for {context}");
        format!("Invalid timezone: {tz_str}")
    })
}

/// Determine if a schedule should trigger based on its cron expression,
/// timezone, and last run time.
///
/// Returns true if:
/// - The schedule has never run (last_run_at is None)
/// - The last run was before the most recent cron trigger time
pub fn should_trigger(
    cron: &Cron,
    timezone: &Tz,
    last_run_at: Option<&str>,
    now: chrono::DateTime<Utc>,
) -> bool {
    match last_run_at {
        None => true, // Never run before — trigger immediately
        Some(last_run_str) => {
            match last_run_str.parse::<chrono::DateTime<Utc>>() {
                Ok(last_run) => {
                    let now_in_tz = now.with_timezone(timezone);
                    match find_previous_trigger(cron, now_in_tz) {
                        Some(prev_trigger) => {
                            let prev_trigger_utc = prev_trigger.with_timezone(&Utc);
                            last_run < prev_trigger_utc
                        }
                        None => false,
                    }
                }
                Err(_) => {
                    error!("Invalid last_run_at '{}'", last_run_str);
                    true // Treat as never run
                }
            }
        }
    }
}
