@done
@BUG-478
@tools
@rust
@code-quality
Feature: Astgrep finishes on the workspace root
  """
  'fspec astgrep --pattern ... --lang rust --path rust' ran for about 92 s
  (4,807 Rust files, 450k lines) and was killed by callers' timeouts. The
  suspected cause, scanning the 113 GB rust/target, was ruled out: the ignore
  walker honours rust/.gitignore and never lists rust/target. The time went
  into matching: AstGrepTool's search_file passed the pattern as &str to
  find_all, and ast-grep-core's Matcher for str runs Pattern::new for every
  AST node it visits, with no node-kind pre-filter. execute now keeps the
  Pattern it builds to validate the pattern and matches every file with it.
  The fspec astgrep CLI (CLI-015) and the AstGrep rig tool share
  AstGrepTool::execute, so both get the fix.
  Tests: rust/tools/tests/bug478_astgrep_on_the_workspace_root.rs.
  """

  Background: User Story
    As a developer searching code with fspec astgrep
    I want to have a search of the whole workspace finish in seconds
    So that I can point astgrep at rust/ instead of at one source folder at a time

  Scenario: A gitignored target directory is not scanned
    Given a git repository whose rust/.gitignore lists target/
    And rust/src/lib.rs and rust/target/debug/build/gen.rs both define "pub fn marker() {}"
    When astgrep searches rust/ for "pub fn marker() {}"
    Then it reports rust/src/lib.rs
    And it reports nothing under rust/target

  Scenario: Searching a source tree costs about as much as parsing it once
    Given a directory of 20 Rust files with 200 functions each
    When the time to parse and walk every file once is measured
    And astgrep searches the directory for one function
    Then it finds that function
    And the search takes less than four times as long as parsing and walking the files
