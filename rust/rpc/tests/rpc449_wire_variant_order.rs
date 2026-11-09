//! RPC-449: FspecService keeps every tarpc variant index stable.
//!
//! Feature: spec/features/rpc-wire-variant-order.feature
//!
//! tarpc numbers requests by trait declaration order. The trait is composed
//! from the `service_declaration/` fragments in `include!` order, so reading
//! their `async fn` names in that order gives each method's variant index.

use std::path::Path;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const FRAGMENTS: [&str; 4] = [
    "workspace.rs",
    "providers_sessions.rs",
    "supervision_credentials.rs",
    "session_mcp.rs",
];

/// FspecService method names in declaration (variant-index) order.
fn declared_methods() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/service_declaration");
    let mut names = Vec::new();
    for fragment in FRAGMENTS {
        let text = std::fs::read_to_string(dir.join(fragment))?;
        for line in text.lines() {
            let line = line.trim_start();
            if line.starts_with("//") {
                continue;
            }
            if let Some(rest) = line.strip_prefix("async fn ") {
                let name = rest.split(['(', '<']).next().unwrap_or_default();
                names.push(name.to_string());
            }
        }
    }
    Ok(names)
}

fn frozen_methods() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/wire_order_frozen.txt");
    Ok(std::fs::read_to_string(path)?
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect())
}

fn index_of(methods: &[String], name: &str) -> Option<usize> {
    methods.iter().position(|method| method == name)
}

#[test]
fn every_frozen_method_keeps_its_variant_index() -> TestResult {
    // @step Given the frozen wire-order fixture
    let frozen = frozen_methods()?;

    // @step When the FspecService methods are read in declaration order
    let declared = declared_methods()?;

    // @step Then the first methods match the fixture exactly, index for index
    assert!(declared.len() >= frozen.len(), "methods were removed");
    let shifted: Vec<String> = frozen
        .iter()
        .zip(&declared)
        .enumerate()
        .filter(|(_, (want, got))| want != got)
        .map(|(index, (want, got))| format!("index {index}: expected {want}, found {got}"))
        .collect();
    assert!(
        shifted.is_empty(),
        "variant indices shifted:\n{}",
        shifted.join("\n")
    );
    Ok(())
}

#[test]
fn the_workspace_methods_are_appended_after_the_original_methods() -> TestResult {
    // @step Given the FspecService methods in declaration order
    let declared = declared_methods()?;

    // @step When the workspace methods' indices are read
    let last_original =
        index_of(&declared, "get_work_unit_details").ok_or("get_work_unit_details")?;

    // @step Then list_workspaces, list_workspace_work_units and create_session_in_workspace come after get_work_unit_details
    for name in [
        "list_workspaces",
        "list_workspace_work_units",
        "create_session_in_workspace",
    ] {
        let index = index_of(&declared, name).ok_or(name)?;
        assert!(
            index > last_original,
            "{name} is index {index}, before get_work_unit_details ({last_original})"
        );
    }

    // @step And set_default_model is index 30 and get_work_unit_details is index 123
    assert_eq!(index_of(&declared, "set_default_model"), Some(30));
    assert_eq!(last_original, 123);
    Ok(())
}

/// BUG-596: N in the header's "index = line number - N".
fn header_offset(fixture: &str) -> Result<usize, Box<dyn std::error::Error>> {
    let header = fixture.lines().next().ok_or("the fixture is empty")?;
    let (_, rest) = header
        .split_once("index = line number - ")
        .ok_or_else(|| format!("the header states no index formula: {header}"))?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    Ok(digits.parse()?)
}

#[test]
fn the_fixture_header_states_the_formula_that_gives_each_methods_index() -> TestResult {
    // @step Given the frozen wire-order fixture whose first line states "index = line number - N"
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/wire_order_frozen.txt");
    let fixture = std::fs::read_to_string(path)?;
    let declared = declared_methods()?;

    // @step When N is read from the header and each method's line number is read
    let offset = header_offset(&fixture)?;
    let lines: Vec<(usize, &str)> = fixture
        .lines()
        .enumerate()
        .map(|(index, line)| (index + 1, line))
        .filter(|(_, line)| !line.is_empty() && !line.starts_with('#'))
        .collect();

    // @step Then every method's line number minus N is its declared variant index
    let wrong: Vec<String> = lines
        .iter()
        .filter_map(|(line_number, method)| {
            let stated = line_number.checked_sub(offset);
            let declared_index = index_of(&declared, method);
            (stated != declared_index).then(|| {
                format!(
                    "line {line_number} ({method}): the header gives index {stated:?}, declared index {declared_index:?}"
                )
            })
        })
        .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));

    // @step And list_work_units on line 4 is index 0 and get_thinking_options on line 155 is index 151
    let line_of = |name: &str| {
        lines
            .iter()
            .find(|(_, method)| *method == name)
            .map(|(n, _)| *n)
    };
    assert_eq!(line_of("list_work_units"), Some(4));
    assert_eq!(index_of(&declared, "list_work_units"), Some(0));
    assert_eq!(line_of("get_thinking_options"), Some(155));
    assert_eq!(index_of(&declared, "get_thinking_options"), Some(151));
    Ok(())
}
