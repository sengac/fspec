//! CLI-019 — fspec context loads acceptance criteria by cucumber tag
//! expression in pages; show-acceptance-criteria, get-scenarios and
//! search-scenarios are gone.
//!
//! Feature: spec/features/load-acceptance-criteria-with-fspec-context.feature
//!
//! Every scenario runs the real fspec binary on fixture features in a
//! temporary project (no clock, no network, no sleeps).

use std::path::Path;
use std::process::{Command, Output};

use codelet_fspec_core::{dispatch_command, DispatchRequest};

mod common;

use common::{fspec_bin, project_root};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const LOGIN: &str = "@auth
@CLI-001
Feature: Login
  \"\"\"
  Login notes.
  \"\"\"

  Background: User Story
    As a user
    I want to log in
    So that I see my board

  @smoke
  Scenario: Valid login
    Given a user \"ada\"
    When she logs in with the right password
    Then she sees the board

  @wip
  Scenario: Wrong password
    Given a user \"ada\"
    When she logs in with a wrong password
    Then she sees \"Wrong password\"
";

const BILLING: &str = "@billing
Feature: Billing

  Scenario: Invoice totals
    Given invoice lines
      | item | price |
      | pen  | 2     |
    Then the total is 2

  @money
  Rule: Refunds

    Scenario: Refund a paid invoice
      Given a paid invoice
      When a refund is requested with
        \"\"\"
        refund-id: R-7
        \"\"\"
      Then the invoice is refunded
";

/// The login and billing fixture features in a fresh project.
fn fixture() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let features = root.path().join("spec/features");
    std::fs::create_dir_all(&features)?;
    // Written in reverse order: the listing order must not matter.
    std::fs::write(features.join("b-billing.feature"), BILLING)?;
    std::fs::write(features.join("a-login.feature"), LOGIN)?;
    Ok(root)
}

fn fspec(cwd: &Path, args: &[&str]) -> std::io::Result<Output> {
    Command::new(fspec_bin())
        .args(args)
        .current_dir(cwd)
        .output()
}

/// Run `fspec context <args>`; Ok(stdout) on exit 0, Err(stderr) otherwise.
fn context(cwd: &Path, args: &[&str]) -> Result<Result<String, String>, std::io::Error> {
    let mut full = vec!["context"];
    full.extend_from_slice(args);
    let output = fspec(cwd, &full)?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    Ok(match output.status.code() {
        Some(0) => Ok(stdout),
        _ => Err(stderr),
    })
}

fn page(cwd: &Path, args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    context(cwd, args)?.map_err(|stderr| format!("fspec context {args:?} failed: {stderr}").into())
}

fn failure(cwd: &Path, args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    match context(cwd, args)? {
        Ok(stdout) => Err(format!("fspec context {args:?} succeeded: {stdout}").into()),
        Err(stderr) => Ok(stderr),
    }
}

/// The scenario names a page lists, in order.
fn names(page: &str) -> Vec<String> {
    page.lines()
        .filter_map(|line| line.trim_start().strip_prefix("Scenario: "))
        .map(str::to_string)
        .collect()
}

/// The page body: everything before the markdown footer.
fn body(page: &str) -> Result<&str, String> {
    page.rfind("---\n\npage ")
        .map(|end| &page[..end])
        .ok_or_else(|| format!("no footer in {page}"))
}

