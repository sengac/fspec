@TUI-076
Feature: Consolidate provider types

  """
  Architecture Context (see spec/attachments/TUI-076/implementation-guide.md for details):

  TUI-034 introduced ModelSelection for hierarchical model selector.
  PROV-007 added profileName/profileConfig for local server support.
  
  Current duplication:
  - ModelSelection: only in AgentView.tsx (line 248)
  - ProviderSection: DUPLICATED in AgentView.tsx (line 267) AND provider.ts (line 18)
  - ModelSelectorItem: only in AgentView.tsx (line 286)
  
  Target: All types consolidated in src/tui/types/provider.ts.
  
  Type compatibility: profileConfig uses inline type in AgentView but ProfileConfig
  import in provider.ts - these are structurally identical (TypeScript structural typing).
  
  ModelSelectorItem depends on NapiModelInfo from @sengac/codelet-napi, which is
  already imported in provider.ts.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # NOTE: Rule/example indices use stable IDs with soft delete.
  # Indices [0]-[3] were deleted during refinement - remaining items keep original IDs.
  #
  # BUSINESS RULES:
  #   [4] ModelSelection interface must be exported from src/tui/types/provider.ts
  #   [5] ModelSelectorItem type must be exported from src/tui/types/provider.ts
  #   [6] ProviderSection interface must remain in src/tui/types/provider.ts (already exists)
  #   [7] AgentView.tsx must import types from ../types/provider instead of defining them locally
  #   [8] No duplicate type definitions allowed - each type defined in exactly one place
  #   [9] Helper functions (buildFlatModelList, etc.) remain in AgentView.tsx - moved in TUI-072
  #
  # EXAMPLES:
  #   [4] Developer imports { ModelSelection } from '../types/provider' in AgentView.tsx
  #   [5] Developer imports { ModelSelectorItem } from '../types/provider' in AgentView.tsx
  #   [6] grep 'interface ModelSelection' src/tui returns only types/provider.ts
  #   [7] grep 'type ModelSelectorItem' src/tui returns only types/provider.ts
  #   [8] npm run build succeeds after type consolidation with no TS errors
  #   [9] npm test passes - all existing model selection tests work with imported types
  #
  # ASSUMPTIONS:
  #   - This is a pure refactoring - no behavior changes, only type location changes
  #   - Helper functions stay in AgentView.tsx for now - moved in TUI-072
  #   - ModelSelectorView.tsx already imports correctly from types/provider
  #   - NapiModelInfo import already exists in provider.ts (verified)
  #
  # ========================================

  Background: User Story
    As a developer
    I want to have model selection types consolidated in src/tui/types/provider.ts
    So that I can import all provider-related types from a single source without duplicates

  Scenario: ModelSelection type exported from provider.ts
    Given the types/provider.ts file exists
    When I check for ModelSelection interface definition
    Then ModelSelection is exported from src/tui/types/provider.ts
    And ModelSelection is NOT defined in AgentView.tsx


  Scenario: ModelSelectorItem type exported from provider.ts
    Given the types/provider.ts file exists
    When I check for ModelSelectorItem type definition
    Then ModelSelectorItem is exported from src/tui/types/provider.ts
    And ModelSelectorItem is NOT defined in AgentView.tsx
    And NapiModelInfo is imported in provider.ts for ModelSelectorItem dependency


  Scenario: AgentView imports types from provider.ts
    Given types are consolidated in types/provider.ts
    When I check AgentView.tsx imports
    Then AgentView imports ModelSelection from ../types/provider
    And AgentView imports ModelSelectorItem from ../types/provider
    And AgentView imports ProviderSection from ../types/provider


  Scenario: Build and tests pass after consolidation
    Given types are consolidated in types/provider.ts
    When I run the build and test commands
    Then npm run build succeeds with no TypeScript errors
    And AgentView.tsx imports types from ../types/provider
    And npm test passes for all existing tests

