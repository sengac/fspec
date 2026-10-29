//! Feature: spec/features/preserve-cli-contracts-while-integrating-precise-coverage.feature

#[path = "refac020/front_door.rs"]
mod front_door;
#[path = "refac020/generation.rs"]
mod generation;
#[path = "refac020/help.rs"]
mod help;
#[path = "refac020/selectors.rs"]
mod selectors;
#[path = "refac020/support.rs"]
mod support;

type TestResult = Result<(), Box<dyn std::error::Error>>;
