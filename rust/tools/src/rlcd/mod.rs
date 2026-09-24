//! RLCD — decision-engine integration (service supervisor).
//!
//! Feature: spec/features/rlcd-service-supervisor.feature
//!
//! Backend-agnostic supervisor for the RLCD decision engine (current
//! backend: laya-rs, speaking the Jev/Simple-Jev v1 protocol over
//! `/health` + `/v1/classifier`). Provides:
//!
//! * user-config loading/writing (`config` — `~/.fspec/fspec-config.json`
//!   `rlcd` section, user-config only, key-preserving writes),
//! * health polling + shared status snapshot (`service` — the
//!   `RLCD_STATUS` every consumer reads; fail-open contract),
//! * local auto-spawn with port scanning (`spawn` — local URLs only,
//!   detached processes, no double-spawn),
//! * pidfile + liveness helpers (`pidfile`),
//! * error types (`error` — `thiserror`, structured for fail-open).
//!
//! The `Decision()` rig tool (RLCD-002), the workflow gate (RLCD-003) and
//! the semantic security layer (RLCD-004) are the consumers of this
//! module: they read the shared status snapshot and degrade per the
//! fail-open policy when the engine is unreachable.

pub mod client;
pub mod config;
pub mod decision;
pub mod decision_validate;
pub mod discover;
pub mod error;
pub mod gate;
pub mod gate_config;
pub mod gate_map;
pub mod health;
pub mod pidfile;
pub mod service;
pub mod security;
pub mod security_config;
pub mod spawn;

pub use client::{HttpRlcdClient, RlcdClient, RlcdQuestion, RlcdResponse};
pub use config::{
    load_rlcd_config, load_rlcd_config_from, save_rlcd_url, save_rlcd_url_at, user_dir, RlcdConfig,
    RlcdSpawnConfig,
};
pub use gate_config::RlcdGateConfig;
pub use decision::{DecisionArgs, DecisionTool};
pub use decision_validate::validate_questions;
pub use error::RlcdError;
pub use gate::{
    allow_session_gate, build_gate_state, clear_session_gate_allowances, is_session_gate_allowed,
    run_rlcd_gate, GateVerdict,
};
pub use gate_map::{map_gate_answer, GateOutcome};
pub use pidfile::{is_local_url, is_pid_alive, pidfile_path, read_pidfile, spawn_url_of};
pub use security::{
    check_file_path_semantic, context_user, run_rlcd_security_check, run_rlcd_security_check_sync,
    RLCD_CHECK_ALLOWANCE, RLCD_SECURITY_RULE_ID,
};
pub use security_config::{RlcdSecurityConfig, SecurityThresholds};
pub use service::{global_supervisor, health_check, RlcdStatus, RlcdSupervisor};
pub use spawn::{
    ensure_local_server, select_port, EnsureOutcome, PortSelection,
};
