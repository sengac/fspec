@critical
@infrastructure
@RPC-010
@rpc
@rust
@workspace
@build
Feature: fspec binary — crate layout, source-shape invariants, and build artifact
  """
  RPC-010 (parent RPC-002, depends on RPC-009). Source-shape +
  workspace-layout regression rules. Locks the new rust/fspec/
  crate's structure so future cards (especially RPC-011 polish) cannot
  accidentally pull in NAPI, construct an own runtime, or fork the
  binary entry point.

  Invariants locked in this card (rules [0] [7] [13] [16]):
  • rust/fspec/ is a workspace member with [[bin]] name = "fspec",
  path = "src/main.rs".
  • cargo build -p fspec --release produces rust/target/release/fspec.
  • rust/fspec/Cargo.toml [dependencies] does NOT list codelet-napi.
  • rust/fspec/Cargo.toml [dev-dependencies] does NOT list codelet-napi.
  • rust/fspec/src/ contains NO `tokio::runtime::Builder`,
  `Runtime::new`, `Runtime::new()`, `runtime::Builder::new_*` calls
  (preserving the RPC-005 Q9 host-supplied-runtime invariant).
  • The existing `codelet-rpc-server` dev-helper binary stays in place;
  its stdout port-line contract is preserved verbatim by `fspec daemon`.
  • An `npm run build:rust:fspec` script copies the artifact to dist/fspec
  for parity with the TS layout; npm `bin` stays on the TS shim.

  Source artifacts:
  • rust/Cargo.toml [workspace] (modified — add `fspec` to members)
  • rust/fspec/Cargo.toml (NEW)
  • rust/fspec/src/{main,combined,daemon,client,common}.rs (NEW)
  • rust/rpc-embedded/tests/architecture_invariants.rs (modified — widen scan)
  • package.json (modified — add `build:rust:fspec` script)
  """

  @smoke
  Scenario: rust/fspec is registered as a workspace member
    Given the file `rust/Cargo.toml` exists
    When the test parses `[workspace].members`
    Then the members list contains the string `fspec`
    And `fspec` appears between `core` and `fspec-core` in the members list (preserving alphabetical order: agent-loop, cli, common, core, fspec, fspec-core, fspec-tui, git, graph, napi, providers, rpc, rpc-embedded, rpc-server, rpc-types, sessions, test-helpers, tools, tui)

  @smoke
  Scenario: rust/fspec/Cargo.toml declares a single bin named fspec
    Given the file `rust/fspec/Cargo.toml` exists
    Then it contains a `[[bin]]` table with `name = "fspec"`
    And the same `[[bin]]` table sets `path = "src/main.rs"`
    And no other `[[bin]]` table is declared

  @smoke
  Scenario: cargo build -p fspec --release produces rust/target/release/fspec
    When the developer runs `cargo build -p fspec --release` from `rust/`
    Then the build completes with exit code 0
    And the file `rust/target/release/fspec` exists and is executable

  @smoke
  Scenario: fspec --version prints the workspace version
    When the developer runs `rust/target/release/fspec --version`
    Then the command exits with code 0
    And STDOUT contains the workspace version string declared in `rust/Cargo.toml [workspace.package].version`

  @smoke
  Scenario: fspec --help shows three subcommands
    When the developer runs `rust/target/release/fspec --help`
    Then the command exits with code 0
    And the help text mentions exactly the subcommands `daemon` and `client`
    And the help text describes the no-subcommand default as combined mode
    And the help text mentions the `--workspace` flag

  @smoke
  Scenario: rust/fspec/src/ contains exactly the locked file layout
    Given the directory `rust/fspec/src/` exists
    Then the directory contains the files `main.rs`, `combined.rs`, `daemon.rs`, `client.rs`, `common.rs`
    And no other `.rs` files exist directly under `rust/fspec/src/`
    And each file in the directory is under 300 lines of code

  @smoke
  Scenario: rust/fspec/Cargo.toml [dependencies] does NOT list codelet-napi
    Given the file `rust/fspec/Cargo.toml` exists
    When the test parses the `[dependencies]` table
    Then there is no key named `codelet-napi`
    And there is no key whose name starts with `codelet-napi`

  @smoke
  Scenario: rust/fspec/Cargo.toml [dev-dependencies] does NOT list codelet-napi
    Given the file `rust/fspec/Cargo.toml` exists
    When the test parses the `[dev-dependencies]` table
    Then there is no key named `codelet-napi`

  @smoke
  Scenario: rust/fspec/Cargo.toml declares the expected production dependencies
    Given the file `rust/fspec/Cargo.toml` exists
    When the test parses the `[dependencies]` table
    Then it contains keys: `clap`, `tokio`, `anyhow`, `tracing`, `tracing-subscriber`, `tracing-appender`, `dirs`, `serde`, `serde_json`, `url`
    And it contains keys: `codelet-rpc`, `codelet-rpc-types`, `codelet-rpc-embedded`, `codelet-rpc-server`, `codelet-fspec-tui`, `codelet-core`

  @smoke
  @regression
  Scenario: No source file under rust/fspec/src/ constructs its own tokio runtime
    """
    Preserves RPC-005 Q9 at the binary boundary. The only runtime is
    the one driven by `#[tokio::main]` on `main.rs`; everywhere else
    must use `tokio::runtime::Handle::current()` or `tokio::spawn`.
    """
    Given the directory `rust/fspec/src/` exists
    When the test scans every `.rs` file under the directory recursively
    Then no file contains the literal substring `tokio::runtime::Builder`
    And no file contains the literal substring `runtime::Builder::new_multi_thread`
    And no file contains the literal substring `runtime::Builder::new_current_thread`
    And no file contains the literal substring `tokio::runtime::Runtime::new`
    And no file contains the literal substring `Runtime::new()`

  @smoke
  @regression
  Scenario: The RPC-005 source-shape invariant is widened to scan rust/fspec/src/
    """
    The existing source-shape regression at
    `rust/rpc-embedded/tests/architecture_invariants.rs` (RPC-005
    + RPC-008's widening) MUST be widened again to include
    `rust/fspec/src/` in its directory list.
    """
    Given the file `rust/rpc-embedded/tests/architecture_invariants.rs` exists
    Then the file contains the substring `"fspec/src"` in its scanned-directory list

  @smoke
  Scenario: The existing codelet-rpc-server dev-helper binary stays in place
    Given the file `rust/rpc-server/src/main.rs` exists
    Then the file still defines `#[tokio::main] async fn main()` (unchanged from RPC-006)
    And `rust/rpc-server/Cargo.toml` still declares the binary
    And no commit in this card has removed those files

  @smoke
  @parity
  Scenario: Existing codelet-rpc-server test harness still works (port-line contract preserved)
    Given the test `rust/rpc-server/tests/websocket_transport.rs::spawn_rpc_server` exists
    When the developer runs `cargo test -p codelet-rpc-server`
    Then the test passes with the OLD `codelet-rpc-server` binary path
    And no behaviour change has been introduced to the RPC-006 binary

  @smoke
  @parity
  Scenario: A new spawn_fspec_daemon helper proves the port-line contract is verbatim
    """
    Locks the cross-binary parity: `fspec daemon` is a drop-in
    replacement for `codelet-rpc-server` at the STDOUT contract layer.
    """
    given the test file `rust/fspec/tests/daemon_mode.rs` exists
    Then it defines a `spawn_fspec_daemon` helper that uses `BufReader::read_line`
    And the helper parses the first STDOUT line as a bare integer port
    And the same parsing logic mirrors `rust/rpc-server/tests/websocket_transport.rs::spawn_rpc_server`

  @smoke
  Scenario: package.json declares the build:rust:fspec script
    Given the file `package.json` exists
    When the test parses the `scripts` object
    Then `scripts["build:rust:fspec"]` exists
    And the script invokes `cargo build -p fspec --release` (or runs a wrapper that does)
    And the script copies `rust/target/release/fspec` to `dist/fspec`

  @smoke
  @end-to-end
  Scenario: npm run build:rust:fspec produces dist/fspec for parity with the TS layout
    When the developer runs `npm run build:rust:fspec` from the repo root
    Then the command exits with code 0
    And the file `dist/fspec` exists and is executable

  @smoke
  Scenario: npm bin entry remains on the TS shim (no npm install path swap in this card)
    Given the file `package.json` exists
    When the test reads the `bin` object
    Then the `fspec` binary path still points at the existing TS shim (NOT `dist/fspec`)
    And the README has NOT been updated to advertise the Rust binary as the npm install path

  @smoke
  @regression
  Scenario: --workspace defaults to CWD when omitted
    Given a tempdir `<W>` containing a seeded spec/work-units.json
    When the developer runs `cd <W> && fspec daemon` (no --workspace flag)
    Then the daemon's WorkUnitsWatcher is rooted at `<W>`
    And calls to `list_work_units` return the work units seeded in `<W>/spec/work-units.json`

  @smoke
  @regression
  Scenario: --workspace <path> overrides CWD for the WorkUnitsWatcher root
    Given a tempdir `<A>` containing seeded work-units (id "A-1")
    And a different tempdir `<B>` containing seeded work-units (id "B-1")
    When the developer runs `cd <A> && fspec daemon --workspace <B>`
    Then the daemon's WorkUnitsWatcher is rooted at `<B>`
    And `list_work_units` returns the work units from `<B>` (NOT `<A>`)

  @smoke
  @regression
  Scenario: Both `fspec` (combined) and `fspec daemon` honour the same --workspace resolution
    Given a tempdir `<W>` containing seeded work-units
    When the developer runs `fspec --workspace <W>` (combined)
    Then the embedded backend's `list_work_units` returns the seeded units
    When the developer runs `fspec daemon --workspace <W>`
    Then the WebSocket-attached client's `list_work_units` returns the same seeded units

  @smoke
  @regression
  Scenario: Existing Vitest smoke at napi-workunitinfo-shape.test.ts remains green
    When the developer runs `npm test -- src/__tests__/napi-workunitinfo-shape.test.ts`
    Then the test passes unchanged
    And no NAPI surface has been altered by this card

  @smoke
  @regression
  Scenario: Existing cargo test suites for RPC-005..009 remain green
    When the developer runs `cargo test -p codelet-rpc-embedded -p codelet-rpc-server -p codelet-fspec-tui` from `rust/`
    Then every test passes unchanged
    And no test in those crates has been modified for this card except the source-shape widening referenced above
