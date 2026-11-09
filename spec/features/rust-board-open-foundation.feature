@done
@navigation
@viewer
@RPC-373
Feature: Wire D key on board to open FOUNDATION.md in browser
  """
  BoardView::handle_event (rust/fspec-tui/src/views/board.rs) gains a KeyCode::Char('d')|Char('D') arm beside the existing f/c arms that emits Action::OpenFoundation and returns EventResult::consumed(). A new Action::OpenFoundation variant is added to src/components/mod.rs. App::bootstrap starts the attachment viewer via codelet_attachment_viewer::start_viewer(cwd) (cwd from std::env::current_dir()) best-effort/non-fatal like checkpoint_counts, storing ViewerHandle + port (Option<u16>) on App state. A new dispatch_viewer.rs helper (mirroring dispatch_changed_files.rs) handles Action::OpenFoundation: it computes App::foundation_target()->Option<String> = viewer_port.map(|p| format!("http://127.0.0.1:{p}/view/spec/FOUNDATION.md")) and, when Some, spawns open::that(url) (the open crate, promoted to a workspace dep). The pure foundation_url(port)->String and App::foundation_target() are unit-testable so no real browser launches in tests; the open::that call sits behind the Some branch only. All files <300 lines.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Pressing D or d on the board emits an OpenFoundation action and consumes the key event
  #   2. TUI-120: OpenFoundation reads spec/FOUNDATION.md of the selected workspace from the daemon (spec/features/foundation-viewer-and-fspec-runner-follow-the-selected-workspace.feature)
  #   3. (BUG-469) The App no longer starts a viewer on the TUI's directory at bootstrap; viewers serve files read from the daemon
  #
  # EXAMPLES:
  #   1. Pressing uppercase D on the focused board emits Action::OpenFoundation and the event is consumed
  #   2. Pressing lowercase d on the focused board emits Action::OpenFoundation and the event is consumed
  #
  # ========================================
  Background: User Story
    As a fspec board user
    I want to press the D key on the board to open the project's FOUNDATION.md in my browser
    So that I can read the rich rendered foundation document without leaving my workflow

  Scenario: Pressing uppercase D opens the foundation document
    Given the board view is focused
    When I press the uppercase D key
    Then the open-foundation action is emitted
    And the key event is consumed

  Scenario: Pressing lowercase d opens the foundation document
    Given the board view is focused
    When I press the lowercase d key
    Then the open-foundation action is emitted
    And the key event is consumed
