// Feature: spec/features/coverage-sidecar-transactions.feature
#[path = "refac020_coverage_transactions/process.rs"]
mod process;
#[path = "refac020_coverage_transactions/scenarios.rs"]
mod scenarios;
#[path = "refac020/support.rs"]
pub mod support;
#[path = "refac020_coverage_transactions/worker.rs"]
mod worker;
