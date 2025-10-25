@done
@reverse-engineering
@file-discovery
@high
@strategy-detection
@reverse-acdd
@cli
@REV-003
Feature: Strategy D processes existing feature files instead of scanning src/ for implementation files
  """
  Uses tinyglobby for recursive file discovery in src/ directory. Implements deriveFeatureName() function for camelCase/PascalCase to kebab-case conversion. Integrates with existing detectGaps() and suggestStrategy() functions in src/commands/reverse.ts. Modifies GapAnalysis type to replace rawImplementation with unmappedImplementation field.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Strategy D should scan src/ directory recursively for .tsx, .ts, .jsx, .js files
  #   2. For each implementation file found, Strategy D should check if a corresponding feature file exists in spec/features/
  #   3. Strategy D should only process implementation files that have NO corresponding feature file
  #   4. Yes, use tinyglobby with pattern 'src/**/*.{ts,tsx,js,jsx}' to recursively find all implementation files, excluding test directories (__tests__, tests, test) and test files (*.test.ts, *.spec.ts)
  #   5. Use filename similarity with kebab-case conversion algorithm: deriveFeatureName() converts camelCase/PascalCase implementation filenames to kebab-case (e.g., MusicPlayer.tsx → music-player.feature, usePlaylistStore.ts → use-playlist-store.feature). Then check if spec/features/{kebab-name}.feature exists. This is simpler and more predictable than import/export analysis.
  #
  # EXAMPLES:
  #   1. MindStrike project has MusicPlayer.tsx (1532 lines) with no feature file. Strategy D should detect this and guide: 'Analyze MusicPlayer.tsx implementation → Create feature file → Create test file → Link coverage'
  #   2. Running 'fspec reverse --strategy=D' outputs: 'Step 1 of 7: Read test file: spec/features/cli-command-interface-for-ai-agent-control.feature. Then create feature file.' This is wrong - it's reading a feature file (not implementation), and the feature already exists.
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should Strategy D use glob patterns to find implementation files (e.g., src/**/*.{ts,tsx,js,jsx})?
  #   A: true
  #
  #   Q: How should Strategy D match implementation files to feature files? By filename similarity (MusicPlayer.tsx → music-player.feature) or by analyzing imports/exports?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a AI agent using fspec reverse for codebases without prior fspec usage
    I want to have Strategy D automatically discover implementation files in src/ and guide me through creating feature files for them
    So that I can apply outside-in BDD to existing codebases without manually specifying which files need features

  Scenario: Detect unmapped implementation file without corresponding feature
    Given a project with src/components/MusicPlayer.tsx implementation file
    Given no feature file exists at spec/features/music-player.feature
    When AI runs 'fspec reverse' to analyze project gaps
    Then Strategy D (Full Reverse ACDD) should be suggested
    Then gap analysis should report '1 implementation files without features'
    Then session.gaps.files should contain 'src/components/MusicPlayer.tsx'

  Scenario: Process implementation files instead of existing feature files when running Strategy D
    Given a project with unmapped src/components/MusicPlayer.tsx
    Given an existing unrelated feature file spec/features/cli-command-interface.feature
    When AI runs 'fspec reverse --strategy=D'
    Then Step 1 guidance should show 'Implementation file: src/components/MusicPlayer.tsx'
    Then guidance should NOT say 'Read test file: spec/features/cli-command-interface.feature'
    Then guidance should include persona-driven prompts 'WHO uses this?'

  Scenario: Convert implementation filenames to expected feature file names using kebab-case
    Given deriveFeatureName() function receives 'src/components/MusicPlayer.tsx'
    When the function converts PascalCase to kebab-case
    Then it should return 'music-player'
    Then hasFeatureFile() should check for 'spec/features/music-player.feature'
