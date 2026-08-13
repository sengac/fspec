@done
@source-shape
@env-vars
@configuration
@providers
@PROV-097
Feature: fspec binary loads dotenv at startup
  """
  Architecture notes:
  - The pure-Rust `fspec` binary entry (rust/fspec/src/main.rs) must call
  the cwd-only dotenv load once before clap dispatch via the seam
  `startup_env::load_startup_env()`, which runs
  `let _ = dotenvy::from_path(std::path::Path::new(".env"));`. This loads
  <cwd>/.env into the process environment before combined::run / daemon::run
  reach ProviderCredentials::detect() (which reads std::env::var(...)).
  - CWD-ONLY (TS parity): the seam uses `dotenvy::from_path`, NOT
  `dotenvy::dotenv()`. `dotenvy::dotenv()` walks UP parent directories for a
  .env, which bleeds an ancestor .env (e.g. a dev repo-root file of real
  provider keys) into a process whose own cwd has none — breaking spawn-based
  parity tests. `from_path` reads strictly <cwd>/.env, matching the
  TypeScript `join(process.cwd(), '.env')`.
  - dotenvy is already a workspace dependency (rust/Cargo.toml); it must be
  declared in rust/fspec/Cargo.toml [dependencies] as
  `dotenvy.workspace = true` (mirroring rust/cli/Cargo.toml).
  - dotenvy::from_path() is NON-overriding: a key already exported in the shell
  keeps its value; a missing .env returns Err and is ignored.
  - cargo_shape.rs locks the [dependencies] keys list and main.rs line cap;
  adding dotenvy must keep that source-shape test green.
  """

  Background: User Story
    As a developer running the pure-Rust fspec TUI
    I want to have my <cwd>/.env API keys loaded into the process environment at fspec startup
    So that Provider Settings detects env-only providers as configured

  @env-loading
  Scenario: A key present only in .env becomes visible to env detection
    Given a working directory containing a .env file with "ANTHROPIC_API_KEY=sk-ant-xyz"
    And the variable "ANTHROPIC_API_KEY" is not exported in the shell
    When the fspec startup dotenv-load runs in that working directory
    Then "std::env::var" for "ANTHROPIC_API_KEY" returns "sk-ant-xyz"

  @env-loading
  Scenario: A shell-exported variable takes precedence over .env
    Given the variable "ANTHROPIC_API_KEY" is exported as "key-from-shell"
    And a working directory containing a .env file with "ANTHROPIC_API_KEY=key-from-env-file"
    When the fspec startup dotenv-load runs in that working directory
    Then "std::env::var" for "ANTHROPIC_API_KEY" returns "key-from-shell"

  @env-loading
  Scenario: A missing .env file is not an error
    Given a working directory containing no .env file
    And the variable "ANTHROPIC_API_KEY" is exported as "exported-key"
    When the fspec startup dotenv-load runs in that working directory
    Then the load completes without aborting startup
    And "std::env::var" for "ANTHROPIC_API_KEY" returns "exported-key"

  @env-loading
  Scenario: A provider with a key only in .env is reported as configured
    Given a working directory containing a .env file with "COHERE_API_KEY=co-test-key"
    And the variable "COHERE_API_KEY" is not exported in the shell
    When the fspec startup dotenv-load runs in that working directory
    And list_provider_credentials is queried
    Then the "cohere" provider row reports configured as true

  @source-shape
  Scenario: The fspec startup seam calls dotenvy in startup_env and main.rs invokes it before clap parse
    Given the file "rust/fspec/src/startup_env.rs"
    When the source is scanned for "dotenvy::from_path"
    Then it contains a call to "dotenvy::from_path"
    And the file "rust/fspec/src/main.rs" invokes "startup_env::load_startup_env()" before the clap parse of the Cli struct

  @source-shape
  Scenario: The fspec crate declares the dotenvy dependency
    Given the file "rust/fspec/Cargo.toml"
    When the "[dependencies]" table is parsed
    Then it contains a key "dotenvy"
