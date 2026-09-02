//! TOOL-022 P2 — exec-stdin wire↔internal mapping.
//!
//! Pure pass-through conversion between the wire shape in
//! `codelet_rpc_types` and the tools-internal type in
//! `codelet_tools::unified_exec`. The two structs carry the same
//! fields; `quiet_seconds` / `ts_ms` are `u64` internally and `i64`
//! on the wire (the workspace's `napi(object)` convention — see
//! `LogRecord.timestamp_ms`), so the numeric conversion clamps
//! out-of-range values instead of panicking. No inference, no content
//! derivation — nothing from output content crosses the wire.

use codelet_rpc_types::ExecStdinRequest as WireExecStdinRequest;
use codelet_tools::unified_exec::ExecStdinRequest as InternalExecStdinRequest;

/// Map the internal exec-stdin request to the wire shape.
pub fn internal_request_to_wire(request: InternalExecStdinRequest) -> WireExecStdinRequest {
    WireExecStdinRequest {
        exec_session_id: request.exec_session_id,
        command: request.command,
        // Non-negative values; clamp to i64::MAX on (impractical)
        // overflow rather than panicking.
        quiet_seconds: request.quiet_seconds.min(i64::MAX as u64) as i64,
        ts_ms: request.ts_ms.min(i64::MAX as u64) as i64,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn mapping_passes_fields_through() {
        let internal = InternalExecStdinRequest {
            exec_session_id: "exec-abc".to_string(),
            command: "git commit".to_string(),
            quiet_seconds: 5,
            ts_ms: 1_700_000_000_000,
        };
        let wire = internal_request_to_wire(internal);
        assert_eq!(wire.exec_session_id, "exec-abc");
        assert_eq!(wire.command, "git commit");
        assert_eq!(wire.quiet_seconds, 5);
        assert_eq!(wire.ts_ms, 1_700_000_000_000);
    }

    #[test]
    fn mapping_clamps_overflowing_values_instead_of_panicking() {
        let internal = InternalExecStdinRequest {
            exec_session_id: "e".to_string(),
            command: "c".to_string(),
            quiet_seconds: u64::MAX,
            ts_ms: u64::MAX,
        };
        let wire = internal_request_to_wire(internal);
        assert_eq!(wire.quiet_seconds, i64::MAX);
        assert_eq!(wire.ts_ms, i64::MAX);
    }
}
