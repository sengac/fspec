@BUG-582
@done
@init
@agent-compatibility
@initialization
@BUG-533
Feature: Codex init writes its prompt to the platform home directory
  """
  fspec-core commands/init/files.rs: HomePlatform {Unix, Windows} (current() from cfg!(windows), home_var() USERPROFILE | HOME) and HomeEnv {platform, home} (process() reads the platform variable; from_vars(platform, lookup) for tests; home_dir() -> Option<PathBuf>, empty = None) replace home_dir() and its silent '.' fallback. init::run delegates to the new pub init::run_with_home(args, root, &HomeEnv); install_agent_files, install_slash_command and generate_agent_doc take &HomeEnv; reinstall_agent_files (sync-version) uses HomeEnv::process(). run_with_home checks the home before any write when a codex agent is requested. Tests: fspec-core/tests/bug533_codex_init_home_per_platform.rs drives run_with_home with injected platforms and variables in temp dirs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The home directory is read from the platform's own variable, as Node's os.homedir (the TypeScript original) does: USERPROFILE on Windows, HOME on every other platform; an empty value counts as unset
  #   2. The Codex prompt (codex and codex-cli) is written to <home>/.codex/prompts/fspec.md, never under the project
  #   3. When the home directory cannot be resolved, init for codex or codex-cli refuses before writing any file, naming the variable it read; there is no fallback to the current or project directory (no silent defaults)
  #   4. The user config ~/.fspec/fspec-config.json that fills <test-command> and <quality-check-commands> in the agent doc is read from the same resolved home; with no home it contributes nothing, as before
  #   5. The platform and the environment are injected into init, so a test chooses Windows or Unix and its variables without depending on the OS it runs on or the machine's real home
  #
  # EXAMPLES:
  #   1. Windows, USERPROFILE=C:\Users\dev and no HOME: fspec init --agent=codex writes C:\Users\dev\.codex\prompts\fspec.md and the project has no .codex directory
  #   2. Linux, HOME=/home/dev: fspec init --agent=codex-cli writes /home/dev/.codex/prompts/fspec.md as before
  #   3. Linux with HOME unset but USERPROFILE set (a stray variable): init for codex fails naming HOME; USERPROFILE is not read on Linux
  #   4. Windows, USERPROFILE points at a profile whose .fspec/fspec-config.json sets tools.test.command npm test: fspec init --agent=claude fills <test-command> with npm test
  #   5. Windows with USERPROFILE unset: fspec init --agent=codex fails saying the home directory cannot be found because USERPROFILE is not set, and neither spec/AGENTS.md nor .codex is written in the project
  #
  # ========================================
  Background: User Story
    As a developer initialising fspec for Codex on Windows
    I want to have the Codex prompt installed in my user profile's .codex/prompts
    So that Codex finds /prompts:fspec and my project is not polluted with a stray .codex folder

  Scenario: On Windows the Codex prompt goes to the user profile
    Given the platform is Windows with USERPROFILE set to a home directory and HOME unset
    When fspec is initialised for "codex" in an empty project
    Then the home directory holds .codex/prompts/fspec.md
    And the project has no .codex directory

  Scenario: On Unix the Codex prompt goes to HOME
    Given the platform is Unix with HOME set to a home directory
    When fspec is initialised for "codex-cli" in an empty project
    Then the home directory holds .codex/prompts/fspec.md
    And the project has no .codex directory

  Scenario Outline: Codex init refuses when the platform's home variable is not set
    Given the platform is <platform> with <home variable> unset and <other variable> set to a home directory
    When fspec is initialised for "codex" in an empty project
    Then init fails saying the home directory cannot be found because <home variable> is not set
    And the project holds neither spec/AGENTS.md nor a .codex directory
    And the other directory holds no .codex directory

    Examples:
      | platform | home variable | other variable |
      | Windows  | USERPROFILE   | HOME           |
      | Unix     | HOME          | USERPROFILE    |

  Scenario: The agent doc reads the user config from the Windows profile
    Given the platform is Windows with USERPROFILE set to a home directory whose .fspec/fspec-config.json sets the test command "npm test"
    When fspec is initialised for "claude" in an empty project
    Then the project's spec/AGENTS.md has "npm test" in place of <test-command>
