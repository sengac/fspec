/**
 * ProviderSettingsScreen - Orchestrator for provider settings
 *
 * TUI-074: Extracts provider settings from AgentView.tsx.
 * Composes useProviderSettingsState (state) + useProviderSettingsInput (keyboard) + ProviderSettingsPanel (UI).
 *
 * Feature: spec/features/provider-settings-screen.feature
 */

import React, { useEffect, useRef } from 'react';
import { ProviderSettingsPanel } from './ProviderSettingsPanel';
import { useProviderSettingsState } from '../hooks/useProviderSettingsState';
import { useProviderSettingsInput } from '../hooks/useProviderSettingsInput';
import { mapToEffectivePanelMode } from '../utils/providerSettingsModeMapper';
import { SETTINGS_PANEL_CHROME_HEIGHT } from '../constants/providerSettings';
import { startCopilotLogin } from '../utils/copilotLoginFlow';

export interface ProviderSettingsScreenProps {
  /** Terminal width for layout */
  width: number;
  /** Terminal height for layout */
  height: number;
  /** Called when screen should close */
  onClose: () => void;
  /** Called to switch to model selector */
  onSwitchToModels: () => void;
  /**
   * PROV-057: When true, the screen auto-dispatches `startCopilotLogin` once
   * on mount so the user can complete the GitHub Copilot OAuth device flow
   * after picking a github-copilot model with no credentials.
   */
  autoStartCopilotLogin?: boolean;
  /**
   * PROV-057: Called once after the auto-start has been dispatched so the
   * parent can clear its trigger state and avoid re-dispatching on every
   * re-render.
   */
  onAutoStartCopilotLoginConsumed?: () => void;
}

export function ProviderSettingsScreen({
  width,
  height,
  onClose,
  onSwitchToModels,
  autoStartCopilotLogin = false,
  onAutoStartCopilotLoginConsumed,
}: ProviderSettingsScreenProps): React.ReactElement {
  // State management hook
  const providerSettings = useProviderSettingsState();

  // Calculate visible height (account for header/footer)
  const visibleHeight = height - SETTINGS_PANEL_CHROME_HEIGHT;

  // Keyboard input handling hook
  useProviderSettingsInput({
    providerSettings,
    visibleHeight,
    onClose,
    onSwitchToModels,
  });

  // PROV-057: Auto-start the Copilot login flow on mount when the parent
  // requested it. We track dispatch in a ref so the effect runs at most once
  // even if the prop bounces back to true on a future re-render.
  const hasAutoDispatchedRef = useRef(false);
  useEffect(() => {
    if (
      autoStartCopilotLogin &&
      !hasAutoDispatchedRef.current
    ) {
      hasAutoDispatchedRef.current = true;
      startCopilotLogin(providerSettings, 'github-copilot');
      onAutoStartCopilotLoginConsumed?.();
    }
  }, [
    autoStartCopilotLogin,
    providerSettings,
    onAutoStartCopilotLoginConsumed,
  ]);

  // Map hook mode to panel mode for rendering
  const effectiveMode = mapToEffectivePanelMode(providerSettings);

  // Render the presentation component
  return (
    <ProviderSettingsPanel
      width={width}
      height={height}
      providers={providerSettings.providers}
      navItems={providerSettings.navItems}
      selectedIndex={providerSettings.selectedIndex}
      scrollOffset={providerSettings.scrollOffset}
      visibleHeight={visibleHeight}
      mode={effectiveMode}
      filter={providerSettings.filter}
      isFilterMode={providerSettings.isFilterMode}
      testResult={providerSettings.testResult}
    />
  );
}
