@done
@tui-component
@session-management
@TUI-046
Feature: Detach Confirmation Modal on AgentView Exit
  """
  Modifies AgentView.tsx ESC handler (Priority 5) to show ThreeButtonDialog instead of calling onExit() directly. Uses sessionManagerDestroy() from codelet-napi for session destruction. Session handler cleanup via GlobalSessionStreamManager. Reuses ThreeButtonDialog component from TUI-040 for DRY/SOLID compliance.


  SUPERSESSION NOTE (BUG-205): the three-option 'Detach / Close Session /
  Cancel' modal was ported to Rust (RPC-098) and the Detach option was
  later removed by BUG-205 — the live dialog is the two-button
  [Close Session, Cancel] form pinned by
  spec/features/exit-session-dialog-two-button-options.feature). This
  historical feature is retained as-is.
  """
