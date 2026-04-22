@tui
@context-window
@providers
@BUG-139
Feature: TUI custom provider builder sources per-model limits from NAPI
  """
  TypeScript/TUI tier of BUG-139. Refactors
  src/tui/services/customProviderSectionBuilder.ts to consume the widened
  JsProviderInfo.models entries (each carrying { id, contextWindow, maxOutput,
  supportsTools, supportsStreaming, supportsThinking }) instead of the legacy
  hardcoded fallbacks contextWindow=128000 / maxOutput=8192. This is what
  actually makes the SessionHeader badge show [1M] / [200k] / [400k] rather
  than the artefact [120k] seen in the bug screenshot.
  """

  # EXAMPLE MAPPING CONTEXT
  #
  # Rules covered by this slice:
  #   - Rule 3: customProviderSectionBuilder MUST NOT hardcode contextWindow or maxOutput
  #   - Rule 4: SessionHeader badge renders formatContextWindow(compactionThreshold ?? contextWindow)
  #   - Rule 5: Regression test with JSON context_window=1000000 / no Rhai hook / badge=[1M]
  Background: 
    Given listProviders() is mocked to return a "claude-rhai" custom provider entry

  Scenario: customProviderSectionBuilder sources contextWindow from NAPI entry
    Given listProviders() returns a "claude-rhai" entry with model { id: "opus-4.7", contextWindow: 1000000, maxOutput: 128000, supportsTools: true, supportsStreaming: true, supportsThinking: true }
    When loadCustomProviderSections() runs
    Then the resulting ProviderSection for "claude-rhai" contains one NapiModelInfo
    And that NapiModelInfo.id equals "opus-4.7"
    And that NapiModelInfo.contextWindow equals 1000000
    And that NapiModelInfo.maxOutput equals 128000
    And no value in that NapiModelInfo is the legacy hardcoded 128000 / 8192 fallback

  Scenario: 1M JSON context_window round-trips to NapiModelInfo.contextWindow
    Given listProviders() returns a "claude-rhai" entry with model { id: "opus-4.7", contextWindow: 1000000, maxOutput: 128000 }
    When loadCustomProviderSections() runs
    Then the resulting NapiModelInfo.contextWindow equals 1000000
    And the resulting NapiModelInfo.contextWindow is NOT 120000
    And the resulting NapiModelInfo.contextWindow is NOT 128000

  Scenario: 200k JSON context_window round-trips verbatim into NapiModelInfo
    Given listProviders() returns a "claude-rhai" entry with model { id: "opus-4.7", contextWindow: 200000, maxOutput: 64000 }
    When loadCustomProviderSections() runs
    Then the resulting NapiModelInfo.contextWindow equals 200000
    And the resulting NapiModelInfo.maxOutput equals 64000

  Scenario: Max output tokens from NAPI propagates verbatim (no 8192 fallback)
    Given listProviders() returns a "claude-rhai" entry with model { id: "opus-4.7", contextWindow: 200000, maxOutput: 64000 }
    When loadCustomProviderSections() runs
    Then the resulting NapiModelInfo.maxOutput equals 64000
    And the resulting NapiModelInfo.maxOutput is NOT the legacy hardcoded 8192

  Scenario: supports_* flags propagate through the builder
    Given listProviders() returns a "claude-rhai" entry with model { id: "opus-4.7", contextWindow: 200000, maxOutput: 64000, supportsTools: true, supportsStreaming: true, supportsThinking: true }
    When loadCustomProviderSections() runs
    Then the resulting NapiModelInfo.supportsTools equals true
    And the resulting NapiModelInfo.supportsStreaming equals true
    And the resulting NapiModelInfo.supportsThinking equals true
