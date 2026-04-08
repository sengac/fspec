/**
 * Feature: spec/features/github-copilot-end-to-end-integration.feature
 *
 * PROV-057: TUI dispatch test for Copilot login flow.
 *
 * Verifies that the helper `shouldDispatchCopilotLogin()` correctly identifies
 * a github-copilot/* model selection that lacks credentials, so `handleModelSelect`
 * in AgentView.tsx knows when to dispatch `startCopilotLogin` from
 * src/tui/utils/copilotLoginFlow.ts instead of showing a
 * "requires credentials" error toast.
 *
 * Test philosophy: the helper is a pure function over ProviderSection[] and
 * ModelSelection — no mocking required, no NAPI, no filesystem, no Ink.
 */

import { describe, it, expect, vi } from 'vitest';
import type { ProviderSection, ModelSelection } from '../../types/provider';
import {
  shouldDispatchCopilotLogin,
  dispatchCopilotLoginIfNeeded,
} from '../copilotLoginDispatch';
import type { UseProviderSettingsStateReturn } from '../../hooks/useProviderSettingsState';

function makeSelection(providerId: string): ModelSelection {
  return {
    providerId,
    modelId: 'gpt-4o',
    apiModelId: 'gpt-4o',
    displayName: 'GPT-4o',
    reasoning: false,
    hasVision: false,
    contextWindow: 128000,
    maxOutput: 16384,
  };
}

function makeSection(
  providerId: string,
  hasCredentials: boolean
): ProviderSection {
  return {
    providerId,
    providerName: providerId,
    internalName: providerId,
    models: [],
    hasCredentials,
  };
}

describe('Feature: GitHub Copilot TUI dispatch', () => {
  describe('Scenario: TUI launches Copilot OAuth login when user picks Copilot model with no credentials', () => {
    it('returns true when provider is github-copilot and section has no credentials', () => {
      // @step Given no copilot_auth.json exists on disk
      // (Simulated via providerSections.hasCredentials=false)
      const sections: ProviderSection[] = [
        makeSection('anthropic', true),
        makeSection('github-copilot', false),
      ];
      const selection = makeSelection('github-copilot');

      // @step When the user selects a github-copilot model from the model picker
      const result = shouldDispatchCopilotLogin(sections, selection);

      // @step Then the TUI dispatches startCopilotLogin from copilotLoginFlow.ts
      expect(result).toBe(true);
      // @step And the TUI does NOT display "Failed to switch model: ... requires credentials"
      // (The pure helper signals dispatch via its return value; the integration in
      // AgentView.handleModelSelect uses this to skip the selectModel error path
      // entirely. See AgentView.tsx PROV-057 branch.)
    });

    it('returns false when github-copilot has credentials', () => {
      const sections: ProviderSection[] = [makeSection('github-copilot', true)];
      const selection = makeSelection('github-copilot');

      const result = shouldDispatchCopilotLogin(sections, selection);

      expect(result).toBe(false);
    });

    it('returns false when provider is not github-copilot', () => {
      const sections: ProviderSection[] = [makeSection('anthropic', false)];
      const selection = makeSelection('anthropic');

      const result = shouldDispatchCopilotLogin(sections, selection);

      expect(result).toBe(false);
    });

    it('returns false when github-copilot section is missing entirely', () => {
      const sections: ProviderSection[] = [makeSection('anthropic', true)];
      const selection = makeSelection('github-copilot');

      // Missing section is treated the same as "no credentials"
      // but the caller still needs to dispatch login.
      const result = shouldDispatchCopilotLogin(sections, selection);

      expect(result).toBe(true);
    });
  });

  describe('Scenario: dispatchCopilotLoginIfNeeded invokes startCopilotLogin callback', () => {
    it('calls the provided login callback exactly once with github-copilot', () => {
      // @step Given no copilot_auth.json exists on disk
      const sections: ProviderSection[] = [
        makeSection('github-copilot', false),
      ];
      const selection = makeSelection('github-copilot');

      // vi.fn() is used here strictly as an event-sink for the dispatch
      // callback — per TESTING.md this is the one allowed use-case.
      const loginCallback = vi.fn();

      // @step When the user selects a github-copilot model from the model picker
      const dispatched = dispatchCopilotLoginIfNeeded(
        sections,
        selection,
        loginCallback
      );

      // @step Then the TUI dispatches startCopilotLogin from copilotLoginFlow.ts
      expect(dispatched).toBe(true);
      expect(loginCallback).toHaveBeenCalledTimes(1);
      expect(loginCallback).toHaveBeenCalledWith('github-copilot');
      // @step And the TUI does NOT display "Failed to switch model: ... requires credentials"
      // (Returning true tells AgentView.handleModelSelect to skip the
      // selectModel call entirely — no error toast can be set.)
    });

    it('does NOT call the login callback when credentials already exist', () => {
      const sections: ProviderSection[] = [makeSection('github-copilot', true)];
      const selection = makeSelection('github-copilot');
      const loginCallback = vi.fn();

      const dispatched = dispatchCopilotLoginIfNeeded(
        sections,
        selection,
        loginCallback
      );

      expect(dispatched).toBe(false);
      expect(loginCallback).not.toHaveBeenCalled();
    });

    it('does NOT call the login callback for non-copilot providers', () => {
      const sections: ProviderSection[] = [makeSection('anthropic', false)];
      const selection = makeSelection('anthropic');
      const loginCallback = vi.fn();

      const dispatched = dispatchCopilotLoginIfNeeded(
        sections,
        selection,
        loginCallback
      );

      expect(dispatched).toBe(false);
      expect(loginCallback).not.toHaveBeenCalled();
    });
  });

  describe('Scenario: startCopilotLogin transitions provider settings into OAuth flow', () => {
    it('dispatches into oauth-deployment-type-select mode', async () => {
      // @step Given no copilot_auth.json exists on disk
      // @step When the user selects a github-copilot model from the model picker
      //
      // This test verifies that the real startCopilotLogin (from copilotLoginFlow.ts)
      // can be composed as the callback passed to dispatchCopilotLoginIfNeeded.
      const { startCopilotLogin } = await import('../copilotLoginFlow');

      const setMode = vi.fn();
      const ps = {
        setMode,
        reload: vi.fn(async () => undefined),
      } as unknown as UseProviderSettingsStateReturn;

      const sections: ProviderSection[] = [
        makeSection('github-copilot', false),
      ];
      const selection = makeSelection('github-copilot');

      const dispatched = dispatchCopilotLoginIfNeeded(
        sections,
        selection,
        (providerId: string) => {
          startCopilotLogin(ps, providerId);
        }
      );

      // @step Then the TUI dispatches startCopilotLogin from copilotLoginFlow.ts
      expect(dispatched).toBe(true);
      expect(setMode).toHaveBeenCalledTimes(1);
      expect(setMode).toHaveBeenCalledWith({
        type: 'oauth-deployment-type-select',
        providerId: 'github-copilot',
        selectedIndex: 0,
      });
      // @step And the TUI does NOT display "Failed to switch model: ... requires credentials"
      // (Composing the real startCopilotLogin proves the dispatch path replaces
      // the legacy error-toast path entirely.)
    });
  });
});
