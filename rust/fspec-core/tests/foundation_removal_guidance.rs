//! REMIND-010 — the removal commands for personas and capabilities are
//! documented next to their add commands in the CONFIGURATION & SETUP
//! section of `fspec bootstrap` (with examples that remove placeholder
//! text), and a refused discover-foundation --finalize explains how to fill
//! or remove the placeholders. `fspec help <group>` was never ported
//! (product owner, 2026-11-06, H-HELP-GROUPS a: the setup help is checked in
//! the bootstrap output).
//!
//! Feature: spec/features/foundation-discovery-removal-commands-documentation.feature
//!
//! Code: rust/fspec-core/src/commands/bootstrap_doc.txt (CONFIGURATION &
//! SETUP, FOUNDATION & DOCUMENTATION) and
//! rust/fspec-core/src/commands/discover_foundation.rs (the placeholder gate
//! of finalize).

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn dispatch(dir: &Path, command: &str, args: Value) -> Result<String, Box<dyn std::error::Error>> {
    let result = dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: dir.to_path_buf(),
    });
    if !result.success {
        return Err(format!("{command} failed: {result:?}").into());
    }
    Ok(result.data)
}

/// The CONFIGURATION & SETUP section of the bootstrap output.
fn setup_section(dir: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let output = dispatch(dir, "bootstrap", json!({}))?;
    let start = output
        .find("\nCONFIGURATION & SETUP\n")
        .ok_or("the bootstrap output has no CONFIGURATION & SETUP section")?;
    let rest = &output[start..];
    let end = rest
        .find("\nLIFECYCLE HOOKS\n")
        .ok_or("the CONFIGURATION & SETUP section is not followed by LIFECYCLE HOOKS")?;
    Ok(rest[..end].to_string())
}

/// The block of `section` documenting `command` (from its usage line to the
/// next blank line).
fn command_block<'a>(section: &'a str, usage: &str) -> Result<&'a str, Box<dyn std::error::Error>> {
    let start = section
        .find(&format!("\n  {usage}\n"))
        .ok_or_else(|| format!("no {usage:?} entry in:\n{section}"))?
        + 1;
    let rest = &section[start..];
    let end = rest.find("\n\n").unwrap_or(rest.len());
    Ok(&rest[..end])
}

#[test]
fn bootstrap_setup_section_includes_remove_persona_command() -> TestResult {
    let tmp = tempfile::tempdir()?;

    // @step Given I am using fspec
    // @step When I run "fspec bootstrap" and read its CONFIGURATION & SETUP section
    let section = setup_section(tmp.path())?;

    // @step Then the output should include "fspec remove-persona <name>"
    let block = command_block(&section, "fspec remove-persona <name>")?;

    // @step And the output should include "Description: Remove persona from foundation.json"
    assert!(
        block.contains("\n    Description: Remove persona from foundation.json\n"),
        "{block}"
    );

    // @step And the output should include example usage for removing personas
    assert!(
        block.contains("    Examples:\n      fspec remove-persona \"Developer\""),
        "{block}"
    );

    // @step And the remove-persona section should appear alongside add-persona
    let add = section
        .find("\n  fspec add-persona <name> <description> --goal <goal>\n")
        .ok_or("no add-persona entry")?;
    let remove = section
        .find("\n  fspec remove-persona <name>\n")
        .ok_or("no remove-persona entry")?;
    let foundation = section
        .find("\nFOUNDATION & DOCUMENTATION\n")
        .ok_or("no FOUNDATION & DOCUMENTATION subsection")?;
    assert!(foundation < add && add < remove, "{section}");
    Ok(())
}

#[test]
fn bootstrap_setup_section_includes_remove_capability_command() -> TestResult {
    let tmp = tempfile::tempdir()?;

    // @step Given I am using fspec
    // @step When I run "fspec bootstrap" and read its CONFIGURATION & SETUP section
    let section = setup_section(tmp.path())?;

    // @step Then the output should include "fspec remove-capability <name>"
    let block = command_block(&section, "fspec remove-capability <name>")?;

    // @step And the output should include "Description: Remove capability from foundation.json"
    assert!(
        block.contains("\n    Description: Remove capability from foundation.json\n"),
        "{block}"
    );

    // @step And the output should include example usage for removing capabilities
    assert!(
        block.contains("    Examples:\n      fspec remove-capability \"User Authentication\""),
        "{block}"
    );

    // @step And the remove-capability section should appear alongside add-capability
    let add = section
        .find("\n  fspec add-capability <name> <description>\n")
        .ok_or("no add-capability entry")?;
    let remove = section
        .find("\n  fspec remove-capability <name>\n")
        .ok_or("no remove-capability entry")?;
    let foundation = section
        .find("\nFOUNDATION & DOCUMENTATION\n")
        .ok_or("no FOUNDATION & DOCUMENTATION subsection")?;
    assert!(foundation < add && add < remove, "{section}");
    Ok(())
}

#[test]
fn removal_commands_show_placeholder_removal_examples() -> TestResult {
    let tmp = tempfile::tempdir()?;

    // @step Given I am using fspec
    // @step When I run "fspec bootstrap" and read its CONFIGURATION & SETUP section
    let section = setup_section(tmp.path())?;
    let persona = command_block(&section, "fspec remove-persona <name>")?;
    let capability = command_block(&section, "fspec remove-capability <name>")?;

    // @step Then the remove-persona examples should include removing placeholder text
    // @step And the output should show 'fspec remove-persona "[QUESTION: Who uses this?]"'
    assert!(
        persona.contains("\n      fspec remove-persona \"[QUESTION: Who uses this?]\""),
        "{persona}"
    );

    // @step And the remove-capability examples should include removing placeholder text
    // @step And the output should show 'fspec remove-capability "[QUESTION: What can users DO?]"'
    assert!(
        capability.contains("\n      fspec remove-capability \"[QUESTION: What can users DO?]\""),
        "{capability}"
    );
    Ok(())
}

#[test]
fn discover_foundation_error_includes_removal_guidance_for_placeholders() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let dir = tmp.path();

    // @step Given I have a foundation draft with placeholder personas
    dispatch(dir, "discover-foundation", json!({}))?;
    let draft = std::fs::read_to_string(dir.join("spec/foundation.json.draft"))?;

    // @step And the draft contains '[QUESTION: Who uses this?]' placeholder
    assert!(draft.contains("[QUESTION: Who uses this?]"), "{draft}");

    // @step When I run "fspec discover-foundation --finalize"
    let data: Value = serde_json::from_str(&dispatch(
        dir,
        "discover-foundation",
        json!({"finalize": true}),
    )?)?;

    // @step Then finalization should be refused
    assert_eq!(data["valid"].as_bool(), Some(false), "{data}");
    assert!(!dir.join("spec/foundation.json").exists());
    let errors = data["validationErrors"].as_str().unwrap_or_default();

    // @step And the error message should explain how to fill placeholders
    assert!(
        errors.contains("Please fill all placeholder fields before finalizing:\n"),
        "{errors}"
    );

    // @step And the error message should explain how to remove unwanted placeholders
    assert!(
        errors.contains("To remove unwanted placeholders:\n"),
        "{errors}"
    );

    // @step And the error message should include "fspec remove-persona" command example
    assert!(
        errors.contains("  - For personas: fspec remove-persona \"<name>\"\n"),
        "{errors}"
    );

    // @step And the error message should include "fspec remove-capability" command example
    assert!(
        errors.contains("  - For capabilities: fspec remove-capability \"<name>\"\n"),
        "{errors}"
    );
    Ok(())
}
