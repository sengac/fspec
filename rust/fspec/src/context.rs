//! `context` shell-facing CLI bridge (CLI-019).
//!
//! Feature: spec/features/load-acceptance-criteria-with-fspec-context.feature
//!
//! clap parses the flags into [`Flags`], which serialises to the JSON args
//! [`codelet_fspec_core::commands::context::run`] takes — the same function
//! the LLM dispatcher calls. The bridge prints the page it returns (exit 0)
//! or `Error: <reason>` on stderr (exit 1).

use std::path::PathBuf;

use anyhow::Result;
use codelet_fspec_core::commands::context;
use serde::Serialize;

use crate::common::{render_core_error, spec_project_root};

/// The flags of `fspec context`.
#[derive(Debug, Default, clap::Args, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Flags {
    /// Cucumber tag expression, e.g. '@auth and not @wip'.
    #[arg(long, value_name = "EXPRESSION", allow_hyphen_values = true)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<String>,
    /// Case-insensitive substring of a scenario's name or steps.
    #[arg(long, value_name = "TEXT", allow_hyphen_values = true)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Regular expression matched against a scenario's name or steps.
    #[arg(long, value_name = "PATTERN", allow_hyphen_values = true)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regex: Option<String>,
    /// Feature file name, e.g. user-login.
    #[arg(long, value_name = "NAME")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feature: Option<String>,
    /// Output format: markdown (default), text or json.
    #[arg(long, value_name = "FORMAT", value_parser = ["markdown", "text", "json"])]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// The page to print (default 1).
    #[arg(long, value_name = "NUMBER", value_parser = clap::value_parser!(u64).range(1..))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u64>,
    /// The most characters on one page (default 40000).
    #[arg(long, value_name = "CHARS", value_parser = clap::value_parser!(u64).range(1..))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<u64>,
}

/// Entry point of the `fspec context` subcommand; returns the exit code.
pub async fn run(flags: Flags) -> Result<u8> {
    let project_root: PathBuf = spec_project_root()?;
    let args_json = serde_json::to_string(&flags)?;
    match context::run(&args_json, &project_root).await {
        Ok(page) => {
            print!("{page}");
            Ok(0)
        }
        Err(err) => {
            eprintln!("Error: {}", render_core_error(&err));
            Ok(1)
        }
    }
}
