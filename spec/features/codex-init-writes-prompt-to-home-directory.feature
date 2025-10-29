@done
@init
@file-ops
@agent-compatibility
@agent-integration
@scaffolding
@AGENT-018
Feature: Codex init writes fspec prompt to user home directory
  """
  Architecture notes:
  - Extend fspec init codex installer to resolve the user home directory with Node's os.homedir()
  - Generate fspec.md directly into `${home}/.codex/prompts/fspec.md`
  - Ensure mkdirp-style directory creation so ~/.codex/prompts exists before writing the file
  - Do not modify the project-level ./.codex/prompts/fspec.md during generation
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. fspec.md must be copied into the user's global ~/.codex/prompts directory rather than the project-level .codex/prompts
  #   2. Home directory resolution must use a cross-platform API (e.g., Node os.homedir()) instead of hard-coded paths or environment assumptions
  #   3. If ~/.codex/prompts does not exist it must be created before copying fspec.md
  #   4. The fspec prompt auto-generation routine must write fspec.md directly into the resolved ~/.codex/prompts directory
  #   5. No changes to existing project-level prompt; fspec init only needs to generate the home-directory copy.
  #
  # EXAMPLES:
  #   1. Given a Linux user with HOME=/home/sam, running the Codex fspec prompt setup copies ./.codex/prompts/fspec.md to /home/sam/.codex/prompts/fspec.md
  #   2. Given a Windows user with USERPROFILE=C:\Users\Riley, the setup writes fspec.md to C:\Users\Riley\.codex\prompts\fspec.md by resolving the home directory rather than hard-coding ~
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should we delete the project-level .codex/prompts/fspec.md after copying to the user directory, or keep it for backward compatibility?
  #   A: No changes to existing project-level prompt; fspec init only needs to generate the home-directory copy.
  #
  # ========================================
  Background: User Story
    As a Codex CLI maintainer
    I want fspec init to install the Codex prompt into the user's ~/.codex/prompts directory
    So that Codex always loads fspec guidance from a consistent agent prompt location

  Scenario: Generate Codex prompt in user home directory on Unix-like systems
    Given the user's home directory resolves to "/home/sam"
    And the directory "/home/sam/.codex/prompts" does not exist
    When I run `fspec init --agent=codex`
    Then the command should create the directory "/home/sam/.codex/prompts"
    And the generated prompt file should be written to "/home/sam/.codex/prompts/fspec.md"
    And the project-level "./.codex/prompts/fspec.md" should remain unchanged

  Scenario: Resolve Codex prompt location using os.homedir on Windows
    Given the user's home directory resolves to "C:\\Users\\Riley"
    When I run `fspec init --agent=codex`
    Then the generated prompt file should be written to "C:\\Users\\Riley\\.codex\\prompts\\fspec.md"
    And the resolved home directory string should be used to build the prompt path

  Scenario: Re-running fspec init keeps project-level prompt intact
    Given the user's home directory resolves to "/Users/alex"
    And the directory "/Users/alex/.codex/prompts" already contains "fspec.md"
    And the project directory contains "./.codex/prompts/fspec.md"
    When I rerun `fspec init --agent=codex`
    Then "fspec.md" in "/Users/alex/.codex/prompts" should be replaced with the regenerated content
    And "./.codex/prompts/fspec.md" in the project should remain untouched

  Scenario: Generated Codex instructions reference /prompts:fspec command
    Given I run `fspec init --agent=codex`
    When the command prints usage instructions for the Codex prompt
    Then the guidance should instruct running "/prompts:fspec"
    And the guidance should not mention running "/fspec"
