//! Typed MCP-facing dispatch. Generated method modules are audited against FspecService.
use crate::FspecServiceClient;
use codelet_rpc_types::*;
use serde_json::Value;
#[derive(Debug, thiserror::Error)]
pub enum InvokeError {
    #[error("Invalid arguments: {0}")]
    Arguments(String),
    #[error("Transport failed; execution may be uncertain: {0}")]
    Transport(String),
    #[error("Daemon rejected operation: {0}")]
    Daemon(String),
    #[error("Result serialization failed after execution: {0}")]
    Serialization(String),
    #[error("Unknown RPC method")]
    Unknown,
}
pub struct Method {
    pub name: &'static str,
    pub schema: Value,
    pub read_only: bool,
    pub destructive: bool,
    pub request_bound: bool,
}
mod catalog;
mod dispatch;
mod group_00;
mod group_01;
mod group_02;
mod group_03;
mod group_04;
mod group_05;
mod group_06;
mod group_07;
mod group_08;
mod group_09;
mod group_10;
mod group_11;
mod group_12;
mod group_13;
mod group_14;
mod group_15;
mod group_16;
mod group_17;
mod group_18;
mod group_19;
mod group_20;
mod group_21;
mod group_22;
mod group_23;
mod group_24;
mod group_25;
mod group_26;
mod group_27;
mod group_28;
mod group_29;
mod group_30;
mod group_31;
mod group_32;
mod group_33;
mod group_34;
mod group_35;
mod group_36;
mod group_37;
mod group_38;
mod group_39;
mod group_40;
mod validate;
pub use catalog::methods;
pub use dispatch::invoke;
pub use validate::validate;
pub const INTERFACE: &str = env!("FSPEC_RPC_SCHEMA");
