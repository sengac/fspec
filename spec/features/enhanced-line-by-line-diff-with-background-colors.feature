@tui
@GIT-008
@diff-viewer
@visualization
@enhancement
Feature: Enhanced line-by-line diff with background colors
  As a developer reviewing changes
  I want to see line-by-line diffs with clear background colors
  So that I can easily identify what was replaced, added, or removed

  Background: 
    Given the diff viewer is displaying file changes
    And the diff pane is focused

  # ============================================================================
  # Example Mapping
  # ============================================================================
  #
  # Story: Enhanced Diff Visualization
  #
  # Rules:
  # 1. Replaced lines (old) show with RED background and WHITE text
  # 2. Replaced lines (new) show with GREEN background and WHITE text
  # 3. Pure deletions show with RED background and WHITE text
  # 4. Pure additions show with GREEN background and WHITE text
  # 5. Context lines show with default styling (no background)
  # 6. Hunk headers show with CYAN text
  # 7. Consecutive - and + lines are paired as replacements
  # 8. Unpaired - lines are pure deletions
  # 9. Unpaired + lines are pure additions
  #
  # Examples:
  # - Simple replacement: one - line followed by one + line
  # - Multi-line replacement: multiple - lines followed by multiple + lines
  # - Pure addition: only + lines with no preceding - lines
  # - Pure deletion: only - lines with no following + lines
  # - Mixed changes: combination of replacements, additions, deletions
  #
  # Questions:
  # Q: Should we show side-by-side or sequential view?
  # A: Sequential (unified) view initially, side-by-side can be future enhancement
  #
  # Q: How to handle intra-line diffs (character-level changes)?
  # A: Future enhancement - start with line-level only
  #
  # Q: What about word-level highlighting within replaced lines?
  # A: Future enhancement - start with full-line background colors
  #
  # Q: How to ensure accessibility with color choices?
  # A: Use sufficient contrast (white text on red/green background)
  #
  # Q: Performance with very large diffs?
  # A: Already handled by VirtualList + 100KB truncation
  #
  # ============================================================================
  Scenario: Simple line replacement shows old line in red, new line in green
    Given a file with the following unified diff:
      """
      @@ -1,3 +1,3 @@
       const x = 1;
      -const y = 2;
      +const y = 3;
       const z = 4;
      """
    When I view the diff
    Then I should see line "const x = 1;" with default styling
    And I should see line "-const y = 2;" with RED background and WHITE text
    And I should see line "+const y = 3;" with GREEN background and WHITE text
    And I should see line "const z = 4;" with default styling
    And I should see line "@@ -1,3 +1,3 @@" with CYAN text

  Scenario: Multi-line replacement shows paired lines with color backgrounds
    Given a file with the following unified diff:
      """
      @@ -1,5 +1,5 @@
       function calculate() {
      -  const total = a + b;
      -  return total;
      +  const sum = a + b;
      +  return sum * 2;
       }
      """
    When I view the diff
    Then I should see line "-  const total = a + b;" with RED background and WHITE text
    And I should see line "-  return total;" with RED background and WHITE text
    And I should see line "+  const sum = a + b;" with GREEN background and WHITE text
    And I should see line "+  return sum * 2;" with GREEN background and WHITE text
    And the removed and added lines should be visually grouped as a replacement

  Scenario: Pure addition shows only green background
    Given a file with the following unified diff:
      """
      @@ -1,2 +1,3 @@
       const x = 1;
      +const y = 2;
       const z = 3;
      """
    When I view the diff
    Then I should see line "+const y = 2;" with GREEN background and WHITE text
    And there should be no RED background lines before it
    And it should be clearly identified as a pure addition

  Scenario: Pure deletion shows only red background
    Given a file with the following unified diff:
      """
      @@ -1,3 +1,2 @@
       const x = 1;
      -const y = 2;
       const z = 3;
      """
    When I view the diff
    Then I should see line "-const y = 2;" with RED background and WHITE text
    And there should be no GREEN background lines after it
    And it should be clearly identified as a pure deletion

  Scenario: Mixed changes show appropriate colors for each type
    Given a file with the following unified diff:
      """
      @@ -1,6 +1,6 @@
       const a = 1;
      -const b = 2;
      +const b = 3;
       const c = 4;
      +const d = 5;
      -const e = 6;
       const f = 7;
      """
    When I view the diff
    Then I should see:
      | Line          | Background | Text  | Type        |
      | const a = 1;  | default    | white | context     |
      | -const b = 2; | RED        | WHITE | replacement |
      | +const b = 3; | GREEN      | WHITE | replacement |
      | const c = 4;  | default    | white | context     |
      | +const d = 5; | GREEN      | WHITE | addition    |
      | -const e = 6; | RED        | WHITE | deletion    |
      | const f = 7;  | default    | white | context     |

  Scenario: Unbalanced replacements handle many-to-one changes
    Given a file with the following unified diff:
      """
      @@ -1,4 +1,2 @@
      -const a = 1;
      -const b = 2;
      -const c = 3;
      +const total = 6;
       const d = 4;
      """
    When I view the diff
    Then I should see 3 lines with RED background (old lines)
    And I should see 1 line with GREEN background (new line)
    And they should be visually grouped as a multi-to-one replacement

  Scenario: Unbalanced replacements handle one-to-many changes
    Given a file with the following unified diff:
      """
      @@ -1,2 +1,4 @@
      -const total = 6;
      +const a = 1;
      +const b = 2;
      +const c = 3;
       const d = 4;
      """
    When I view the diff
    Then I should see 1 line with RED background (old line)
    And I should see 3 lines with GREEN background (new lines)
    And they should be visually grouped as a one-to-multi replacement

  Scenario: Long lines are wrapped but maintain background color
    Given a file with a line longer than terminal width:
      """
      -const veryLongVariableName = 'this is a very long string that exceeds the terminal width and will wrap to multiple lines';
      +const veryLongVariableName = 'this is a different very long string that also exceeds the terminal width';
      """
    When I view the diff
    Then the removed line should show RED background across all wrapped portions
    And the added line should show GREEN background across all wrapped portions
    And the wrapped lines should maintain WHITE text color

  Scenario: Empty lines in diff maintain appropriate background
    Given a file with the following unified diff:
      """
      @@ -1,3 +1,3 @@
      -
      +  // Added comment
       function test() {}
      """
    When I view the diff
    Then the removed empty line should show RED background
    And the added line should show GREEN background
    And empty lines should be visually distinguishable
