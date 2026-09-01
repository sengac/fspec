@done
@bug
@bug-fix
@tui
@persistence
@BUG-167
Feature: Mux config persistence wiring — shared fspec-config.json dirs resolve in every App
  """
  Bug: /mux save, the MuxConfigDialog 's' commit, and the mux-exit auto-save all
  called MuxState::save() with the persist dirs NEVER set in production
  (set_mux_persist_dir was only ever called from tests), so every save returned
  Err('mux: persist dirs not set') which the callers swallowed — the shared
  fspec-config.json tui.mux key was never written and load always fell back to
  the default preset.

  Fix (SOLID/DRY — same wiring as the rest of the fspec-config.json
  persistence): the process-global data directory is the single source of
  truth for the shared-config dirs. Production entry points initialize it
  exactly once before the App is constructed (combined/daemon mode already
  does this in build_service → ~/.fspec; client mode now does it in
  client::run → ~/.fspec). MuxState keeps the config ONLY (no dirs) and
  delegates load/save to the existing codelet_sessions::
  mux_config_persistence globals (load_mux_config / save_mux_config), which
  resolve the CONFIG-008 two-scope dirs themselves (process-global data dir
  + current dir — the same resolution default_thinking_level_persistence and
  last_used_model_persistence already use); the manual MuxState::set_persist_dir
  + App::set_mux_persist_dir plumbing is removed. When the global is unset
  (unit tests that don't root it), load degrades to the default preset and
  save surfaces a logged error — never a write to an uncontrolled path.

  Tests: persistence integration tests root the process-global data directory
  at a temp dir (codelet_common::set_data_directory, the established tui093
  pattern, serialised) and drive the real save paths end-to-end; tests that
  need a NON-global cwd (the project-scope override) use the path-injectable
  *_with_dirs core directly.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R1: Every App construction site (production and test) gets working
  #      mux persistence with zero manual wiring — App::new is the single
  #      wiring point, mirroring how the rest of the fspec-config.json
  #      persistence resolves its dirs from the process-global data directory
  #   2. R2: The shared-config dirs resolve to the SAME two-scope pair the
  #      CONFIG-008 fspec-config.json uses everywhere else: user scope =
  #      codelet_common::get_data_dir() (~/.fspec in the binary), project
  #      scope = std::env::current_dir() (spec/fspec-config.json overrides
  #      via deep merge)
  #   3. R3: A mux save (MuxConfigDialog 's' commit, /mux save, or the
  #      mux-exit auto-save) writes tui.mux into the USER-scope shared
  #      fspec-config.json preserving sibling keys, with no manual
  #      set-persist-dirs call — the manual MuxState dirs plumbing is removed
  #   4. R4: A missing or unresolvable data directory is non-fatal: load
  #      falls back to the default preset and save surfaces a logged error
  #      (the /mux save scrollback notice), never a panic
  #   5. R5: The persistence behavior stays byte-identical to the existing
  #      mux_config_persistence core (read-modify-write to USER scope; load
  #      reads the deep-merged view) — only the dirs wiring changes, not the
  #      on-disk layout
  #
  # EXAMPLES:
  #   1. In the real binary (combined/daemon mode, data dir initialized by
  #      build_service), opening the mux config dialog, setting Orientation
  #      to Vertical and pressing 's' writes ~/.fspec/fspec-config.json with
  #      a tui.mux key (orientation "Vertical"), preserving the sibling
  #      'agent' and 'tools' keys
  #   2. Exiting mux mode (dialog commit with Enabled: Off, or /mux off)
  #      auto-saves the post-exit config (enabled=false) to the same tui.mux
  #      key so a restart comes back with mux off
  #
  # ========================================
  Background: User Story
    As a developer supervising agents in the TUI
    I want to save my mux grid config (dialog 's' key, /mux save, or mux exit)
    So that the grid persists in the shared fspec-config.json and is restored on the next TUI start

  # R1 + R2: App::new wires the shared-config dirs — a save with NO manual
  # persist-dirs call (the production shape) lands in the data directory
  Scenario: /mux save persists to the shared fspec-config.json with no manual persist-dirs wiring
    Given an App constructed with a backend and no explicit persist-dirs setup
    And the process-global data directory is rooted at a throwaway directory
    When mux mode is active and I submit the slash command "/mux save"
    Then the fspec-config.json under the data directory contains the tui.mux key
    And the tui.mux value holds the live mux config (orientation, splits, pane list, focused pane, enabled)

  # R3 (dialog 's' path): the config dialog save commits + persists without
  # any set-persist-dirs call
  Scenario: committing the mux config dialog with 's' persists tui.mux to the shared config
    Given an App constructed with a backend and no explicit persist-dirs setup
    And the process-global data directory is rooted at a throwaway directory
    And the MuxConfigDialog is open with the Orientation row set to Vertical
    When I press 's' to apply and save
    Then the MuxConfigDialog is closed and the live mux layout is vertical
    And the fspec-config.json under the data directory contains a tui.mux key with orientation "Vertical"
    And pre-existing sibling keys in the shared config are preserved

  # R3 (mux-exit auto-save): exiting mux mode persists the post-exit config
  Scenario: exiting mux mode auto-saves the post-exit config to tui.mux
    Given an App constructed with a backend and no explicit persist-dirs setup
    And the process-global data directory is rooted at a throwaway directory
    And mux mode is active
    When I submit the slash command "/mux off"
    Then the fspec-config.json under the data directory contains a tui.mux key
    And the saved tui.mux value has enabled=false and the live panes and splits

  # R2: the project-scope override still wins on load
  Scenario: a project-scope tui.mux overrides the user-scope value on load
    Given a user-scope fspec-config.json with tui.mux holding a horizontal 50/50 Board|Agent preset
    And a project spec/fspec-config.json with tui.mux holding a vertical 40/60 Board|Agent preset
    When a fresh TUI bootstrap loads the persisted mux config
    Then the live mux config is vertical with a 40/60 split (the project value wins)

  # R4: a missing data directory is non-fatal — load degrades to the default
  # preset, save surfaces a logged error instead of a panic
  Scenario: an unresolvable data directory never panics load or save
    Given the process-global data directory cannot be resolved and no fallback home is available
    When a fresh TUI bootstrap loads the persisted mux config
    Then the live mux config is the default preset (horizontal Board|Agent 50/50)
    And submitting "/mux save" surfaces a one-line error notice in the agent scrollback
    And the TUI remains usable (no panic)
