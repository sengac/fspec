@wip
@cli
@outside-in-bdd
@strategy-detection
@reverse-acdd
@high
@FEAT-019
Feature: Reverse ACDD Strategy D Not Detecting Implementation Files
  """
  Strategy D uses system-reminders to guide AI through outside-in BDD workflow. Foundation.json personas are central to discovery. Semi-automatic coverage linking balances automation with educational transparency. Feature-scoped sessions allow multiple implementation files per user-facing feature.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Strategy D guides AI agents through outside-in BDD: Context → Personas → User Behavior → Example Maps → Features → Test Skeletons → Implementation Links
  #   2. Strategy D uses foundation.json personas to help AI understand WHO uses the system and WHAT they want to accomplish (not HOW it's implemented)
  #   3. AI provides implementation context as input, Strategy D helps transform it into user-centric feature specifications and example maps
  #   4. Strategy D should fill gaps backward from behavior to implementation: Example Maps → Features → Tests → Coverage Links to existing code
  #   5. Yes. Strategy D should actively prompt with persona-based questions using system-reminders to guide AI thinking. Example: 'WHO uses this? (Check foundation.json personas)' and 'WHAT does [persona] want to accomplish?'. This provides cognitive scaffolding to help AI shift from inside-out code-thinking to outside-in behavior-thinking. Prompts should be guiding (not blocking), conditional on foundation.json existence, and include examples of good persona-driven thinking.
  #   6. Foundation.json must exist (run 'fspec discover-foundation' if missing as one-time setup). During Strategy D, when persona is needed but missing, prompt AI to add it incrementally via 'fspec add-persona <name> <description> --goal <goal>'. For granular code (APIs, functions), guide AI to identify which human persona BENEFITS (not which system calls it) using system-reminders: 'Not who calls calculateDiscount() but who benefits from accurate discounts? Answer: Shopper'. Skip feature files for pure utilities (formatDate, parseJSON) - test-only coverage.
  #   7. Yes. Strategy D should provide transformation templates via system-reminders to help AI translate implementation → behavior. Templates: (1) UI Elements → User Actions (button → 'User clicks/taps ACTION', input → 'User enters DATA'), (2) State → User Expectations (useState → 'User sees STATE', loading → 'User waits for PROCESS'), (3) API Endpoints → User Needs (POST /orders → 'User completes order'). Templates accelerate mental shift, ensure consistency, and reinforce outside-in thinking.
  #   8. Pause for review with guided prompts. After AI creates example map, Strategy D emits system-reminder with review checklist: 'Do rules cover examples?', 'Are examples concrete?', 'All questions answered?', 'Examples map to personas?'. Then offers explicit choice: (1) Generate features now: fspec generate-scenarios, (2) Add more examples/rules. AI explicitly chooses when to generate (not automatic). Example Mapping is a conversation requiring reflection, not a form-filling exercise. Quality gate prevents garbage-in-garbage-out.
  #   9. Semi-automatic with educational system-reminder. Strategy D auto-links coverage by default using 'fspec link-coverage --skip-validation' but emits system-reminder showing: (1) What was linked (scenario → test skeleton → implementation), (2) Exact command that was run (educational), (3) How to verify: 'fspec show-coverage', (4) How to fix if wrong: 'fspec unlink-coverage' then re-link manually. Balances low friction (automatic) with transparency (not a black box) and learning (AI sees the mechanism).
  #   10. Feature-scoped sessions (multiple related files allowed). Strategy D allows multiple implementation files in one session if they deliver the SAME user-facing feature. System-reminder guides AI: 'Do these files deliver same user behavior?' YES → One work unit, one example map, one feature file, multiple test files (one per implementation). NO → Separate sessions. BDD focuses on user behavior which often spans multiple files. Track all files in work unit metadata under 'reverseSession.implementationFiles[]'. Link all via coverage to same feature file.
  #
  # EXAMPLES:
  #   1. AI: 'I have MusicPlayer.tsx with play/pause/skip buttons' → Strategy D: 'Who uses this?' → AI sees foundation.json personas: Music Listener persona → Strategy D guides: 'What does Music Listener want?' → AI creates example map: 'Music Listener plays song', 'Music Listener pauses playback' → Generates feature file + test skeletons → Links coverage to MusicPlayer.tsx
  #   2. AI: 'I have usePlaylistStore.ts with addSong(), removeSong(), clearPlaylist()' → Strategy D: 'What user behavior does this support?' → AI identifies: Managing playlists → Creates example map with rules: 'User can add songs', 'User can remove songs' → Generates playlist-management.feature → Creates test skeleton → Links to usePlaylistStore.ts implementation
  #   3. Project has foundation.json with 'Mobile App User' persona (goal: Stream music on the go) → AI provides AudioPlayer.tsx context → Strategy D prompts: 'How does Mobile App User stream music?' → AI creates example map: 'User taps play', 'User adjusts volume', 'User sees album art' → Feature file generated with user-centric scenarios
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should Strategy D scan src/ for files lacking feature files in spec/features?
  #   A: true
  #
  #   Q: When AI provides implementation context (e.g., 'MusicPlayer.tsx has play/pause buttons'), should Strategy D prompt with persona-based questions like 'Which persona from foundation.json uses this feature?' or 'What does [persona] want to accomplish?'
  #   A: true
  #
  #   Q: If foundation.json is missing or has no personas, should Strategy D guide AI through creating foundation first via 'fspec discover-foundation', or allow manual persona specification for this reverse session only?
  #   A: true
  #
  #   Q: Should Strategy D provide templates or prompts to help AI transform implementation details (buttons, functions, state) into user behavior descriptions (plays song, pauses playback, manages playlist)?
  #   A: true
  #
  #   Q: After AI creates example maps (rules, examples, questions), should Strategy D automatically generate feature files and test skeletons, or should it prompt AI to review/refine the example map first?
  #   A: true
  #
  #   Q: Should Strategy D guide AI to link test skeletons to existing implementation using 'fspec link-coverage --skip-validation' automatically, or require AI to manually create coverage links?
  #   A: true
  #
  #   Q: Should Strategy D track multiple implementation files in a single reverse session (e.g., MusicPlayer.tsx + usePlaylistStore.ts together), or require separate sessions per file?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a AI agent using fspec for reverse ACDD
    I want to be guided through outside-in BDD discovery when I have implementation files without features
    So that I create user-centric specifications that trace behavior back to existing code, rather than just scanning files inside-out

  Scenario: Persona-driven discovery from UI component implementation
    Given AI provides implementation context "MusicPlayer.tsx with play/pause/skip buttons"
    And foundation.json exists with "Music Listener" persona
    When Strategy D prompts "Who uses this? Check foundation.json personas"
    Then AI identifies "Music Listener" persona from foundation.json
    And Strategy D prompts "What does Music Listener want to accomplish?"
    And AI creates example map with examples "Music Listener plays song" and "Music Listener pauses playback"
    And AI runs "fspec generate-scenarios" to create feature file
    And Strategy D generates test skeletons
    And Strategy D auto-links coverage to MusicPlayer.tsx implementation

  Scenario: Behavior identification from state management implementation
    Given AI provides implementation context "usePlaylistStore.ts with addSong(), removeSong(), clearPlaylist()"
    And foundation.json exists with personas
    When Strategy D prompts "What user behavior does this support?"
    Then AI identifies user behavior "Managing playlists"
    And AI creates example map with rules "User can add songs" and "User can remove songs"
    And AI runs "fspec generate-scenarios" to create playlist-management.feature
    And Strategy D generates test skeleton
    And Strategy D links coverage to usePlaylistStore.ts implementation

  Scenario: Foundation-driven scenario generation from audio implementation
    Given foundation.json exists with "Mobile App User" persona with goal "Stream music on the go"
    And AI provides implementation context "AudioPlayer.tsx"
    When Strategy D prompts "How does Mobile App User stream music?"
    Then AI creates example map with examples "User taps play", "User adjusts volume", "User sees album art"
    And AI runs "fspec generate-scenarios" to create feature file
    And generated feature file contains user-centric scenarios based on Mobile App User persona
