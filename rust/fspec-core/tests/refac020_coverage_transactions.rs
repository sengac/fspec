// Feature: spec/features/precise-lossless-coverage-mutations.feature
#[path = "refac020/support.rs"]
pub mod support;
#[path = "refac020_coverage_transactions/process.rs"]
mod process;
#[path = "refac020_coverage_transactions/worker.rs"]
mod worker;
#[path = "refac020_coverage_transactions/selected.rs"]
mod selected;
#[path = "refac020_coverage_transactions/legacy.rs"]
mod legacy;
#[path = "refac020_coverage_transactions/generation.rs"]
mod generation;
