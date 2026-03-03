/**
 * List mode input handler
 *
 * TUI-074: Handles keyboard input in list navigation mode
 */

import type { Key } from 'ink';
import type { UseProviderSettingsStateReturn } from '../hooks/useProviderSettingsState';
import type {
  SettingsNavItem,
  ProviderDisplayInfo,
  ProfileDisplayInfo,
} from '../components/ProviderSettingsPanel';
import {
  initializeNewProfile,
  initializeEditProfile,
} from '../utils/providerSettingsHelpers';

export interface ListModeHandlerOptions {
  input: string;
  key: Key;
  providerSettings: UseProviderSettingsStateReturn;
  currentItem: SettingsNavItem | undefined;
  currentProvider: ProviderDisplayInfo | undefined;
  currentProfile: ProfileDisplayInfo | undefined;
  visibleHeight: number;
  onClose: () => void;
  onSwitchToModels: () => void;
}

/**
 * Handles input in list mode (navigation and actions)
 */
export function handleListMode({
  input,
  key,
  providerSettings,
  currentItem,
  currentProvider,
  currentProfile,
  visibleHeight,
  onClose,
  onSwitchToModels,
}: ListModeHandlerOptions): void {
  // Escape: close screen (or clear filter first)
  if (key.escape) {
    if (providerSettings.filter) {
      providerSettings.setFilter('');
      return;
    }
    onClose();
    return;
  }

  // Tab: switch to model selector
  if (key.tab) {
    onSwitchToModels();
    return;
  }

  // '/' to enter filter mode
  if (input === '/') {
    providerSettings.setIsFilterMode(true);
    return;
  }

  // Arrow navigation
  if (key.upArrow && providerSettings.selectedIndex > 0) {
    providerSettings.setSelectedIndex(providerSettings.selectedIndex - 1);
    providerSettings.setTestResult(null);
    if (providerSettings.selectedIndex - 1 < providerSettings.scrollOffset) {
      providerSettings.setScrollOffset(providerSettings.selectedIndex - 1);
    }
    return;
  }

  if (
    key.downArrow &&
    providerSettings.selectedIndex < providerSettings.navItems.length - 1
  ) {
    providerSettings.setSelectedIndex(providerSettings.selectedIndex + 1);
    providerSettings.setTestResult(null);
    if (
      providerSettings.selectedIndex + 1 >=
      providerSettings.scrollOffset + visibleHeight
    ) {
      providerSettings.setScrollOffset(
        providerSettings.selectedIndex + 1 - visibleHeight + 1
      );
    }
    return;
  }

  // Action keys require currentItem
  if (!currentItem) {
    return;
  }

  handleActions(
    input,
    key,
    providerSettings,
    currentItem,
    currentProvider,
    currentProfile
  );
}

function handleActions(
  input: string,
  key: Key,
  providerSettings: UseProviderSettingsStateReturn,
  currentItem: SettingsNavItem,
  _currentProvider: ProviderDisplayInfo | undefined,
  currentProfile: ProfileDisplayInfo | undefined
): void {
  // Enter: expand provider, start login, edit api-key, edit profile, create profile
  if (key.return) {
    if (currentItem.type === 'provider') {
      providerSettings.toggleProviderExpansion(currentItem.providerId);
    } else if (currentItem.type === 'oauth-login') {
      if (currentItem.method === 'browser') {
        providerSettings.startBrowserLogin(currentItem.providerId);
      } else if (currentItem.method === 'headless') {
        providerSettings.startDeviceLogin(currentItem.providerId);
      }
    } else if (currentItem.type === 'api-key') {
      providerSettings.setEditingApiKey('');
      providerSettings.setMode({
        type: 'edit-api-key',
        providerId: currentItem.providerId,
      });
    } else if (currentItem.type === 'profile' && currentProfile) {
      initializeEditProfile(
        providerSettings,
        currentItem.providerId,
        currentItem.profileName,
        currentProfile.config
      );
    } else if (currentItem.type === 'add-profile') {
      initializeNewProfile(providerSettings, currentItem.providerId);
    }
    return;
  }

  // 'd' to delete/disconnect with confirmation
  if (input === 'd' || input === 'D') {
    if (currentItem.type === 'api-key') {
      providerSettings.setMode({
        type: 'delete-api-key',
        providerId: currentItem.providerId,
      });
    } else if (currentItem.type === 'oauth-status') {
      providerSettings.setMode({
        type: 'disconnect-oauth',
        providerId: currentItem.providerId,
      });
    } else if (currentItem.type === 'profile') {
      providerSettings.setMode({
        type: 'delete-profile',
        providerId: currentItem.providerId,
        profileName: currentItem.profileName,
      });
    }
    return;
  }
}
