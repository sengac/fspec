//! BUG-564 — installing fspec on Windows while fspec is running.
//!
//! A running fspec.exe cannot be overwritten or deleted on Windows, only
//! renamed. scripts/build-install.ps1 and scripts/install.ps1 install through
//! one function, `Install-FspecBinary`, that copies the new binary to a
//! temporary file in the install directory, renames fspec.exe aside and
//! renames the temporary file into place (BUG-555's rule for the PowerShell
//! scripts).
//!
//! PowerShell and Windows are not available where these tests run, so they
//! check the scripts' text exactly: the shared function, the order of its
//! steps and the absence of an in-place copy. Running the scripts on Windows
//! is not covered here.
//!
//! Feature: spec/features/installing-fspec-on-windows-while-fspec-is-running.feature

use std::fs;
use std::path::{Path, PathBuf};

const SCRIPTS: [&str; 2] = ["scripts/build-install.ps1", "scripts/install.ps1"];
const FUNCTION_START: &str = "function Install-FspecBinary {";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_script(relative: &str) -> Result<String, String> {
    let path = repo_root().join(relative);
    fs::read_to_string(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))
}

/// The text of `Install-FspecBinary`, from its `function` line to the first
/// line that is exactly `}`.
fn install_function(script: &str) -> Result<String, String> {
    let start = script
        .find(&format!("\n{FUNCTION_START}\n"))
        .map(|at| at + 1)
        .ok_or("no Install-FspecBinary function")?;
    let body = &script[start..];
    let end = body
        .find("\n}\n")
        .ok_or("Install-FspecBinary is not closed")?;
    Ok(body[..end + 2].to_string())
}

/// The line index of the first line of `text` containing `needle`.
fn line_of(text: &str, needle: &str) -> Result<usize, String> {
    text.lines()
        .position(|line| line.contains(needle))
        .ok_or(format!(
            "Install-FspecBinary has no line containing `{needle}`"
        ))
}

// Scenario: Both Windows installers install through the same rename-aside function
#[test]
fn both_windows_installers_install_through_the_same_function() -> Result<(), String> {
    // @step Given scripts/build-install.ps1 and scripts/install.ps1
    let scripts = SCRIPTS
        .iter()
        .map(|s| read_script(s).map(|text| (*s, text)))
        .collect::<Result<Vec<_>, _>>()?;

    // @step When their install steps are read
    let functions = scripts
        .iter()
        .map(|(name, text)| install_function(text).map_err(|e| format!("{name}: {e}")))
        .collect::<Result<Vec<_>, _>>()?;

    // @step Then each defines Install-FspecBinary with the same text
    if functions[0] != functions[1] {
        return Err("Install-FspecBinary differs between the two scripts".to_string());
    }

    // @step And each installs fspec.exe only by calling Install-FspecBinary
    for (name, text) in &scripts {
        let outside = text.replace(&functions[0], "");
        let calls = outside
            .lines()
            .filter(|line| line.trim_start().starts_with("Install-FspecBinary "))
            .count();
        if calls != 1 {
            return Err(format!(
                "{name} must call Install-FspecBinary once, found {calls}"
            ));
        }

        // @step And neither copies onto fspec.exe in place
        if let Some(line) = outside.lines().find(|line| line.contains("Copy-Item")) {
            return Err(format!(
                "{name} copies outside Install-FspecBinary: `{}`",
                line.trim()
            ));
        }
    }
    Ok(())
}

// Scenario: The new binary replaces fspec.exe by renames inside the install directory
#[test]
fn the_new_binary_replaces_fspec_exe_by_renames_in_the_install_directory() -> Result<(), String> {
    // @step Given the Install-FspecBinary function
    let function = install_function(&read_script(SCRIPTS[0])?)?;

    // @step When its steps are read in order
    let cleanup = line_of(&function, r#"-Filter "$name.*.old""#)?;
    let remove_old = line_of(&function, "Remove-Item -LiteralPath $_.FullName")?;
    let temp = line_of(
        &function,
        r#"$temp = Join-Path $installDir ".fspec-install-$id.tmp""#,
    )?;
    let old = line_of(&function, r#"$old = "$Target.$id.old""#)?;
    let copy = line_of(
        &function,
        "Copy-Item -LiteralPath $Source -Destination $temp",
    )?;
    let aside = line_of(
        &function,
        "Move-Item -LiteralPath $Target -Destination $old",
    )?;
    let into_place = line_of(
        &function,
        "Move-Item -LiteralPath $temp -Destination $Target",
    )?;
    let message = line_of(
        &function,
        r#"Write-Host "Running fspec processes keep the old version until restarted.""#,
    )?;

    // @step Then it removes the fspec.exe.*.old files no process holds
    if !(cleanup < remove_old && remove_old < copy) {
        return Err("old binaries must be removed before the new one is copied".to_string());
    }

    // @step And it copies the new binary to a temporary file in the install directory
    if !(temp < copy && old < copy) {
        return Err("the temporary file must be named in the install directory first".to_string());
    }

    // @step And it renames fspec.exe aside to fspec.exe.<id>.old before renaming the temporary file to fspec.exe
    if !(copy < aside && aside < into_place) {
        return Err(
            "fspec.exe must be renamed aside after the copy and before the move".to_string(),
        );
    }

    // @step And it prints "Running fspec processes keep the old version until restarted."
    if message < into_place {
        return Err("the restart message must follow the install".to_string());
    }
    Ok(())
}

// Scenario: A failed install leaves fspec.exe as it was
#[test]
fn a_failed_install_leaves_fspec_exe_as_it_was() -> Result<(), String> {
    // @step Given the Install-FspecBinary function
    let function = install_function(&read_script(SCRIPTS[1])?)?;

    // @step When copying or renaming fails
    let catch = line_of(&function, "catch {")?;
    let into_place = line_of(
        &function,
        "Move-Item -LiteralPath $temp -Destination $Target",
    )?;

    // @step Then it removes the temporary file
    let remove_temp = line_of(&function, "Remove-Item -LiteralPath $temp")?;

    // @step And it moves a renamed-aside fspec.exe back
    let restore_check = line_of(
        &function,
        "if (-not (Test-Path -LiteralPath $Target) -and (Test-Path -LiteralPath $old))",
    )?;
    let restore = line_of(
        &function,
        "Move-Item -LiteralPath $old -Destination $Target",
    )?;

    // @step And it rethrows the error
    let rethrow = function
        .lines()
        .skip(catch)
        .position(|line| line.trim() == "throw")
        .map(|at| at + catch)
        .ok_or("the catch block does not rethrow")?;
    if !(into_place < catch
        && catch < remove_temp
        && remove_temp < restore_check
        && restore_check < restore
        && restore < rethrow)
    {
        return Err(
            "the catch block must remove the temporary file, restore fspec.exe, then rethrow"
                .to_string(),
        );
    }
    Ok(())
}
