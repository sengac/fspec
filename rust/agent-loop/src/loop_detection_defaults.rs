//! BUG-419: the RIG-014 loop-detection defaults and the one conversion from
//! the stored per-profile values (PROV-145) to the per-turn wiring.

use codelet_sessions::model_resolution::ProfileLoopDetection;

use crate::background_output::LoopDetectionWiring;

/// Detector on when the profile stores no `loopDetectionEnabled`.
pub const DEFAULT_LOOP_DETECTION_ENABLED: bool = true;
/// Sliding window in words when the profile stores no `loopDetectionWindow`.
pub const DEFAULT_LOOP_DETECTION_WINDOW: u32 = 160;
/// Tail repeat threshold when the profile stores no `loopDetectionMaxRepeats`.
pub const DEFAULT_LOOP_DETECTION_MAX_REPEATS: u32 = 10;
/// Auto-continue retries when the profile stores no `loopDetectionMaxRetries`.
pub const DEFAULT_LOOP_DETECTION_MAX_RETRIES: u32 = 10;

impl From<ProfileLoopDetection> for LoopDetectionWiring {
    fn from(stored: ProfileLoopDetection) -> Self {
        Self {
            enabled: stored.enabled.unwrap_or(DEFAULT_LOOP_DETECTION_ENABLED),
            window: stored.window.unwrap_or(DEFAULT_LOOP_DETECTION_WINDOW),
            max_repeats: stored
                .max_repeats
                .unwrap_or(DEFAULT_LOOP_DETECTION_MAX_REPEATS),
            max_retries: stored
                .max_retries
                .unwrap_or(DEFAULT_LOOP_DETECTION_MAX_RETRIES),
        }
    }
}
