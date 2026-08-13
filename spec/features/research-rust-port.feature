@done
@rust
@research
@cli
@RPC-286
Feature: Port research command to Rust
  """
  Core impl target: rust/fspec-core/src/commands/research.rs. Signature changes from the
  current stub run(args_json) to the canonical run(args_json, project_root). Args mirror the TS
  ResearchOptions surface exposed by the Commander.js registration at src/commands/research.ts:276-407:
  {tool:Option<String>, workUnit:Option<String>, all:bool, query:Option<String>, args:Vec<String>(forwarded)}.
  Returns a JSON envelope String the CLI bridge decodes to render exact TS stdout/stderr.

  ⚠️ SCOPE FLAG (poll_sync_future / async-process safety) — see WORKER report to supervisor.
  The TS `research` command has TWO modes:
  (A) LIST mode (no --tool): pure file IO + static registry. resolveConfig (src/utils/config-resolution.ts)
  reads env vars + spec/fspec-config.json + ~/.fspec/fspec-config.json synchronously. The static
  TOOL_REGISTRY (ast, perplexity, jira, confluence, stakeholder) is a compile-time table. This mode is
  100% poll_sync_future-SAFE (blocking std::fs only) and is the surface specified in THIS feature file.
  (B) EXECUTE mode (--tool=X ...): NOT poll_sync_future-safe on the dispatcher front-door and partly
  un-portable as parity. Detail:
  - bundled perplexity/jira/confluence/stakeholder tools do real network IO over https → genuine
  async; would return Poll::Pending under single-poll dispatch.
  - bundled ast tool calls @sengac/codelet-napi astGrepSearch/astGrepRefactor (async NAPI). Rust HAS
  a native ast-grep core (the AstGrep tool / codelet) so this COULD be re-implemented natively, but
  that is a sizeable sub-port needing its own decision.
  - custom tools load via dynamic `import()` of arbitrary user spec/research-tools/<name>.js modules
  → NO Rust equivalent (cannot import JS plugins).
  - script tools (spec/research-scripts/*) execute via child_process.spawn awaiting the 'close'
  event. This single sub-path IS portable with BLOCKING std::process::Command::output() (resolves
  on first poll, sync-safe) — but only if the supervisor wants script execution in scope.
  EXECUTE-mode behaviour is therefore DEFERRED pending a supervisor scope decision and is intentionally
  NOT specified here beyond the pre-spawn argument-validation / tool-not-found error paths, which ARE
  deterministic and sync-safe.

  Reuse / new shared infrastructure (Phase C, supervisor-owned wiring): a config-resolution helper mirroring
  src/utils/config-resolution.ts (env → user → project → default precedence) under rust/fspec-core/src/io/
  or a new module; a static tool registry table. SHARED-FILE REQUESTS to supervisor (deferred to Phase C):
  (1) canonical.rs PORTED_COMMANDS add 'research'; (2) dispatch.rs move research from run_stub to run_ported
  with run(args_json, project_root); (3) main.rs Mode::Research clap variant {tool, work_unit, all, trailing
  var-args} + allow-unknown forwarding + forward! arm + intercept_ts_help arm + mod research;
  (4) help/configs/mod.rs register research::CONFIG. Worker owns: commands/research.rs (rewrite stub),
  help/configs/research.rs, fspec/src/research.rs bridge, tests, fixture.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. With no --tool flag the command operates in LIST mode: it emits the static research tool registry
  #      (ast, perplexity, jira, confluence, stakeholder) each annotated with a configuration status derived
  #      from resolveConfig (ENV vars → ~/.fspec/fspec-config.json → spec/fspec-config.json → defaults).
  #   2. The 'ast' tool has no required config fields, so it is always reported CONFIGURED ('✓'); the other
  #      tools are reported NOT CONFIGURED ('✗') unless all their required fields resolve to non-empty values.
  #   3. perplexity requires apiKey; jira and confluence require url+token; stakeholder requires at least one
  #      platform webhook/token. Required fields satisfied via env vars or config file flip the indicator to ✓.
  #   4. LIST mode performs only blocking file/env reads and never spawns a process or opens a socket, so it is
  #      safe under the single-poll sync dispatcher used by the LLM front-door.
  #   5. EXECUTE mode (--tool given) first validates inputs BEFORE any tool work: an unknown tool name yields a
  #      'Research tool not found: <name>' error; the deterministic pre-execution validation path is in scope.
  #   6. Actual execution of network/NAPI/dynamic-JS research tools is OUT OF SCOPE for this port pending a
  #      supervisor scope decision (see SCOPE FLAG docstring); the dispatcher must not return Poll::Pending.
  #
  # EXAMPLES:
  #   1. Empty project, no config: dispatching research with no flags returns the five bundled tools with ast
  #      marked configured and the remaining four marked not-configured.
  #   2. Configured perplexity: with PERPLEXITY_API_KEY set (or spec/fspec-config.json research.perplexity.apiKey),
  #      perplexity is reported CONFIGURED.
  #   3. Unknown tool: dispatching research --tool=does-not-exist returns a 'Research tool not found' error.
  #
  # ========================================
  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want to port the research command's list/discovery surface to Rust as a parity port
    So that the standalone Rust binary and the dispatcher can enumerate research tools and their configuration
    status without falling back to TypeScript

  Scenario: List mode enumerates the bundled tool registry with ast configured by default
    Given an empty project root tempdir with no spec/fspec-config.json and no research env vars
    When I dispatch research with no flags
    Then the dispatcher returns success
    And the result lists the tool "ast"
    And the result lists the tool "perplexity"
    And the result lists the tool "jira"
    And the result lists the tool "confluence"
    And the result lists the tool "stakeholder"
    And the tool "ast" is reported as configured
    And the tool "perplexity" is reported as not configured

  Scenario: List mode reflects a configured perplexity api key from project config
    Given a project root tempdir whose spec/fspec-config.json sets research.perplexity.apiKey to "pplx-test"
    When I dispatch research with no flags
    Then the dispatcher returns success
    And the tool "perplexity" is reported as configured

  Scenario: List mode reports stakeholder as configured when its required webhook field is present
    Given a project root tempdir whose spec/fspec-config.json sets research.stakeholder.teamsWebhook to "https://example.test/hook"
    When I dispatch research with no flags
    Then the dispatcher returns success
    And the tool "stakeholder" is reported as configured

  Scenario: List mode does not create any files or spawn any process
    Given an empty project root tempdir with no spec subdirectory
    When I dispatch research with no flags
    Then the dispatcher returns success
    And no spec/fspec-config.json is created in the project root
    And the command completes without spawning a child process or opening a network socket

  Scenario: Execute mode rejects an unknown tool name before doing any work
    Given an empty project root tempdir
    When I dispatch research with tool="does-not-exist"
    Then the dispatcher returns an error
    And the error message contains "Research tool not found: does-not-exist"

  Scenario: Both front doors converge on the same fspec-core function
    Given an empty project root tempdir
    When I dispatch research with no flags via the dispatcher and via the standalone binary
    Then both invocations enumerate the same five bundled research tools
