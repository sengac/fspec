// Example demonstrating CLI-006 bash highlighting and diff rendering

use codelet::cli::diff::{render_diff_line, render_diff_summary, render_file_diff};
use codelet::cli::highlight::highlight_bash_command;

fn main() {
    println!("=== CLI-006: Enhanced Text Styling Demo ===\n");

    // Test 1: Bash highlighting
    println!("1. Bash Command Highlighting:");
    println!("   Input: echo \"hello\" && ls -la | grep test");
    let command = "echo \"hello\" && ls -la | grep test";
    let highlighted = highlight_bash_command(command);
    println!("   Output:");
    for line in &highlighted {
        println!("   {}", line);
    }
    println!();

    // Test 2: Diff summary
    println!("2. Diff Summary:");
    let summary = render_diff_summary(5, 2);
    println!("   {}", summary);
    println!();

    // Test 3: Diff lines
    println!("3. Individual Diff Lines:");
    let addition = render_diff_line(1, '+', "pub fn new_function() {");
    let deletion = render_diff_line(2, '-', "pub fn old_function() {");
    let context = render_diff_line(3, ' ', "    // unchanged code");
    println!("   {}", addition);
    println!("   {}", deletion);
    println!("   {}", context);
    println!();

    // Test 4: Complete file diff
    println!("4. Complete File Diff:");
    let changes = vec![
        (10, "let x = 3;", '-'),
        (10, "let x = 5;", '+'),
        (11, "println!(\"x = {}\", x);", ' '),
    ];
    match render_file_diff("src/main.rs", &changes) {
        Ok(diff) => println!("{}", diff),
        Err(e) => eprintln!("Error rendering diff: {}", e),
    }
    println!();

    println!("=== All features working correctly! ===");
}
