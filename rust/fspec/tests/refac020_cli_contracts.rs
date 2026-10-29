//! Feature: spec/features/preserve-cli-contracts-while-integrating-precise-coverage.feature

#[path = "refac020/contracts.rs"]
mod contracts;
#[path = "refac020/dispatch.rs"]
mod dispatch;
#[path = "refac020/help.rs"]
mod help;
#[path = "refac020/source.rs"]
mod source;
#[path = "refac020/tree.rs"]
mod tree;

type TestResult = Result<(), Box<dyn std::error::Error>>;
