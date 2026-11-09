//! Feature: spec/features/precise-coverage-cli-front-door.feature

#[path = "refac020/help.rs"]
mod help;
#[path = "refac020/scenarios.rs"]
mod scenarios;
#[path = "refac020/support.rs"]
mod support;

type TestResult = Result<(), Box<dyn std::error::Error>>;
