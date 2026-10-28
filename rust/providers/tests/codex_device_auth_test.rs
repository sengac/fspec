//! Feature: spec/features/device-auth-flow.feature
#[path = "device_protocol/legacy_failures.rs"]
mod failures;
#[path = "device_protocol/legacy_happy.rs"]
mod happy;
#[path = "device_protocol/legacy_poll.rs"]
mod polling;
pub mod prov159_support;
#[path = "device_protocol/support.rs"]
pub mod support;
