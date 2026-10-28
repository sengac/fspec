//! Shared fixture API for provider unit tests only. No shipping module is exposed.
//! Exporting once avoids partially used private copies of integration fixtures.
#[path = "../tests/prov159_support/mod.rs"]
pub mod auth;
#[path = "../tests/prov159_support/network.rs"]
pub mod network;
#[path = "../tests/fixtures/prov148.rs"]
pub mod transport;