fn footer_lines(page: &str) -> Vec<&str> {
    page.lines()
        .rev()
        .take(2)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

#[test]
fn a_tag_expression_with_and_and_not_selects_scenarios() -> TestResult {
    // @step Given the login and billing fixture features
    let root = fixture()?;
    // @step When I run "fspec context --tags '@auth and not @wip'"
    let out = page(root.path(), &["--tags", "@auth and not @wip"])?;
    // @step Then the page lists only the scenario "Valid login"
    assert_eq!(names(&out), ["Valid login"], "{out}");
    Ok(())
}

#[test]
fn a_tag_expression_with_or_and_parentheses_lists_scenarios_by_feature_path_then_line() -> TestResult
{
    // @step Given the login and billing fixture features
    let root = fixture()?;
    // @step When I run "fspec context --tags '(@smoke or @billing) and not @wip'"
    let out = page(
        root.path(),
        &["--tags", "(@smoke or @billing) and not @wip"],
    )?;
    // @step Then the page lists "Valid login", "Invoice totals" and "Refund a paid invoice" in that order
    assert_eq!(
        names(&out),
        ["Valid login", "Invoice totals", "Refund a paid invoice"],
        "{out}"
    );
    Ok(())
}

#[test]
fn scenarios_inherit_the_tags_of_their_feature_and_rule() -> TestResult {
    // @step Given the login and billing fixture features
    let root = fixture()?;
    // @step When I run "fspec context --tags @money"
    let out = page(root.path(), &["--tags", "@money"])?;
    // @step Then the page lists only the scenario "Refund a paid invoice"
    assert_eq!(names(&out), ["Refund a paid invoice"], "{out}");
    // @step When I run "fspec context --tags @auth"
    let out = page(root.path(), &["--tags", "@auth"])?;
    // @step Then the page lists "Valid login" and "Wrong password" in that order
    assert_eq!(names(&out), ["Valid login", "Wrong password"], "{out}");
    Ok(())
}

#[test]
fn a_criterion_carries_its_location_tags_steps_doc_strings_and_tables_under_its_features_header(
) -> TestResult {
    // @step Given the login and billing fixture features
    let root = fixture()?;
    // @step When I run "fspec context --tags '@smoke or @billing'"
    let out = page(root.path(), &["--tags", "@smoke or @billing"])?;
    // @step Then the Login feature's tags, name, description and Background are printed once, above "Valid login"
    let login_header = "## spec/features/a-login.feature\n\n```gherkin\n@auth @CLI-001\nFeature: Login\n  \"\"\"\n  Login notes.\n  \"\"\"\n\n  Background: User Story\n    As a user\n    I want to log in\n    So that I see my board\n```\n\n";
    assert_eq!(out.matches(login_header).count(), 1, "{out}");
    assert_eq!(out.matches("Feature: Login").count(), 1, "{out}");
    let valid = "### Valid login\n\nLocation: spec/features/a-login.feature:14\nInherited tags: @auth @CLI-001\n\n```gherkin\n@smoke\nScenario: Valid login\n  Given a user \"ada\"\n  When she logs in with the right password\n  Then she sees the board\n```\n\n";
    assert!(out.starts_with(&format!("{login_header}{valid}")), "{out}");
    // @step And "Valid login" shows "spec/features/a-login.feature:14", its tag @smoke and the inherited tags @auth @CLI-001
    assert!(out.contains(valid), "{out}");
    // @step And "Invoice totals" shows its data table
    let invoice = "### Invoice totals\n\nLocation: spec/features/b-billing.feature:4\nInherited tags: @billing\n\n```gherkin\nScenario: Invoice totals\n  Given invoice lines\n    | item | price |\n    | pen  | 2     |\n  Then the total is 2\n```\n\n";
    assert!(out.contains(invoice), "{out}");
    // @step And "Refund a paid invoice" shows its Rule, its doc string and the inherited tags @billing @money
    let refund = "### Refund a paid invoice\n\nLocation: spec/features/b-billing.feature:13\nInherited tags: @billing @money\n\n```gherkin\n@money\nRule: Refunds\n\n  Scenario: Refund a paid invoice\n    Given a paid invoice\n    When a refund is requested with\n      \"\"\"\n      refund-id: R-7\n      \"\"\"\n    Then the invoice is refunded\n```\n\n";
    assert!(out.contains(refund), "{out}");
    assert!(
        out.contains(
            "## spec/features/b-billing.feature\n\n```gherkin\n@billing\nFeature: Billing\n```\n\n"
        ),
        "{out}"
    );
    Ok(())
}

#[test]
fn the_text_format_prints_the_same_criteria_without_markdown() -> TestResult {
    // @step Given the login and billing fixture features
    let root = fixture()?;
    // @step When I run "fspec context --tags @smoke --format text"
    let out = page(root.path(), &["--tags", "@smoke", "--format", "text"])?;
    // @step Then the page shows the Login feature's header and "Valid login" with its location and inherited tags
    let expected = "Feature file: spec/features/a-login.feature\n@auth @CLI-001\nFeature: Login\n  \"\"\"\n  Login notes.\n  \"\"\"\n\n  Background: User Story\n    As a user\n    I want to log in\n    So that I see my board\n\nLocation: spec/features/a-login.feature:14\nInherited tags: @auth @CLI-001\n@smoke\nScenario: Valid login\n  Given a user \"ada\"\n  When she logs in with the right password\n  Then she sees the board\n\npage 1 of 1\n";
    assert_eq!(out, expected);
    // @step And the page has no markdown heading or code fence
    assert!(!out.contains("```") && !out.contains("## "), "{out}");
    Ok(())
}

#[test]
fn the_llm_dispatcher_runs_context_with_the_same_arguments() -> TestResult {
    // @step Given the login and billing fixture features
    let root = fixture()?;
    // @step When the dispatcher runs the command context with the arguments {"tags": "@auth and not @wip"}
    let result = dispatch_command(DispatchRequest {
        command: "context".to_string(),
        args_json: serde_json::json!({"tags": "@auth and not @wip"}).to_string(),
        project_root: root.path().to_path_buf(),
    });
    // @step Then the result is the same page "fspec context --tags '@auth and not @wip'" prints
    assert!(result.success, "{:?}", result.error);
    let cli = page(root.path(), &["--tags", "@auth and not @wip"])?;
    assert_eq!(result.data, cli);
    Ok(())
}

#[test]
fn an_invalid_tag_expression_is_an_exact_error() -> TestResult {
    // @step Given the login and billing fixture features
    let root = fixture()?;
    // @step When I run "fspec context --tags '@a not @b'"
    let err = failure(root.path(), &["--tags", "@a not @b"])?;
    // @step Then the command fails with an error starting "invalid tag expression '@a not @b':"
    assert!(
        err.starts_with("Error: invalid tag expression '@a not @b':"),
        "{err}"
    );
    Ok(())
}

#[test]
fn the_text_filter_matches_scenario_names_and_steps_doc_strings_and_table_cells_ignoring_case(
) -> TestResult {
    // @step Given the login and billing fixture features
    let root = fixture()?;
    // @step When I run "fspec context --text 'WRONG PASSWORD'"
    let out = page(root.path(), &["--text", "WRONG PASSWORD"])?;
    // @step Then the page lists only the scenario "Wrong password"
    assert_eq!(names(&out), ["Wrong password"], "{out}");
    // @step When I run "fspec context --text r-7"
    let out = page(root.path(), &["--text", "r-7"])?;
    // @step Then the page lists only the scenario "Refund a paid invoice"
    assert_eq!(names(&out), ["Refund a paid invoice"], "{out}");
    // @step When I run "fspec context --text PEN"
    let out = page(root.path(), &["--text", "PEN"])?;
    // @step Then the page lists only the scenario "Invoice totals"
    assert_eq!(names(&out), ["Invoice totals"], "{out}");
    Ok(())
}

#[test]
fn the_regex_filter_matches_scenario_names_and_steps_and_an_invalid_pattern_is_an_exact_error(
) -> TestResult {
    // @step Given the login and billing fixture features
    let root = fixture()?;
    // @step When I run "fspec context --regex 'Refund (a|an) paid'"
    let out = page(root.path(), &["--regex", "Refund (a|an) paid"])?;
    // @step Then the page lists only the scenario "Refund a paid invoice"
    assert_eq!(names(&out), ["Refund a paid invoice"], "{out}");
    // @step When I run "fspec context --regex '('"
    let err = failure(root.path(), &["--regex", "("])?;
    // @step Then the command fails with an error starting "invalid regex '(':"
    assert!(err.starts_with("Error: invalid regex '(':"), "{err}");
    Ok(())
}

#[test]
fn the_feature_filter_selects_one_feature_and_an_unknown_feature_is_an_exact_error() -> TestResult {
    // @step Given the login and billing fixture features
    let root = fixture()?;
    // @step When I run "fspec context --feature b-billing"
    let out = page(root.path(), &["--feature", "b-billing"])?;
    // @step Then the page lists "Invoice totals" and "Refund a paid invoice" in that order
    assert_eq!(
        names(&out),
        ["Invoice totals", "Refund a paid invoice"],
        "{out}"
    );
    // @step When I run "fspec context --feature missing"
    let err = failure(root.path(), &["--feature", "missing"])?;
    // @step Then the command fails with the error "feature 'missing' does not exist in spec/features/"
    assert_eq!(
        err,
        "Error: feature 'missing' does not exist in spec/features/\n"
    );
    Ok(())
}

#[test]
fn a_page_holds_exactly_as_many_characters_as_the_page_size_allows() -> TestResult {
    // @step Given the login and billing fixture features
    let root = fixture()?;
    // @step And the page body of "fspec context --feature b-billing" is N characters
    let whole = page(root.path(), &["--feature", "b-billing"])?;
    let n = body(&whole)?.chars().count();
    // @step When I run "fspec context --feature b-billing --page-size N"
    let size = n.to_string();
    let out = page(
        root.path(),
        &["--feature", "b-billing", "--page-size", &size],
    )?;
    // @step Then both billing scenarios are on page 1 of 1
    assert_eq!(
        names(&out),
        ["Invoice totals", "Refund a paid invoice"],
        "{out}"
    );
    assert_eq!(footer_lines(&out), ["", "page 1 of 1"], "{out}");
    // @step When I run "fspec context --feature b-billing --page-size N-1"
    let size = (n - 1).to_string();
    let first = page(
        root.path(),
        &["--feature", "b-billing", "--page-size", &size],
    )?;
    let second = page(
        root.path(),
        &[
            "--feature",
            "b-billing",
            "--page-size",
            &size,
            "--page",
            "2",
        ],
    )?;
    // @step Then "Invoice totals" is on page 1 of 2 and "Refund a paid invoice" is on page 2 of 2
    assert_eq!(names(&first), ["Invoice totals"], "{first}");
    assert!(first.contains("\npage 1 of 2\n"), "{first}");
    assert_eq!(names(&second), ["Refund a paid invoice"], "{second}");
    assert!(second.ends_with("\npage 2 of 2\n"), "{second}");
    assert!(
        body(&first)?.chars().count() < n,
        "page 1 must fit the budget: {first}"
    );
    Ok(())
}

#[test]
fn a_scenario_larger_than_the_page_size_is_alone_on_its_page() -> TestResult {
    // @step Given an import feature with the scenarios "Short one", "Long import" and "Short two" where "Long import" is longer than 300 characters
    let root = tempfile::tempdir()?;
    let features = root.path().join("spec/features");
    std::fs::create_dir_all(&features)?;
    let long_line = "x".repeat(320);
    let import = format!(
        "@import\nFeature: Import\n\n  Scenario: Short one\n    Given a small file\n\n  Scenario: Long import\n    Given a file with\n      \"\"\"\n      {long_line}\n      \"\"\"\n\n  Scenario: Short two\n    Given another small file\n"
    );
    std::fs::write(features.join("c-import.feature"), import)?;
    // @step When I run "fspec context --feature c-import --page-size 300"
    let mut pages = Vec::new();
    for number in ["1", "2", "3"] {
        let args = [
            "--feature",
            "c-import",
            "--page-size",
            "300",
            "--page",
            number,
        ];
        pages.push(page(root.path(), &args)?);
    }
    // @step Then "Short one" is on page 1 of 3, "Long import" on page 2 of 3 and "Short two" on page 3 of 3
    for (index, name) in ["Short one", "Long import", "Short two"].iter().enumerate() {
        let out = &pages[index];
        assert_eq!(names(out), [*name], "{out}");
        assert!(
            out.contains(&format!("\npage {} of 3\n", index + 1)),
            "{out}"
        );
    }
    Ok(())
}

#[test]
fn every_page_ends_with_its_number_and_the_exact_command_for_the_next_page() -> TestResult {
    // @step Given the login and billing fixture features
    let root = fixture()?;
    // @step When I run "fspec context --tags '@auth or @smoke' --page-size 300"
    let first = page(
        root.path(),
        &["--tags", "@auth or @smoke", "--page-size", "300"],
    )?;
    // @step Then the page lists only the scenario "Valid login"
    assert_eq!(names(&first), ["Valid login"], "{first}");
    // @step And it ends with "page 1 of 2" and "Next page: fspec context --tags '@auth or @smoke' --page-size 300 --page 2"
    assert_eq!(
        footer_lines(&first),
        [
            "page 1 of 2",
            "Next page: fspec context --tags '@auth or @smoke' --page-size 300 --page 2"
        ],
        "{first}"
    );
    // @step When I run "fspec context --tags '@auth or @smoke' --page-size 300 --page 2"
    let second = page(
        root.path(),
        &[
            "--tags",
            "@auth or @smoke",
            "--page-size",
            "300",
            "--page",
            "2",
        ],
    )?;
    // @step Then the page lists only the scenario "Wrong password"
    assert_eq!(names(&second), ["Wrong password"], "{second}");
    // @step And it ends with "page 2 of 2" and no next page command
    assert!(second.ends_with("---\n\npage 2 of 2\n"), "{second}");
    assert!(!second.contains("Next page:"), "{second}");
    Ok(())
}

#[test]
fn a_page_past_the_last_page_is_an_exact_error() -> TestResult {
    // @step Given the login and billing fixture features
    let root = fixture()?;
    // @step When I run "fspec context --tags @auth --page-size 300 --page 3"
    let err = failure(
        root.path(),
        &["--tags", "@auth", "--page-size", "300", "--page", "3"],
    )?;
    // @step Then the command fails with the error "page 3 does not exist: there are 2 pages"
    assert_eq!(err, "Error: page 3 does not exist: there are 2 pages\n");
    Ok(())
}

#[test]
fn the_json_format_prints_one_page_with_its_number_the_page_count_the_next_command_and_the_items(
) -> TestResult {
    // @step Given the login and billing fixture features
    let root = fixture()?;
    // @step When I run "fspec context --tags @auth --format json --page-size 300"
    let out = page(
        root.path(),
        &["--tags", "@auth", "--format", "json", "--page-size", "300"],
    )?;
    let json: serde_json::Value = serde_json::from_str(&out)?;
    // @step Then the JSON has page 1, pages 2 and nextCommand "fspec context --tags @auth --format json --page-size 300 --page 2"
    assert_eq!(json["page"], 1, "{out}");
    assert_eq!(json["pages"], 2, "{out}");
    assert_eq!(
        json["nextCommand"],
        "fspec context --tags @auth --format json --page-size 300 --page 2"
    );
    // @step And its features hold the Login header and its items hold "Valid login" with its file, line, tags, inherited tags and Gherkin text
    assert_eq!(
        json["features"],
        serde_json::json!([{
            "file": "spec/features/a-login.feature",
            "name": "Login",
            "tags": ["@auth", "@CLI-001"],
            "gherkin": "@auth @CLI-001\nFeature: Login\n  \"\"\"\n  Login notes.\n  \"\"\"\n\n  Background: User Story\n    As a user\n    I want to log in\n    So that I see my board"
        }])
    );
    assert_eq!(
        json["items"],
        serde_json::json!([{
            "file": "spec/features/a-login.feature",
            "line": 14,
            "feature": "Login",
            "rule": null,
            "name": "Valid login",
            "tags": ["@smoke"],
            "inheritedTags": ["@auth", "@CLI-001"],
            "gherkin": "@smoke\nScenario: Valid login\n  Given a user \"ada\"\n  When she logs in with the right password\n  Then she sees the board"
        }])
    );
    // @step When I run "fspec context --tags @auth --format json --page-size 300 --page 2"
    let out = page(
        root.path(),
        &[
            "--tags",
            "@auth",
            "--format",
            "json",
            "--page-size",
            "300",
            "--page",
            "2",
        ],
    )?;
    let json: serde_json::Value = serde_json::from_str(&out)?;
    // @step Then the JSON has page 2, pages 2 and a null nextCommand
    assert_eq!(json["page"], 2, "{out}");
    assert_eq!(json["pages"], 2, "{out}");
    assert!(json["nextCommand"].is_null(), "{out}");
    Ok(())
}

#[test]
fn no_matching_scenario_is_one_empty_page() -> TestResult {
    // @step Given the login and billing fixture features
    let root = fixture()?;
    // @step When I run "fspec context --tags @nothing"
    let out = page(root.path(), &["--tags", "@nothing"])?;
    // @step Then the page says "No scenarios match." and ends with "page 1 of 1" and no next page command
    assert_eq!(out, "No scenarios match.\n\n---\n\npage 1 of 1\n");
    Ok(())
}

#[test]
fn the_three_replaced_commands_are_unknown_commands() -> TestResult {
    // @step Given the fspec command line
    let root = fixture()?;
    let removed: [&[&str]; 3] = [
        &["show-acceptance-criteria"],
        &["get-scenarios"],
        &["search-scenarios", "--query", "login"],
    ];
    for args in removed {
        // @step When I run the removed commands show-acceptance-criteria, get-scenarios and search-scenarios --query login
        let output = fspec(root.path(), args)?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        // @step Then each fails as an unknown command
        assert_eq!(output.status.code(), Some(1), "{args:?}: {stderr}");
        assert!(
            stderr.contains(&format!("error: unknown command '{}'", args[0])),
            "{args:?}: {stderr}"
        );
        // @step And the dispatcher rejects each as an unknown command
        let result = dispatch_command(DispatchRequest {
            command: args[0].to_string(),
            args_json: "{}".to_string(),
            project_root: root.path().to_path_buf(),
        });
        let error = result.error.unwrap_or_default();
        assert!(
            !result.success && error.starts_with(&format!("Unknown fspec command: {}", args[0])),
            "{args:?}: {error}"
        );
    }
    Ok(())
}

#[test]
fn help_and_the_workflow_docs_teach_fspec_context() -> TestResult {
    // @step Given the fspec command line
    let root = fixture()?;
    // @step When I run "fspec context --help"
    let output = fspec(root.path(), &["context", "--help"])?;
    let help = String::from_utf8_lossy(&output.stdout);
    // @step Then it lists --tags, --text, --regex, --feature, --format, --page and --page-size with examples
    assert_eq!(output.status.code(), Some(0), "{help}");
    for flag in [
        "--tags <expression>",
        "--text <text>",
        "--regex <pattern>",
        "--feature <name>",
        "--format <format>",
        "--page <number>",
        "--page-size <chars>",
    ] {
        assert!(help.contains(flag), "help lacks {flag}: {help}");
    }
    assert!(help.contains("EXAMPLES"), "{help}");
    assert!(
        help.contains("$ fspec context --tags '@auth and not @wip'"),
        "{help}"
    );
    // @step And every init doc and the bootstrap doc have a "Loading acceptance criteria" section that runs fspec context
    let commands = project_root().join("rust/fspec-core/src/commands");
    for doc in [
        "init_doc_plain_cli.md",
        "init_doc_plain_ext.md",
        "init_doc_plain_ide.md",
        "init_doc_sysrem_cli.md",
        "bootstrap_doc.txt",
    ] {
        let text = std::fs::read_to_string(commands.join(doc))?;
        let section = text
            .split("### Loading acceptance criteria\n")
            .nth(1)
            .ok_or_else(|| format!("{doc} has no Loading acceptance criteria section"))?;
        let section = section.split("\n### ").next().unwrap_or_default();
        assert!(
            section.contains("fspec context --tags @<workUnitId>"),
            "{doc}: {section}"
        );
    }
    // @step And the research reminder of create-bug suggests "fspec context --text"
    std::fs::write(
        root.path().join("spec/foundation.json"),
        r#"{"version":"2.0.0"}"#,
    )?;
    std::fs::write(
        root.path().join("spec/prefixes.json"),
        r#"{"prefixes":{"BUG":{"prefix":"BUG","description":"Bugs","createdAt":"2026-06-01T00:00:00.000Z"}}}"#,
    )?;
    let output = fspec(root.path(), &["create-bug", "BUG", "Login crash on submit"])?;
    // The research reminder is printed on stderr, after the summary on stdout.
    let created = String::from_utf8_lossy(&output.stderr);
    assert!(
        created.contains("fspec context --text \"login crash\""),
        "{created}"
    );
    Ok(())
}
