@done
@validator
@coverage
@bug-fix
@rust
@BUG-635
Feature: audit --fix carries implementation mappings through the git line diff
  """
  fspec audit --fix shifted a drifted implementation mapping by one offset when its recorded first line occurred once and its last line read the recorded text at the same offset (COV-079). Lines inserted inside the range left it the same length, so when another '}' sat at the old offset it ended one line short of the code it covered. --fix now carries the mapping through the exact git line diff (git diff -U0, myers) from the implementation file at the commit that linked the mapping to the file the audit reads: unchanged mapped lines move to their new numbers, lines inserted or changed strictly inside a contiguous mapped run join it, deleted lines leave it, and lines inserted before the first or after the last mapped line stay outside. An edit to the mapping's first or last line, a mapping git cannot carry (not a git working tree, not committed, or not reading its recorded text at the linking commit), or a carry whose first and last lines do not read the recorded text is reported with the reason instead of fixed (product owner, 2026-11-08: the text-only shift is replaced entirely).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. audit --fix carries a drifted implementation mapping through the git line diff (git diff -U0) from the implementation file at the commit that linked the mapping to the working-tree file: an unchanged mapped line moves to its new number, lines inserted or changed strictly between two lines of a contiguous mapped run join the mapping, deleted lines inside it leave the mapping, and lines inserted before the first or after the last mapped line stay outside it
  #   2. When a hunk changes or deletes a mapping's first or last line, or the carried first and last lines do not read the recorded firstLine and lastLine, --fix writes nothing and the problem names why and the link-coverage command; nothing is guessed
  #   3. A mapping git cannot carry (not a git working tree, the mapping not committed, the linking commit's file not reading the recorded text) is reported with that reason; the text-only constant-offset shift of COV-079 is replaced by the carry
  #   4. Product owner, 2026-11-08: replace COV-079's text-only shift entirely. --fix repairs only through the exact git line diff from the commit that linked the mapping; an edit touching the first or last mapped line, or a mapping that is not committed yet, is reported instead of shifted.
  #
  # EXAMPLES:
  #   1. src/a.rs [10-20] (fn at 10, '}' at 20); 3 lines inserted after line 4: --fix rewrites it to [13-23]
  #   2. 3 lines inserted after line 15 (inside): --fix rewrites [10-20] to [10-23], the last line '}' at 23, not [10-20]
  #   3. 2 lines inserted directly before line 10 (at the start): --fix rewrites it to [12-22] and the inserted lines stay outside
  #   4. 2 lines inserted directly after line 20 (at the end): the mapping still reads its text and is not a problem
  #   5. Line 10 (the signature) is edited: --fix writes nothing and the problem says the edit changed the mapping's first line
  #   6. The mapping was linked but not committed and its code moved: --fix writes nothing; the problem says git cannot show how its lines moved
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the git-diff carry fully replace COV-079's text-only shift (firstLine found once, lastLine at the same offset, every line moved by one offset)? That shift is what cut the range one line short: with lines inserted inside a range ending in '}', another '}' sat at the old offset. RECOMMENDED: replace it. --fix then repairs only through the exact git line diff from the commit that linked the mapping, and a mapping that is not committed yet (linked and edited in the same working tree) is reported with 'git cannot show how its lines moved' instead of being shifted. Alternative: keep the text shift only for a mapping git cannot carry, and only when the range length is unchanged and the carried last line is not a line that occurs more than once. That's still a text match, so it is a heuristic.
  #   A: Product owner, 2026-11-08: replace COV-079's text-only shift entirely. --fix repairs only through the exact git line diff from the commit that linked the mapping; an edit touching the first or last mapped line, or a mapping that is not committed yet, is reported instead of shifted.
  #
  # ========================================
  Background: User Story
    As a developer keeping coverage mappings right while editing implementation code
    I want fspec audit --fix to carry each implementation mapping through my edits exactly, or tell me it cannot
    So that a fixed mapping covers exactly the code it covered before, never one line short

  Scenario: Lines inserted before a committed mapping shift it
    Given a committed mapping src/login.rs 10-24 for the log_in function
    When 3 lines are inserted after line 4 and fspec audit --fix runs
    Then the mapping becomes src/login.rs 13-27

  Scenario: Lines inserted inside a committed mapping extend it
    Given a committed mapping src/login.rs 10-24 for the log_in function, whose line 23 closes an inner block with '}'
    When 3 lines are inserted after line 4 and 1 line after line 15, and fspec audit --fix runs
    Then the mapping becomes src/login.rs 13-28, ending at the function's closing line, not 13-27

  Scenario: Lines inserted directly before the first line stay outside the mapping
    Given a committed mapping src/login.rs 10-24 for the log_in function
    When 2 lines are inserted directly before line 10 and fspec audit --fix runs
    Then the mapping becomes src/login.rs 12-26

  Scenario: Lines inserted directly after the last line leave the mapping as it is
    Given a committed mapping src/login.rs 10-24 for the log_in function
    When 2 lines are inserted directly after line 24
    Then fspec audit reports no problem for the mapping

  Scenario: An edit to the mapping's first line is reported, not fixed
    Given a committed mapping src/login.rs 10-24 for the log_in function
    When line 4 gains a line after it and the signature on line 10 is edited, and fspec audit --fix runs
    Then the mapping stays src/login.rs 10-24
    And fspec audit reports it as impl-text-changed saying an edit changed its first line 10

  Scenario: A mapping that is not committed is reported, not shifted
    Given an uncommitted mapping src/login.rs 10-24 whose function moved unchanged to lines 30-44
    When fspec audit --fix runs
    Then the mapping stays src/login.rs 10-24
    And fspec audit reports it saying the mapping is not committed, so git cannot show how its lines moved
