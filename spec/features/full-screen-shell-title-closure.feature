@done
@RPC-339
@tui
@rust
@ui-refinement
Feature: Full-screen shell title-renderer closure variant

  """
  render_full_screen_scaffold_with_title<T, B> lives in views/full_screen_shell.rs alongside the existing render_full_screen_scaffold and render_full_screen_scaffold_raw_title; body = Clear.render + Layout [Length(1),Length(1),Min(0),Length(1)] -> (split[0], split[2], split[3]); calls title_fn(title_area), body_fn(body_area), render_footer_hint(footer_area, hint), then overlay branch. render_title_with_count is defined in views/agent/mode_view_render.rs (not the shell). The count wrapper re-expresses its title via the closure: |a,b| render_title_with_count(a,b,title,count,suffix).
  """

  Background: User Story
    As a Rust TUI developer
    I want the shared full-screen shell to accept a caller-supplied title-renderer closure
    So that views with non-count titles can reuse the same Clear/split/overlay scaffold

  Scenario: Shell paints a view through the title-renderer closure variant
    Given a full-screen area and a caller-supplied title closure, body closure, and a static footer hint
    When the view is rendered via render_full_screen_scaffold_with_title with overlay None
    Then the title closure paints the first row
    And the body closure paints the body sub-rect below the separator
    And the static footer hint paints the last row
    And no ConfirmDialog overlay is drawn

  Scenario: Count-title wrapper preserves the title-count format
    Given a view rendered via the count-title wrapper render_full_screen_scaffold
    When it is called with title "Resume Session", count 5, and suffix "available"
    Then the title row reads "Resume Session (5 available)"
    And the rendered output is identical to the pre-RPC-339 baseline
