//! FOUND-006 — the discover-foundation workflow is documented: the
//! spec/AGENTS.md that `fspec init` writes describes it, and
//! `fspec discover-foundation --help` has WHEN TO USE, COMMON PATTERNS (what
//! to analyse per project type) and EXAMPLES walking the discovery loop.
//!
//! Feature: spec/features/update-documentation-and-help.feature
//!
//! Code: rust/fspec-core/src/commands/init_doc_sysrem_cli.md (the claude
//! spec/AGENTS.md template) and
//! rust/fspec-core/src/help/configs/discover_foundation.rs.

use std::path::Path;

use codelet_fspec_core::help::{configs, format_command_help};
use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::json;
use serial_test::serial;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Points HOME at a temporary directory while alive, so init never
/// touches the real home.
struct HomeGuard(Option<std::ffi::OsString>);

impl HomeGuard {
    fn redirect(path: &Path) -> Self {
        let previous = Self(std::env::var_os("HOME"));
        std::env::set_var("HOME", path);
        previous
    }
}

impl Drop for HomeGuard {
    fn drop(&mut self) {
        match &self.0 {
            Some(previous) => std::env::set_var("HOME", previous),
            None => std::env::remove_var("HOME"),
        }
    }
}

/// The text of the section of `help` that starts with the line `heading`
/// and runs to the next blank line.
fn help_section<'a>(help: &'a str, heading: &str) -> Result<&'a str, Box<dyn std::error::Error>> {
    let start = help
        .find(&format!("\n{heading}\n"))
        .ok_or_else(|| format!("no {heading} section in:\n{help}"))?
        + 1;
    let rest = &help[start..];
    let end = rest.find("\n\n").unwrap_or(rest.len());
    Ok(&rest[..end])
}

#[test]
#[serial]
fn agents_md_documents_discover_foundation_workflow() -> TestResult {
    let project = tempfile::tempdir()?;
    let home = tempfile::tempdir()?;
    let _home = HomeGuard::redirect(home.path());

    // @step Given I run "fspec init" for the claude agent
    let result = dispatch_command(DispatchRequest {
        command: "init".to_string(),
        args_json: json!({"agent": ["claude"]}).to_string(),
        project_root: project.path().to_path_buf(),
    });
    assert!(result.success, "init failed: {result:?}");
    let agents = std::fs::read_to_string(project.path().join("spec/AGENTS.md"))?;

    // @step When I search spec/AGENTS.md for the "Bootstrapping Foundation for New Projects" section
    let start = agents
        .find("### Bootstrapping Foundation for New Projects")
        .ok_or("spec/AGENTS.md has no discover-foundation section")?;
    let rest = &agents[start..];
    let end = rest.find("\n## ").unwrap_or(rest.len());
    let section = &rest[..end];

    // @step Then I should find workflow documentation with command examples
    assert!(section.contains("```bash\n"), "{section}");

    // @step And documentation should show how to run discover-foundation command
    assert!(
        section.contains("fspec discover-foundation           # Create draft with placeholders"),
        "{section}"
    );
    assert!(
        section
            .contains("fspec discover-foundation --finalize # Validate and create foundation.json"),
        "{section}"
    );

    // @step And documentation should describe the draft, field-by-field and finalize workflow and point to fspec discover-foundation --help
    assert!(
        section.contains(
            "**Workflow**: Draft creation → Field-by-field prompting → AI analysis/update → Automatic chaining → Validation/finalization. See `fspec discover-foundation --help` for complete details."
        ),
        "{section}"
    );
    Ok(())
}

#[test]
fn help_system_provides_comprehensive_command_help() -> TestResult {
    // @step Given I run 'fspec discover-foundation --help'
    // @step When the help output is displayed
    let help = format_command_help(&configs::discover_foundation::CONFIG);

    // @step Then it should include "WHEN TO USE" section
    let when = help_section(&help, "WHEN TO USE")?;
    assert!(
        when.contains("Use when starting a new project with fspec to bootstrap foundation.json."),
        "{when}"
    );

    // @step And it should include "EXAMPLES" section with concrete usage
    let examples_at = help
        .find("\nEXAMPLES\n")
        .ok_or_else(|| format!("no EXAMPLES section in:\n{help}"))?;
    let examples = &help[examples_at..];

    // @step And examples should show typical discovery workflow
    let steps = [
        "  1. Create draft and receive guidance for first field\n  $ fspec discover-foundation\n",
        "  2. Fill first field, command chains to next field\n  $ fspec update-foundation projectName \"fspec\"\n",
        "  3. Validate complete draft and create final foundation.json\n  $ fspec discover-foundation --finalize\n",
    ];
    let mut positions = Vec::new();
    for step in steps {
        positions.push(
            examples
                .find(step)
                .ok_or_else(|| format!("missing example {step:?} in:\n{examples}"))?,
        );
    }
    assert!(
        positions.windows(2).all(|pair| pair[0] < pair[1]),
        "examples out of order:\n{examples}"
    );
    Ok(())
}

#[test]
fn discovery_guide_explains_code_analysis_patterns() -> TestResult {
    // @step Given I run 'fspec discover-foundation --help'
    let help = format_command_help(&configs::discover_foundation::CONFIG);

    // @step When I review its COMMON PATTERNS section
    let patterns = help_section(&help, "COMMON PATTERNS")?;
    let lines: Vec<&str> = patterns.lines().skip(1).collect();
    assert_eq!(lines.len(), 3, "{patterns}");

    // @step Then I should find the CLI tools pattern: CLI framework usage and command definitions
    assert_eq!(
        lines[0],
        "  • CLI tools: AI analyzes CLI framework usage, command definitions, identifies command-line user persona"
    );

    // @step And I should find the web apps pattern: routes and UI components
    assert_eq!(
        lines[1],
        "  • Web apps: AI analyzes routes, UI components, identifies End User and API Consumer personas"
    );

    // @step And I should find the libraries pattern: exports and API
    assert_eq!(
        lines[2],
        "  • Libraries: AI analyzes exports and API, identifies Developer persona"
    );

    // @step And each pattern should name the persona to identify, not how to implement it
    for line in &lines {
        assert!(line.contains("persona"), "{line}");
    }
    Ok(())
}
