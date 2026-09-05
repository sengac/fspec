@done
@agent-view
@RPC-020
@rust
@source-shape
@tui
@rpc
Feature: RPC-020 source-shape regression for the slash + file search popup port
  """
  RPC-020 introduces several new source artefacts that downstream cards
  (RPC-021 / RPC-022) rely on. This feature pins the file layout +
  symbol surface so future refactors cannot silently regress the
  integration shape:

  1. rust/core/src/file_search.rs — new helper module exposing
  `pub fn search(cwd, prefix, limit) -> Vec<String>` backed by
  `ignore::WalkBuilder` + `globset::GlobBuilder` (case-insensitive).
  2. rust/rpc/src/lib.rs — `FspecService` trait gains
  `async fn search_files(prefix: String, limit: u32) -> Vec<String>`.
  3. rust/fspec-tui/src/transport/mod.rs — `FspecBackend` trait
  declares `async fn search_files(prefix: String, limit: u32) -> Result<Vec<String>>`.
  4. rust/fspec-tui/src/transport/embedded.rs +
  rust/fspec-tui/src/transport/websocket.rs — both implement
  the new trait method.
  5. rust/fspec-tui/src/views/agent/slash_commands.rs — new module
  declaring `SlashCommand`, `SlashCommandAction`, `SLASH_COMMANDS`,
  and `filter_commands`. Under 300 LoC.
  6. rust/fspec-tui/src/views/agent/slash_command_popup.rs — new
  widget module declaring `SlashCommandPopup`. Under 300 LoC.
  7. rust/fspec-tui/src/views/agent/file_search_popup.rs — new
  widget module declaring `FileSearchPopup`. Under 300 LoC.
  8. rust/fspec-tui/src/views/agent.rs (orchestrator) — owns
  `slash_popup: Option<SlashCommandPopup>` and
  `file_popup: Option<FileSearchPopup>`. Stays under 300 LoC.
  9. rust/fspec-tui/src/components/mod.rs — Action enum gains
  three additive variants: SlashCommandSelected,
  SearchFiles, FileSearchResults.

  Existing TS code paths
  (src/tui/components/SlashCommandPalette.tsx,
  src/tui/components/FileSearchPopup.tsx,
  src/tui/hooks/useSlashCommandInput.ts,
  src/tui/hooks/useFileSearchInput.ts,
  src/tui/utils/slashCommands.ts) are NOT touched.
  """

  Background: User Story
    As a Rust fspec TUI developer
    I want the RPC-020 source layout to be locked in by a regression test
    So that future cards inheriting SlashCommandAction / FspecBackend::search_files continue to find them where they expect

  Scenario: codelet_core::file_search helper module exists with the documented surface
    Given rust/core/src/file_search.rs after RPC-020 lands
    Then the file exists
    And the file contains the substring "pub fn search"

  Scenario: codelet_core::lib re-exports the new file_search module
    Given rust/core/src/lib.rs after RPC-020 lands
    Then the file contains the substring "pub mod file_search"

  Scenario: FspecService trait gains the search_files RPC method
    Given rust/rpc/src/lib.rs after RPC-020 lands
    Then the file contains the substring "async fn search_files"
    And the file contains the substring "prefix: String"
    And the file contains the substring "limit: u32"

  Scenario: FspecBackend trait declares the search_files method
    Given rust/fspec-tui/src/transport/mod.rs after RPC-020 lands
    Then the file contains the substring "async fn search_files"

  Scenario: Both transports implement the search_files FspecBackend method
    Given the rust/fspec-tui crate after RPC-020 lands
    Then rust/fspec-tui/src/transport/embedded.rs contains the substring "async fn search_files"
    And rust/fspec-tui/src/transport/websocket.rs contains the substring "async fn search_files"

  Scenario: New slash_commands module exists with the documented surface
    Given the rust/fspec-tui crate after RPC-020 lands
    Then the file rust/fspec-tui/src/views/agent/slash_commands.rs exists
    And the file contains the substring "pub struct SlashCommand"
    And the file contains the substring "pub enum SlashCommandAction"
    And the file contains the substring "pub const SLASH_COMMANDS"
    And the file contains the substring "pub fn filter_commands"

  Scenario: New SlashCommandPopup module exists with the documented surface
    Given the rust/fspec-tui crate after RPC-020 lands
    Then the file rust/fspec-tui/src/views/agent/slash_command_popup.rs exists
    And the file contains the substring "pub struct SlashCommandPopup"

  Scenario: New FileSearchPopup module exists with the documented surface
    Given the rust/fspec-tui crate after RPC-020 lands
    Then the file rust/fspec-tui/src/views/agent/file_search_popup.rs exists
    And the file contains the substring "pub struct FileSearchPopup"

  Scenario: AgentView orchestrator owns the new popup fields
    Given rust/fspec-tui/src/views/agent.rs after RPC-020 lands
    Then the file contains the substring "slash_popup"
    And the file contains the substring "file_popup"
    And the file contains the substring "SlashCommandPopup"
    And the file contains the substring "FileSearchPopup"

  Scenario: Action enum gains three new variants
    Given rust/fspec-tui/src/components/mod.rs after RPC-020 lands
    Then the file contains the substring "SlashCommandSelected"
    And the file contains the substring "SearchFiles"
    And the file contains the substring "FileSearchResults"

  Scenario: Every file under views/agent/ and views/agent.rs stays under 300 lines
    Given the directory rust/fspec-tui/src/views/agent/ plus the views/agent.rs orchestrator
    When a test counts the line-count of every .rs file
    Then every file in views/agent/ has fewer than 300 lines
    And the orchestrator file views/agent.rs has fewer than 300 lines

  Scenario: Views do not directly import codelet_core / napi / tarpc / tokio_tungstenite
    Given the directory rust/fspec-tui/src/views/ (including views/agent/) after RPC-020 lands
    When a test scans every *.rs file
    Then no file imports `codelet_core::` or `codelet_napi::` or `tarpc::` or `tokio_tungstenite::`
    And no file constructs `tokio::runtime::Builder` or `Runtime::new()`

  @no-alias
  @ts-parity
  Scenario: SlashCommandAction enum contains no Providers variant
    Given rust/fspec-tui/src/views/agent/slash_commands.rs after the 2026-06-01 revision
    When the source is parsed for SlashCommandAction variants
    Then the enum contains "Provider" exactly once
    And the enum does NOT contain a "Providers" variant
    And the SLASH_COMMANDS const contains exactly one entry whose action is SlashCommandAction::Provider
    And no entry in SLASH_COMMANDS has the name "providers"

  @no-alias
  @ts-parity
  Scenario: dispatch_slash_commands.rs has no Providers arm
    Given rust/fspec-tui/src/app/dispatch_slash_commands.rs after the 2026-06-01 revision
    When the file is read
    Then it contains exactly one arm matching "SlashCommandAction::Provider =>"
    And it does NOT contain "SlashCommandAction::Providers"
    And it does NOT contain "| SlashCommandAction::Providers"
