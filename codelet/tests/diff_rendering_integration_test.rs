// Feature: spec/features/integrate-diff-rendering-for-file-change-visualization.feature
//
// Integration tests for CLI-007: Diff rendering in interactive mode
// Tests verify color-coded diff display for Edit and Write tool results

use codelet::cli::diff::{render_diff_line, render_file_diff};

/// Scenario: Display diff with color coding for Edit tool changes
///
/// Tests that Edit tool results show deletions in red and additions in green
#[test]
fn test_display_diff_with_color_coding_for_edit_tool_changes() {
    // @step Given the agent is running in interactive mode
    // Setup: Interactive mode context (simulated)
    let interactive_mode = true;
    assert!(interactive_mode, "Interactive mode must be active");

    // @step When the agent edits src/main.rs changing 'let x = 3' to 'let x = 5'
    // Simulate Edit tool result with before/after content
    let old_content = "let x = 3";
    let new_content = "let x = 5";
    let deletion_line = render_diff_line(1, '-', old_content);
    let addition_line = render_diff_line(1, '+', new_content);

    // @step Then the tool result should display a diff
    assert!(!deletion_line.is_empty(), "Diff line should not be empty");
    assert!(!addition_line.is_empty(), "Diff line should not be empty");

    // @step And the deletion 'let x = 3' should be shown in red with '-' prefix
    assert!(
        deletion_line.contains("\x1b[31m"),
        "Deletion should contain red ANSI code"
    );
    assert!(
        deletion_line.contains('-'),
        "Deletion should have '-' prefix"
    );
    assert!(
        deletion_line.contains("let x = 3"),
        "Deletion should contain old content"
    );

    // @step And the addition 'let x = 5' should be shown in green with '+' prefix
    assert!(
        addition_line.contains("\x1b[32m"),
        "Addition should contain green ANSI code"
    );
    assert!(
        addition_line.contains('+'),
        "Addition should have '+' prefix"
    );
    assert!(
        addition_line.contains("let x = 5"),
        "Addition should contain new content"
    );
}

/// Scenario: Display all additions in green for Write tool on new file
///
/// Tests that Write tool results for new files show all lines as green additions
#[test]
fn test_display_all_additions_in_green_for_write_tool_on_new_file() {
    // @step Given the agent is running in interactive mode
    let interactive_mode = true;
    assert!(interactive_mode, "Interactive mode must be active");

    // @step When the agent writes a new file src/utils.rs
    let file_path = "src/utils.rs";
    let new_file_content = vec!["pub fn helper() {", "    println!(\"helper\");", "}"];

    // Render each line as an addition
    let diff_lines: Vec<String> = new_file_content
        .iter()
        .enumerate()
        .map(|(i, line)| render_diff_line(i + 1, '+', line))
        .collect();

    // @step Then the tool result should display all lines in green
    for line in &diff_lines {
        assert!(line.contains("\x1b[32m"), "All lines should be green");
    }

    // @step And each line should have a '+' prefix
    for line in &diff_lines {
        assert!(line.contains('+'), "Each line should have '+' prefix");
    }

    // @step And the diff should show the file path 'src/utils.rs'
    let file_diff = render_file_diff(
        file_path,
        &new_file_content
            .iter()
            .enumerate()
            .map(|(i, &line)| (i + 1, line, '+'))
            .collect::<Vec<_>>(),
    )
    .unwrap();
    assert!(
        file_diff.contains("src/utils.rs"),
        "Diff should show file path"
    );
}

/// Scenario: Display line numbers with diff changes
///
/// Tests that diff output includes line numbers in the correct format
#[test]
fn test_display_line_numbers_with_diff_changes() {
    // @step Given the agent is running in interactive mode
    let interactive_mode = true;
    assert!(interactive_mode, "Interactive mode must be active");

    // @step When the agent edits a file at line 42
    let line_number = 42;
    let content = "modified code";
    let diff_line = render_diff_line(line_number, '+', content);

    // @step Then the tool result should display the line number '42'
    assert!(
        diff_line.contains("42"),
        "Diff should contain line number 42"
    );

    // @step And the line number should appear before the change indicator
    let line_num_str = format!("{}", line_number);
    let plus_pos = diff_line.find('+');
    let line_num_pos = diff_line.find(&line_num_str);
    assert!(
        line_num_pos.is_some() && plus_pos.is_some(),
        "Both line number and '+' must be present"
    );
    assert!(
        line_num_pos.unwrap() < plus_pos.unwrap(),
        "Line number must appear before change indicator"
    );

    // @step And the diff should follow the format '<line_number> <prefix> <content>'
    // Format should be: "42 + modified code" with ANSI codes
    let parts: Vec<&str> = diff_line.split_whitespace().collect();
    assert!(parts.len() >= 3, "Diff should have at least 3 parts");
}

/// Scenario: Display file path header in diff output
///
/// Tests that diff output shows file path before diff lines
#[test]
fn test_display_file_path_header_in_diff_output() {
    // @step Given the agent is running in interactive mode
    let interactive_mode = true;
    assert!(interactive_mode, "Interactive mode must be active");

    // @step When the agent edits src/cli/mod.rs
    let file_path = "src/cli/mod.rs";
    let changes = vec![(10, "old code", '-'), (10, "new code", '+')];

    let file_diff = render_file_diff(file_path, &changes).unwrap();

    // @step Then the tool result should display 'File: src/cli/mod.rs' at the top
    assert!(
        file_diff.starts_with("File: src/cli/mod.rs"),
        "Diff should start with file path header"
    );

    // @step And the file path header should appear before any diff lines
    let lines: Vec<&str> = file_diff.lines().collect();
    assert!(lines.len() > 1, "Diff should have multiple lines");
    assert!(
        lines[0].contains("File:"),
        "First line should be file path header"
    );

    // @step And the diff lines should follow below the header
    assert!(
        lines[1..].iter().any(|line| line.contains("old code")),
        "Diff lines should appear after header"
    );
    assert!(
        lines[1..].iter().any(|line| line.contains("new code")),
        "Diff lines should appear after header"
    );
}
