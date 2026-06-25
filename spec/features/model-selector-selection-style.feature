@done
@ts-parity
@model-selection
@tui
@RPC-351
Feature: Model selector view selection/arrow style does not match /provider TS-parity

  """
  Selection styling lives in views/model_selector/rows_render.rs (model rows) and header.rs (provider headers). Replaces Modifier::REVERSED|BOLD with a solid Style::default().bg(Color::Cyan).fg(Color::Black) band, mirroring provider_settings/row_render.rs:132-136 full-width pre-fill loop. Model-local SEL='> ' / NOSEL='  ' constants follow provider_settings/icons.rs. /provider view is reference-only and must not change.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Selected rows paint a solid Color::Cyan background with Color::Black foreground, not Modifier::REVERSED
  #   2. The selection band fills the full row width (every cell from area.x to area.x+area.width)
  #   3. Selected model rows render a '  > ' arrow marker; unselected render '    ' of equal width
  #   4. Selected header rows prepend '> ' before the expand icon; unselected prepend '  '
  #   5. Every inline coloured token flips to Color::Black when its row is selected: badges, (current), folder icon, (unreachable)
  #   6. The band colour is uniform cyan for both header and model rows (no per-kind tint)
  #   7. Unselected rows are unchanged: white/accent colours, DIM badges, no band
  #
  # EXAMPLES:
  #   1. A selected model row paints cyan-bg/black-fg across the full row width with a '> ' arrow
  #   2. A selected header row prepends '> ' before the ▼ expand icon
  #   3. A selected model row with [C][R][V][ctx] badges and (current) marker renders all those tokens black
  #   4. A selected profile header renders 📁 and (unreachable) black, not magenta/red
  #   5. An unselected model row keeps white label, DIM coloured badges, and no cyan band
  #   6. A short selected row still has cyan background painted to the right edge of the row
  #
  # ========================================

  Background: User Story
    As a fspec user navigating the /model selector
    I want to see the selected row highlighted with a solid cyan band and a > arrow matching /provider and the TS reference
    So that the /model view looks consistent with /provider and its own TypeScript source

  Scenario: A selected model row paints a solid cyan band with a > arrow
    Given the model selector lists a model row
    When that model row is rendered while selected
    Then the row paints a solid cyan background with black foreground
    And the row is not styled with reverse video
    And the row shows a "> " arrow marker

  Scenario: The selection band fills the full row width
    Given the model selector lists a short model row
    When that model row is rendered while selected
    Then the cyan background extends to the right edge of the row

  Scenario: A selected header row prepends the selection marker before the expand icon
    Given the model selector shows an expanded provider header row
    When that header row is rendered while selected
    Then the header prepends "> " before the expand icon
    And the header paints a solid cyan background with black foreground

  Scenario: An unselected header row prepends padding before the expand icon
    Given the model selector shows an expanded provider header row
    When that header row is rendered while not selected
    Then the header prepends two spaces before the expand icon
    And the header shows no cyan background

  Scenario: A selected model row flips every inline token to black
    Given the model selector lists a model row with custom, reasoning, vision and context badges that is the current model
    When that model row is rendered while selected
    Then the badges are rendered black
    And the "(current)" marker is rendered black

  Scenario: A selected profile header flips the folder and unreachable markers to black
    Given the model selector shows an unreachable profile header row
    When that header row is rendered while selected
    Then the folder icon is rendered black rather than magenta
    And the "(unreachable)" marker is rendered black rather than red

  Scenario: An unselected model row is unchanged
    Given the model selector lists a model row with badges
    When that model row is rendered while not selected
    Then the model label keeps its white foreground
    And the badges keep their accent colours and are dimmed
    And the row shows no cyan background
