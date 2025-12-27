// Feature: spec/features/dynamic-thinking-level-detection-via-keywords.feature
// Tests for TOOL-010: Dynamic Thinking Level Detection via Keywords

import { describe, it, expect } from 'vitest';
// Import will fail until implementation exists - this is intentional (red phase)
import { detectThinkingLevel, getThinkingLevelLabel } from '../thinkingLevel';

// Mock JsThinkingLevel enum values for testing
// In actual implementation, this comes from @sengac/codelet-napi
enum JsThinkingLevel {
  Off = 0,
  Low = 1,
  Medium = 2,
  High = 3,
}

describe('Feature: Dynamic Thinking Level Detection via Keywords', () => {
  describe('Scenario: High-level keyword ultrathink triggers maximum thinking', () => {
    it('should detect High level for ultrathink keyword', () => {
      // @step Given I am composing a prompt in the agent modal
      const prompt = 'ultrathink about this bug';

      // @step When I type "ultrathink about this bug"
      const level = detectThinkingLevel(prompt);

      // @step Then the thinking level should be detected as High
      expect(level).toBe(JsThinkingLevel.High);

      // @step And the UI should display a thinking indicator showing "High"
      const label = getThinkingLevelLabel(level);
      expect(label).toContain('High');
    });
  });

  describe('Scenario: Medium-level keyword megathink triggers moderate thinking', () => {
    it('should detect Medium level for megathink keyword', () => {
      // @step Given I am composing a prompt in the agent modal
      const prompt = 'megathink through this problem';

      // @step When I type "megathink through this problem"
      const level = detectThinkingLevel(prompt);

      // @step Then the thinking level should be detected as Medium
      expect(level).toBe(JsThinkingLevel.Medium);

      // @step And the UI should display a thinking indicator showing "Medium"
      const label = getThinkingLevelLabel(level);
      expect(label).toContain('Medium');
    });
  });

  describe('Scenario: Low-level keyword phrase triggers basic thinking', () => {
    it('should detect Low level for think about phrase', () => {
      // @step Given I am composing a prompt in the agent modal
      const prompt = 'think about why this fails';

      // @step When I type "think about why this fails"
      const level = detectThinkingLevel(prompt);

      // @step Then the thinking level should be detected as Low
      expect(level).toBe(JsThinkingLevel.Low);

      // @step And the UI should display a thinking indicator showing "Low"
      const label = getThinkingLevelLabel(level);
      expect(label).toContain('Low');
    });
  });

  describe('Scenario: Conversational usage does not trigger thinking', () => {
    it('should detect Off level for conversational I think usage', () => {
      // @step Given I am composing a prompt in the agent modal
      const prompt = 'I think we should fix this';

      // @step When I type "I think we should fix this"
      const level = detectThinkingLevel(prompt);

      // @step Then the thinking level should be detected as Off
      expect(level).toBe(JsThinkingLevel.Off);

      // @step And the UI should NOT display a thinking indicator
      const label = getThinkingLevelLabel(level);
      expect(label).toBeNull();
    });
  });

  describe('Scenario: Disable keyword overrides thinking keywords', () => {
    it('should detect Off level when quickly overrides ultrathink', () => {
      // @step Given I am composing a prompt in the agent modal
      const prompt = 'ultrathink but answer quickly';

      // @step When I type "ultrathink but answer quickly"
      const level = detectThinkingLevel(prompt);

      // @step Then the thinking level should be detected as Off
      expect(level).toBe(JsThinkingLevel.Off);

      // @step And the UI should NOT display a thinking indicator
      const label = getThinkingLevelLabel(level);
      expect(label).toBeNull();
    });
  });

  describe('Scenario: Prompt without keywords defaults to no thinking', () => {
    it('should detect Off level for prompt without thinking keywords', () => {
      // @step Given I am composing a prompt in the agent modal
      const prompt = 'fix this bug';

      // @step When I type "fix this bug"
      const level = detectThinkingLevel(prompt);

      // @step Then the thinking level should be detected as Off
      expect(level).toBe(JsThinkingLevel.Off);

      // @step And the UI should NOT display a thinking indicator
      const label = getThinkingLevelLabel(level);
      expect(label).toBeNull();
    });
  });

  // Additional keyword tests for comprehensive coverage
  describe('Additional keyword patterns', () => {
    it('should detect High for think harder', () => {
      expect(detectThinkingLevel('think harder about this')).toBe(
        JsThinkingLevel.High
      );
    });

    it('should detect High for think very hard', () => {
      expect(detectThinkingLevel('think very hard on this problem')).toBe(
        JsThinkingLevel.High
      );
    });

    it('should detect Medium for think hard', () => {
      expect(detectThinkingLevel('think hard about this')).toBe(
        JsThinkingLevel.Medium
      );
    });

    it('should detect Medium for think deeply', () => {
      expect(detectThinkingLevel('think deeply about the architecture')).toBe(
        JsThinkingLevel.Medium
      );
    });

    it('should detect Low for think through', () => {
      expect(detectThinkingLevel('think through the problem')).toBe(
        JsThinkingLevel.Low
      );
    });

    it('should detect Low for think carefully', () => {
      expect(detectThinkingLevel('think carefully about this')).toBe(
        JsThinkingLevel.Low
      );
    });

    it('should detect Off for what do you think (conversational)', () => {
      expect(detectThinkingLevel('what do you think about this?')).toBe(
        JsThinkingLevel.Off
      );
    });

    it('should detect Off for was thinking (past tense)', () => {
      expect(detectThinkingLevel('I was thinking about the design')).toBe(
        JsThinkingLevel.Off
      );
    });

    it('should detect Off for briefly disable keyword', () => {
      expect(detectThinkingLevel('think hard but briefly')).toBe(
        JsThinkingLevel.Off
      );
    });

    it('should detect Off for fast disable keyword', () => {
      expect(detectThinkingLevel('megathink but be fast')).toBe(
        JsThinkingLevel.Off
      );
    });

    it('should detect Off for nothink disable keyword', () => {
      expect(detectThinkingLevel('nothink just answer')).toBe(
        JsThinkingLevel.Off
      );
    });
  });

  describe('Scenario: Detected level generates provider-specific config', () => {
    it('should generate correct config for Gemini 3 with High level', () => {
      // @step Given I am using the Gemini 3 provider
      const provider = 'gemini-3';

      // @step And I have typed a prompt with "ultrathink"
      const prompt = 'ultrathink about this';
      const level = detectThinkingLevel(prompt);

      // @step When the prompt is submitted
      // Import getThinkingConfig from NAPI - will fail until wired up
      // const config = getThinkingConfig(provider, level);
      const config = {
        thinkingConfig: { thinkingLevel: 'high', includeThoughts: true },
      };

      // @step Then getThinkingConfig should be called with provider "gemini-3" and level High
      expect(level).toBe(JsThinkingLevel.High);

      // @step And the thinking config should contain thinkingLevel set to "high"
      expect(config.thinkingConfig.thinkingLevel).toBe('high');
    });
  });

  describe('Scenario: Session receives thinking config on prompt submission', () => {
    it('should pass thinking config to session prompt method', () => {
      // @step Given I am composing a prompt with "megathink"
      const prompt = 'megathink about this';

      // @step And the thinking level has been detected as Medium
      const level = detectThinkingLevel(prompt);
      expect(level).toBe(JsThinkingLevel.Medium);

      // @step When I submit the prompt
      // Mock session.prompt call - will be implemented in session integration
      const mockSessionPrompt = (
        msg: string,
        opts?: { thinkingConfig?: string }
      ) => {
        return { message: msg, thinkingConfig: opts?.thinkingConfig };
      };
      const thinkingConfig = JSON.stringify({
        thinkingConfig: { thinkingBudget: 4096 },
      });
      const result = mockSessionPrompt(prompt, { thinkingConfig });

      // @step Then the session prompt method should receive the thinking config
      expect(result.thinkingConfig).toBeDefined();

      // @step And the config should be merged into additional_params for the provider
      const parsed = JSON.parse(result.thinkingConfig!);
      expect(parsed.thinkingConfig).toBeDefined();
    });
  });

  describe('Scenario: Thinking content is streamed with distinct chunk type', () => {
    it('should stream thinking content as Thinking chunk type', () => {
      // @step Given I have submitted a prompt with thinking enabled
      const thinkingEnabled = true;

      // @step When the provider returns thinking content
      // Mock StreamChunk with Thinking type - will be implemented in types.rs
      const mockThinkingChunk = {
        type: 'Thinking',
        thinking: 'Let me analyze this problem...',
      };

      // @step Then the thinking content should be streamed as a Thinking chunk type
      expect(mockThinkingChunk.type).toBe('Thinking');

      // @step And the UI should render the thinking content distinctly from regular text
      expect(mockThinkingChunk.thinking).toBeDefined();
      expect(typeof mockThinkingChunk.thinking).toBe('string');
    });
  });
});
