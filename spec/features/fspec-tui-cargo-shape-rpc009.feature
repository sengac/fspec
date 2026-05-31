@done
@tui
@rust
@infrastructure
@rpc
@parity
@RPC-009
@critical
Feature: codelet/fspec-tui Cargo + source-shape regression (RPC-009)
  """
  Source-shape regression widening: codelet/fspec-tui/tests/source_shape_cargo.rs gains an assertion that `[dependencies]` lists `tui-input` (workspace dep) alongside the existing entries (`codelet-rpc`, `codelet-rpc-types`, `codelet-rpc-embedded`, `codelet-rpc-server`, `tokio`, `async-trait`, `futures`, `ratatui`, `crossterm`, `tui-popup`, `tokio-tungstenite`, `url`, `tarpc`, `anyhow`, `tracing`); the existing assertions that `codelet-napi`/`codelet-core` are NOT in `[dependencies]` and `codelet-napi` is NOT in `[dev-dependencies]` remain green. codelet/fspec-tui/tests/source_shape_trait.rs widens its scan loop's directory list from `["src"]` to additionally cover `src/views/` (already a subdir of `src/`, so the recursive `collect_rs_files` walker picks them up automatically — the test asserts that NONE of the new files contain forbidden patterns: `tokio::runtime::Builder`, `runtime::Builder::new_multi_thread`, `runtime::Builder::new_current_thread`, `tokio::runtime::Runtime::new`, `Runtime::new()`, NOR direct imports of `codelet_napi::`, `codelet_core::`, `tarpc::`, or `tokio_tungstenite::` (these stay encapsulated in the existing transport/embedded.rs and transport/websocket.rs files). codelet/Cargo.toml [workspace.dependencies] gains `tui-input = "0.10"` in the `# === Terminal & UI ===` block.
  """

  Background: User Story
    As a fspec developer building the ratatui frontend
    I want the existing RPC-008 source-shape regression tests widened so codelet/fspec-tui/Cargo.toml [dependencies] now lists tui-input alongside the existing entries while still excluding codelet-napi/codelet-core, [dev-dependencies] still excludes codelet-napi, and the new src/views/*.rs files contain no tokio::runtime::Builder/Runtime::new calls and no direct codelet_napi/codelet_core/tarpc/tokio_tungstenite imports
    So that the trait-only seam invariant from RPC-008 is preserved as the new view layer is added in RPC-009

  Scenario: codelet/fspec-tui/Cargo.toml [dependencies] now lists tui-input alongside the existing entries
    Given codelet/fspec-tui/Cargo.toml exists
    When the test parses the manifest's `[dependencies]` table
    Then the table contains `tui-input` (consumed via `tui-input.workspace = true`)
    And the table still contains `codelet-rpc`, `codelet-rpc-types`, `codelet-rpc-embedded`, `codelet-rpc-server`
    And the table still contains `tokio`, `async-trait`, `futures`, `ratatui`, `crossterm`, `tui-popup`
    And the table still contains `tokio-tungstenite`, `url`, `tarpc`, `anyhow`, `tracing`

  Scenario: codelet/Cargo.toml [workspace.dependencies] declares tui-input = "0.10"
    Given codelet/Cargo.toml exists
    When the test parses the manifest's `[workspace.dependencies]` table
    Then the table contains an entry `tui-input = "0.10"` in the `# === Terminal & UI ===` block

  Scenario: Production [dependencies] of codelet/fspec-tui still excludes codelet-napi and codelet-core
    Given codelet/fspec-tui/Cargo.toml exists
    When the test parses the manifest's `[dependencies]` table
    Then the table does NOT contain `codelet-napi`
    And the table does NOT contain `codelet-core`

  Scenario: dev-dependencies of codelet/fspec-tui still excludes codelet-napi (codelet-core stays allowed for fixtures)
    Given codelet/fspec-tui/Cargo.toml exists
    When the test parses the manifest's `[dev-dependencies]` table
    Then the table does NOT contain `codelet-napi`
    And the table MAY contain `codelet-core` (allowed for real-service fixtures per RPC-008 Q-DEV-CORE-1)

  Scenario: New src/views/*.rs files preserve the host-supplied tokio runtime invariant (Q9)
    Given the source files codelet/fspec-tui/src/views/work_units_list.rs, agent_repl.rs, root.rs, footer.rs, and mod.rs exist
    When the test scans each file's body
    Then NO file contains `tokio::runtime::Builder`
    And NO file contains `runtime::Builder::new_multi_thread`
    And NO file contains `runtime::Builder::new_current_thread`
    And NO file contains `tokio::runtime::Runtime::new`
    And NO file contains `Runtime::new()`

  Scenario: New src/views/*.rs files do not directly import the encapsulated transport crates
    Given the source files codelet/fspec-tui/src/views/*.rs exist
    When the test scans each file's `use` declarations
    Then NO file directly imports `codelet_napi::`
    And NO file directly imports `codelet_core::`
    And NO file directly imports `tarpc::`
    And NO file directly imports `tokio_tungstenite::`
    And every interaction with the backend goes through `Arc<dyn FspecBackend>` (or the `FspecBackend` trait surface)
