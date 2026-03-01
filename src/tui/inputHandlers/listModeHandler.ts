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
import { isOAuthProvider } from '../../utils/provider-config';

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
  currentProvider: ProviderDisplayInfo | undefined,
  currentProfile: ProfileDisplayInfo | undefined
): void {
  // Enter: expand provider or edit profile
  if (key.return) {
    if (currentItem.type === 'provider') {
      providerSettings.toggleProviderExpansion(currentItem.providerId);
    } else if (currentItem.type === 'oauth-login') {
      if (currentItem.method === 'browser') {
        providerSettings.startBrowserLogin(currentItem.providerId);
      } else if (currentItem.method === 'headless') {
        providerSettings.startDeviceLogin(currentItem.providerId);
      }
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

  // 'e' to edit API key (provider) or profile
  if (input === 'e' || input === 'E') {
    if (currentItem.type === 'provider') {
      if (isOAuthProvider(currentItem.providerId)) {
        // OAuth providers use OAuth flow, not API keys
        providerSettings.startBrowserLogin(currentItem.providerId);
      } else {
        providerSettings.setEditingApiKey('');
        providerSettings.setMode({
          type: 'edit-api-key',
          providerId: currentItem.providerId,
        });
      }
    } else if (currentItem.type === 'profile' && currentProfile) {
      initializeEditProfile(
        providerSettings,
        currentItem.providerId,
        currentItem.profileName,
        currentProfile.config
      );
    }
    return;
  }

  // 'n' to create new profile
  if (input === 'n' || input === 'N') {
    initializeNewProfile(providerSettings, currentItem.providerId);
    return;
  }

  // 'd' to delete
  if (input === 'd' || input === 'D') {
    if (currentItem.type === 'profile') {
      providerSettings.setMode({
        type: 'delete-profile',
        providerId: currentItem.providerId,
        profileName: currentItem.profileName,
      });
    } else if (
      currentItem.type === 'provider' &&
      currentProvider?.hasOAuthTokens &&
      isOAuthProvider(currentItem.providerId)
    ) {
      // OAuth provider — disconnect OAuth tokens instead of removing API key
      void providerSettings.disconnectOauth(currentItem.providerId);
    } else if (
      currentItem.type === 'provider' &&
      currentProvider?.status.hasKey
    ) {
      void providerSettings.removeApiKey(currentItem.providerId);
    }
    return;
  }

  // 't' to test connection
  if (input === 't' || input === 'T') {
    if (currentItem.type === 'provider') {
      void providerSettings
        .testConnection(currentItem.providerId)
        .then(providerSettings.setTestResult);
    } else if (currentItem.type === 'profile') {
      void providerSettings
        .testConnection(currentItem.providerId, currentItem.profileName)
        .then(providerSettings.setTestResult);
    }
    return;
  }

  // 'r' to refresh
  if (input === 'r' || input === 'R') {
    void providerSettings.reload();
  }
}
