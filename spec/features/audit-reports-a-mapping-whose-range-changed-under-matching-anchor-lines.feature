@BUG-668
@done
@validator
@coverage
@bug-fix
@rust
@BUG-666
Feature: The audit reports a mapping whose range changed under matching anchor lines
  """
  COV-079 detects a drifted implementation mapping only by its recorded first and last line text, so code inserted inside a range whose old last position still reads the recorded text (another '}') went unreported and the mapping silently covered the wrong lines. Since BUG-635 the audit knows the commit that linked each mapping and carries mappings through the git line diff from it. fspec audit now carries every committed mapping whose anchors still read their text; when the carried lines differ from the mapped lines it reports coverage impl-range-changed, and --fix rewrites the mapping exactly as BUG-635 does. An edit to the first or last line, or carried anchors that no longer read their text, is reported with the reason. Mappings git cannot carry keep the text check only. git is read in a bounded number of processes per run (one log of the coverage files, one log of every file, one cat-file process, one diff per linking commit).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. fspec audit carries every committed implementation mapping whose recorded first and last lines still read their text through the git line diff since its linking commit (BUG-635's carry); when the carried lines differ from the mapped lines it reports 'coverage impl-range-changed' naming the commit and the carried lines, and --fix rewrites the mapping to them
  #   2. When that diff edits the mapping's first or last line, or the carried first and last lines do not read the recorded text, the problem is reported with the reason and --fix writes nothing
  #   3. A mapping git cannot carry (not a git working tree, not committed, not reading its recorded text at the linking commit) is checked only by its recorded text, as before
  #   4. The check reads git in a bounded number of processes per audit run (one log of the coverage files' commits, one log of every file's commits, one cat-file process for file versions, one git diff per linking commit); a mapping whose file has not changed since its linking commit is not diffed
  #
  # EXAMPLES:
  #   1. Committed mapping src/login.rs 10-24 whose lines 23 and 24 both read '}'; one line is inserted after line 15: the anchors still read their text, fspec audit reports impl-range-changed src/login.rs:10-24 -> 10-25 and --fix rewrites it to 10-25
  #   2. The file is unchanged since the linking commit, or changed only after line 24: no problem
  #   3. The closing line 24 is re-indented from '    }' to '}': the text still reads '}', the diff edits the last line, so the problem is reported saying an edit changed its last line 24 and --fix writes nothing
  #   4. An uncommitted mapping whose code grew inside: no impl-range-changed problem (text check only, as before)
  #
  # ========================================
  Background: User Story
    As a developer relying on fspec audit to keep coverage mappings honest
    I want the audit to notice when the code inside a mapping's range grew or shrank even though its first and last lines still read the same
    So that a mapping never silently covers the wrong lines

  Scenario: Code inserted inside a range whose anchors still read their text is reported and carried
    Given a committed mapping src/login.rs 10-24 whose lines 23 and 24 both read '}'
    When one line is inserted after line 15
    Then fspec audit reports "coverage impl-range-changed" for src/login.rs:10-24 naming lines 10-25
    And fspec audit --fix rewrites the mapping to src/login.rs 10-25

  Scenario: A file unchanged inside the range is not reported
    Given a committed mapping src/login.rs 10-24
    When nothing changes, and then two lines are added after line 24
    Then fspec audit reports no problem for the mapping either time

  Scenario: An edit to the last line under matching text is reported, not fixed
    Given a committed mapping src/login.rs 10-24 whose line 24 reads '}'
    When line 24 is re-indented and still reads '}' when trimmed
    Then fspec audit reports "coverage impl-range-changed" saying an edit changed its last line 24
    And fspec audit --fix leaves the mapping at src/login.rs 10-24

  Scenario: An uncommitted mapping keeps the text check only
    Given an uncommitted mapping src/login.rs 10-24 whose lines 23 and 24 both read '}'
    When one line is inserted after line 15
    Then fspec audit reports no impl-range-changed problem for the mapping
