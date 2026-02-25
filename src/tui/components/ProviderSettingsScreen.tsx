/**
 * ProviderSettingsScreen - Orchestrator for provider settings
 *
 * TUI-074: Extracts provider settings from AgentView.tsx.
 * Composes useProviderSettingsState (state) + useProviderSettingsInput (keyboard) + ProviderSettingsPanel (UI).
 *
 * Feature: spec/features/provider-settings-screen.feature
 */

import React from 'react';
import { ProviderSettingsPanel } from './ProviderSettingsPanel';
import { useProviderSettingsState } from '../hooks/useProviderSettingsState';
import { useProviderSettingsInput } from '../hooks/useProviderSettingsInput';
import { mapToEffectivePanelMode } from '../utils/providerSettingsModeMapper';
import { SETTINGS_PANEL_CHROME_HEIGHT } from '../constants/providerSettings';

export interface ProviderSettingsScreenProps {
  /** Terminal width for layout */
  width: number;
  /** Terminal height for layout */
  height: number;
  /** Called when screen should close */
  onClose: () => void;
  /** Called to switch to model selector */
  onSwitchToModels: () => void;
}

export function ProviderSettingsScreen({
  width,
  height,
  onClose,
  onSwitchToModels,
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
