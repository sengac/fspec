//! `foundation-status` shell-facing CLI bridge (DISC-003).
//!
//! Rust-only extension command (not in the 162-command canonical list).
//! Thin façade: marshal the `--json` flag into JSON args, delegate to
//! [`codelet_fspec_core::commands::foundation_status::run`], and print the
//! rendered report verbatim.

use crate::common::spec_project_root;
use std::path::PathBuf;

use anyhow::Result;
use codelet_fspec_core::commands::foundation_status;
use serde_json::json;

#[derive(Debug, Default)]
pub struct CliArgs {
    pub json: bool,
}

pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = spec_project_root()?;

    let args_json = json!({ "json": args.json }).to_string();

    match foundation_status::run(&args_json, &project_root).await {
        Ok(rendered) => {
            println!("{rendered}");
            Ok(0)
        }
        Err(err) => {
            let reason = crate::common::render_core_error(&err);
            eprintln!("Error: {reason}");
            Ok(1)
        }
    }
}
