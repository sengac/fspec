/**
 * Test Constants for Model Selector Tests
 *
 * Centralized constants for model selector integration tests.
 * Single source of truth for model IDs, provider names, etc.
 */

// Re-export from centralized NAPI model fixtures
export {
  TEST_MODEL_IDS,
  TEST_PROVIDER_IDS,
  TEST_PROVIDER_NAMES,
} from '../../../../test-helpers/napi-model-fixtures';

/**
 * UI indicators used in model selector
 */
export const UI_INDICATORS = {
  /** Expanded section indicator */
  expanded: '▼',
  /** Collapsed section indicator */
  collapsed: '▶',
  /** Selection indicator (at start of line) */
  selected: '>',
} as const;

/**
 * Regex patterns for asserting UI state
 */
export const UI_PATTERNS = {
  /** Match selected + expanded Anthropic section */
  selectedExpandedAnthropic: />\s*▼\s*Anthropic/,
  /** Match selected + collapsed Anthropic section */
  selectedCollapsedAnthropic: />\s*▶\s*Anthropic/,
  /** Match expanded Anthropic section (not necessarily selected) */
  expandedAnthropic: /▼\s*Anthropic/,
  /** Match collapsed Anthropic section (not necessarily selected) */
  collapsedAnthropic: /▶\s*Anthropic/,

  /** Match selected + expanded OpenAI section */
  selectedExpandedOpenai: />\s*▼\s*OpenAI/,
  /** Match selected + collapsed OpenAI section */
  selectedCollapsedOpenai: />\s*▶\s*OpenAI/,
  /** Match expanded OpenAI section (not necessarily selected) */
  expandedOpenai: /▼\s*OpenAI/,
  /** Match collapsed OpenAI section (not necessarily selected) */
  collapsedOpenai: /▶\s*OpenAI/,

  /** Match selected model item (model name follows >) */
  selectedModel: (modelName: string) => new RegExp(`>\\s*${modelName}`),
} as const;

/**
 * Timing constants for tests
 */
export const TEST_TIMING = {
  /** Wait after models load */
  afterModelsLoad: 100,
  /** Wait after key press */
  afterKeyPress: 50,
  /** Wait after typing text */
  afterTyping: 100,
  /** Wait for async state updates */
  asyncUpdate: 150,
} as const;
