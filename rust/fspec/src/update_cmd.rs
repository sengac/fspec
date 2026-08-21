//! UPD-002: `fspec update` CLI subcommand — headless self-update.
//!
//! Feature: spec/features/in-place-self-update-cli-command.feature
//!
//! Calls the SAME shared `codelet_fspec_core::update` engine as the TUI
//! `/update` command (rule [0]: one engine, no duplication). `--check` is
//! scriptable: exit 0 when current, exit 1 when a newer release is available.

use anyhow::Result;
use codelet_fspec_core::update::{UpdateConfig, UpdateOutcome};

/// Run `fspec update [--check]`.
///
/// `check_only` → only query the latest release and print it; exit 0 when
/// current, exit 1 when a newer release is available. Otherwise download,
/// verify, and replace the installed binary, printing a human-readable
/// result.
///
/// Every path terminates via `std::process::exit`; the `Result` return is
/// only for signature parity with the other subcommands' `run` fns.
pub async fn run(check_only: bool) -> Result<()> {
    let cfg = UpdateConfig::for_production(env!("CARGO_PKG_VERSION"));
    if check_only {
        // Every path exits the process; the Result is only for signature
        // parity, so discard it explicitly.
        let _ = check(&cfg).await;
    } else {
        let _ = update(&cfg).await;
    }
    Ok(())
}

/// `fspec update --check` — pure query, no download or replace.
///
/// Exit codes: 0 = current, 1 = update available, 2 = network/API failure.
/// Every path terminates via `std::process::exit`; the trailing `Ok(())`
/// only satisfies the compiler (unreachable in practice).
#[allow(unreachable_code)]
async fn check(cfg: &UpdateConfig) -> Result<()> {
    match cfg.check_latest().await {
        Ok(info) => {
            println!("latest version: {}", info.version);
            if info.is_newer {
                println!(
                    "update available: {} -> {} (run `fspec update` to install)",
                    cfg.current_version, info.version
                );
                std::process::exit(1);
            }
            println!("fspec is up to date (v{})", cfg.current_version);
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(2);
        }
    }
    Ok(())
}

/// `fspec update` — download + verify + replace.
///
/// Exit codes: 0 = up-to-date or updated, 1 = update failed.
/// Every path terminates via `std::process::exit`; the trailing `Ok(())`
/// only satisfies the compiler (unreachable in practice).
#[allow(unreachable_code)]
async fn update(cfg: &UpdateConfig) -> Result<()> {
    match cfg.perform_update().await {
        Ok(UpdateOutcome::UpToDate { version }) => {
            println!("fspec is up to date (v{version})");
            std::process::exit(0);
        }
        Ok(UpdateOutcome::Updated {
            version,
            restart_required,
        }) => {
            println!("✓ fspec v{version} installed.");
            if restart_required {
                println!("Restart fspec to activate the new version.");
            }
            std::process::exit(0);
        }
        Ok(UpdateOutcome::Failed { message }) => {
            eprintln!("error: {message}");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
    Ok(())
}
