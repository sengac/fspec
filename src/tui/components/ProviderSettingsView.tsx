/**
 * ProviderSettingsView - Provider and Profile Management Screen
 *
 * PROV-007: Full-screen overlay for managing provider API keys and profiles.
 * Implements profile CRUD (create/read/update/delete) operations.
 *
 * Features:
 * - List all providers with API key status
 * - Expand providers to see profiles
 * - Create/edit/delete profiles with full config (baseUrl, apiKey, contextWindow, maxOutputTokens)
 * - Test connections to cloud providers and local servers
 * - Source indicators showing where config comes from ([env], [config], [profile])
 */

import React, { useState, useCallback, useMemo } from 'react';
import { Box, Text, useInput } from 'ink';
import { useProviderProfiles } from '../hooks/useProviderProfiles';
import type { ProfileConfig } from '../../utils/provider-config';
import type {
  SettingsViewMode,
  ConnectionTestResult,
} from '../types/provider';

/**
 * Props for ProviderSettingsView
 */
interface ProviderSettingsViewProps {
  /** Terminal width */
  width: number;
  /** Terminal height */
  height: number;
  /** Called when view should close */
  onClose: () => void;
  /** Called to switch to model selector */
  onSwitchToModels: () => void;
}

/**
 * Navigation item type for flat list
 */
type NavItem =
  | { type: 'provider'; providerId: string; providerName: string }
  | { type: 'profile'; providerId: string; profileName: string }
  | { type: 'add-profile'; providerId: string };

/**
 * Profile form fields configuration
 */
const FORM_FIELDS: Array<{
  key: keyof ProfileConfig;
  label: string;
  type: 'text' | 'number' | 'password';
  required: boolean;
  placeholder: string;
}> = [
  {
    key: 'baseUrl',
    label: 'Base URL',
    type: 'text',
    required: true,
    placeholder: 'http://localhost:8888',
  },
  {
    key: 'apiKey',
    label: 'API Key',
    type: 'password',
    required: true,
    placeholder: 'Enter API key',
  },
  {
    key: 'contextWindow',
    label: 'Context Window',
    type: 'number',
    required: false,
    placeholder: '128000',
  },
  {
    key: 'maxOutputTokens',
    label: 'Max Output Tokens',
    type: 'number',
    required: false,
    placeholder: '16384',
  },
];

/**
 * ProviderSettingsView Component
 */
export function ProviderSettingsView({
  width,
  height,
  onClose,
  onSwitchToModels,
}: ProviderSettingsViewProps): React.ReactElement {
  const profiles = useProviderProfiles();

  // View mode state
  const [mode, setMode] = useState<SettingsViewMode>({ type: 'list' });

  // Navigation state
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [scrollOffset, setScrollOffset] = useState(0);

  // Filter state
  const [filter, setFilter] = useState('');
  const [isFilterMode, setIsFilterMode] = useState(false);

  // Form state for create/edit profile
  const [formValues, setFormValues] = useState<Partial<ProfileConfig>>({});
  const [formFieldIndex, setFormFieldIndex] = useState(0);
  const [profileName, setProfileName] = useState('');
  const [isEditingName, setIsEditingName] = useState(false);

  // API key editing state
  const [editingApiKey, setEditingApiKey] = useState('');

  // Connection test result
  const [testResult, setTestResult] = useState<ConnectionTestResult | null>(
    null
  );

  // Delete confirmation state
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

  // Calculate visible height
  const visibleHeight = height - 6; // Header + footer + padding

  // Build flat navigation list
  const navItems = useMemo((): NavItem[] => {
    const items: NavItem[] = [];
    const filterLower = filter.toLowerCase();

    for (const provider of profiles.state.providers) {
      // Filter check
      if (
        filter &&
        !provider.name.toLowerCase().includes(filterLower) &&
        !provider.id.toLowerCase().includes(filterLower)
      ) {
        continue;
      }

      // Add provider
      items.push({
        type: 'provider',
        providerId: provider.id,
        providerName: provider.name,
      });

      // Add profiles if expanded
      if (provider.isExpanded) {
        for (const profile of provider.profiles) {
          items.push({
            type: 'profile',
            providerId: provider.id,
            profileName: profile.name,
          });
        }
        // Add "Create Profile" option
        items.push({
          type: 'add-profile',
          providerId: provider.id,
        });
      }
    }

    return items;
  }, [profiles.state.providers, filter]);

  // Get current item
  const currentItem = navItems[selectedIndex];

  // Get provider for current item
  const getCurrentProvider = useCallback(() => {
    if (!currentItem) {
      return null;
    }
    return profiles.state.providers.find(p => p.id === currentItem.providerId);
  }, [currentItem, profiles.state.providers]);

  // Get profile for current item
  const getCurrentProfile = useCallback(() => {
    if (!currentItem || currentItem.type !== 'profile') {
      return null;
    }
    const provider = getCurrentProvider();
    return provider?.profiles.find(p => p.name === currentItem.profileName);
  }, [currentItem, getCurrentProvider]);

  // Handle scroll adjustment
  const adjustScroll = useCallback(
    (newIndex: number) => {
      if (newIndex < scrollOffset) {
        setScrollOffset(newIndex);
      } else if (newIndex >= scrollOffset + visibleHeight) {
        setScrollOffset(newIndex - visibleHeight + 1);
      }
    },
    [scrollOffset, visibleHeight]
  );

  // Start editing profile
  const startEditProfile = useCallback(() => {
    if (currentItem?.type !== 'profile') {
      return;
    }
    const profile = getCurrentProfile();
    if (!profile) {
      return;
    }

    setFormValues({ ...profile.config });
    setProfileName(currentItem.profileName);
    setFormFieldIndex(0);
    setIsEditingName(false);
    setMode({
      type: 'edit-profile',
      providerId: currentItem.providerId,
      profileName: currentItem.profileName,
    });
  }, [currentItem, getCurrentProfile]);

  // Start creating profile
  const startCreateProfile = useCallback(() => {
    if (!currentItem) {
      return;
    }

    setFormValues({
      baseUrl: 'http://localhost:8888',
      apiKey: '',
    });
    setProfileName('');
    setFormFieldIndex(0);
    setIsEditingName(true);
    setMode({ type: 'create-profile', providerId: currentItem.providerId });
  }, [currentItem]);

  // Save profile
  const saveProfileForm = useCallback(async () => {
    if (mode.type !== 'create-profile' && mode.type !== 'edit-profile') {
      return;
    }

    // Validate required fields
    if (!formValues.baseUrl || !formValues.apiKey || !profileName.trim()) {
      return;
    }

    const config: ProfileConfig = {
      baseUrl: formValues.baseUrl,
      apiKey: formValues.apiKey,
      ...(formValues.contextWindow && {
        contextWindow: formValues.contextWindow,
      }),
      ...(formValues.maxOutputTokens && {
        maxOutputTokens: formValues.maxOutputTokens,
      }),
    };

    if (mode.type === 'create-profile') {
      await profiles.createProfile(mode.providerId, profileName.trim(), config);
    } else {
      await profiles.updateProfile(
        mode.providerId,
        mode.profileName,
        config
      );
    }

    setMode({ type: 'list' });
  }, [mode, formValues, profileName, profiles]);

  // Delete profile
  const deleteCurrentProfile = useCallback(async () => {
    if (mode.type !== 'delete-profile') {
      return;
    }

    await profiles.removeProfile(mode.providerId, mode.profileName);
    setMode({ type: 'list' });
    setShowDeleteConfirm(false);
  }, [mode, profiles]);

  // Handle input
  useInput(
    (input, key) => {
      // Filter mode handling
      if (isFilterMode) {
        if (key.escape) {
          setIsFilterMode(false);
          setFilter('');
          return;
        }
        if (key.return) {
          setIsFilterMode(false);
          return;
        }
        if (key.backspace || key.delete) {
          setFilter(prev => prev.slice(0, -1));
          return;
        }
        // Accept printable characters
        const clean = input
          .split('')
          .filter(ch => {
            const code = ch.charCodeAt(0);
            return code >= 32 && code <= 126;
          })
          .join('');
        if (clean) {
          setFilter(prev => prev + clean);
        }
        return;
      }

      // Delete confirmation
      if (showDeleteConfirm) {
        if (input === 'y' || input === 'Y') {
          void deleteCurrentProfile();
          return;
        }
        if (key.escape || input === 'n' || input === 'N') {
          setShowDeleteConfirm(false);
          setMode({ type: 'list' });
          return;
        }
        return;
      }

      // API key editing mode
      if (mode.type === 'edit-api-key') {
        if (key.escape) {
          setMode({ type: 'list' });
          setEditingApiKey('');
          return;
        }
        if (key.return) {
          if (editingApiKey.trim()) {
            void profiles.saveApiKey(mode.providerId, editingApiKey.trim());
          }
          setMode({ type: 'list' });
          setEditingApiKey('');
          return;
        }
        if (key.backspace || key.delete) {
          setEditingApiKey(prev => prev.slice(0, -1));
          return;
        }
        const clean = input
          .split('')
          .filter(ch => {
            const code = ch.charCodeAt(0);
            return code >= 32 && code <= 126;
          })
          .join('');
        if (clean) {
          setEditingApiKey(prev => prev + clean);
        }
        return;
      }

      // Profile form mode
      if (mode.type === 'create-profile' || mode.type === 'edit-profile') {
        if (key.escape) {
          setMode({ type: 'list' });
          return;
        }

        // Tab to move between fields
        if (key.tab) {
          if (isEditingName) {
            setIsEditingName(false);
            setFormFieldIndex(0);
          } else if (key.shift) {
            if (formFieldIndex > 0) {
              setFormFieldIndex(prev => prev - 1);
            } else if (mode.type === 'create-profile') {
              setIsEditingName(true);
            }
          } else {
            if (formFieldIndex < FORM_FIELDS.length - 1) {
              setFormFieldIndex(prev => prev + 1);
            }
          }
          return;
        }

        // Enter to save
        if (key.return && !key.shift) {
          void saveProfileForm();
          return;
        }

        // Handle text input
        if (key.backspace || key.delete) {
          if (isEditingName) {
            setProfileName(prev => prev.slice(0, -1));
          } else {
            const field = FORM_FIELDS[formFieldIndex];
            setFormValues(prev => {
              const current = String(prev[field.key] || '');
              return { ...prev, [field.key]: current.slice(0, -1) };
            });
          }
          return;
        }

        const clean = input
          .split('')
          .filter(ch => {
            const code = ch.charCodeAt(0);
            return code >= 32 && code <= 126;
          })
          .join('');

        if (clean) {
          if (isEditingName) {
            setProfileName(prev => prev + clean);
          } else {
            const field = FORM_FIELDS[formFieldIndex];
            setFormValues(prev => {
              const current = String(prev[field.key] || '');
              const newValue = current + clean;
              // Convert to number if needed
              if (field.type === 'number') {
                const num = parseInt(newValue, 10);
                return { ...prev, [field.key]: isNaN(num) ? undefined : num };
              }
              return { ...prev, [field.key]: newValue };
            });
          }
        }
        return;
      }

      // List mode
      if (mode.type === 'list') {
        // Close on Escape
        if (key.escape) {
          if (filter) {
            setFilter('');
            return;
          }
          onClose();
          return;
        }

        // Tab to switch to models
        if (key.tab) {
          onSwitchToModels();
          return;
        }

        // Filter mode
        if (input === '/') {
          setIsFilterMode(true);
          return;
        }

        // Navigation
        if (key.upArrow && selectedIndex > 0) {
          const newIndex = selectedIndex - 1;
          setSelectedIndex(newIndex);
          adjustScroll(newIndex);
          setTestResult(null);
          return;
        }
        if (key.downArrow && selectedIndex < navItems.length - 1) {
          const newIndex = selectedIndex + 1;
          setSelectedIndex(newIndex);
          adjustScroll(newIndex);
          setTestResult(null);
          return;
        }

        // Enter to expand/edit
        if (key.return && currentItem) {
          if (currentItem.type === 'provider') {
            profiles.toggleProviderExpansion(currentItem.providerId);
          } else if (currentItem.type === 'profile') {
            startEditProfile();
          } else if (currentItem.type === 'add-profile') {
            startCreateProfile();
          }
          return;
        }

        // 'e' to edit API key (provider) or profile
        if ((input === 'e' || input === 'E') && currentItem) {
          if (currentItem.type === 'provider') {
            setEditingApiKey('');
            setMode({ type: 'edit-api-key', providerId: currentItem.providerId });
          } else if (currentItem.type === 'profile') {
            startEditProfile();
          }
          return;
        }

        // 'n' to create new profile
        if ((input === 'n' || input === 'N') && currentItem) {
          startCreateProfile();
          return;
        }

        // 'd' to delete
        if ((input === 'd' || input === 'D') && currentItem) {
          if (currentItem.type === 'profile') {
            setMode({
              type: 'delete-profile',
              providerId: currentItem.providerId,
              profileName: currentItem.profileName,
            });
            setShowDeleteConfirm(true);
          } else if (currentItem.type === 'provider') {
            const provider = getCurrentProvider();
            if (provider?.status.hasKey) {
              void profiles.removeApiKey(currentItem.providerId);
            }
          }
          return;
        }

        // 't' to test connection
        if ((input === 't' || input === 'T') && currentItem) {
          if (currentItem.type === 'provider') {
            void profiles
              .testConnection(currentItem.providerId)
              .then(setTestResult);
          } else if (currentItem.type === 'profile') {
            void profiles
              .testConnection(currentItem.providerId, currentItem.profileName)
              .then(setTestResult);
          }
          return;
        }

        // 'r' to refresh
        if (input === 'r' || input === 'R') {
          void profiles.reload();
          return;
        }
      }
    },
    { isActive: true }
  );

  // Calculate content width
  const contentWidth = width - 4;

  // Render loading state
  if (profiles.state.isLoading) {
    return (
      <Box
        flexDirection="column"
        width={width}
        height={height}
        backgroundColor="black"
      >
        <Box padding={2}>
          <Text color="yellow">Loading providers...</Text>
        </Box>
      </Box>
    );
  }

  // Render delete confirmation
  if (showDeleteConfirm && mode.type === 'delete-profile') {
    return (
      <Box
        flexDirection="column"
        width={width}
        height={height}
        backgroundColor="black"
      >
        <Box flexDirection="column" padding={2}>
          <Text bold color="red">
            Delete Profile
          </Text>
          <Box marginTop={1}>
            <Text>
              Are you sure you want to delete profile "{mode.profileName}"?
            </Text>
          </Box>
          <Box marginTop={1}>
            <Text dimColor>Press 'y' to confirm, 'n' or Esc to cancel</Text>
          </Box>
        </Box>
      </Box>
    );
  }

  // Render API key edit mode
  if (mode.type === 'edit-api-key') {
    const provider = profiles.state.providers.find(
      p => p.id === mode.providerId
    );
    return (
      <Box
        flexDirection="column"
        width={width}
        height={height}
        backgroundColor="black"
      >
        <Box flexDirection="column" padding={2}>
          <Text bold color="yellow">
            Edit API Key: {provider?.name || mode.providerId}
          </Text>
          <Box marginTop={1}>
            <Text color="cyan">API Key: </Text>
            <Text>
              {editingApiKey ? '•'.repeat(editingApiKey.length) : ''}
              <Text inverse> </Text>
            </Text>
          </Box>
          <Box marginTop={1}>
            <Text dimColor>Enter to save | Esc to cancel</Text>
          </Box>
        </Box>
      </Box>
    );
  }

  // Render profile form mode
  if (mode.type === 'create-profile' || mode.type === 'edit-profile') {
    const isCreate = mode.type === 'create-profile';
    const provider = profiles.state.providers.find(
      p => p.id === mode.providerId
    );

    return (
      <Box
        flexDirection="column"
        width={width}
        height={height}
        backgroundColor="black"
      >
        <Box flexDirection="column" padding={2}>
          <Text bold color="yellow">
            {isCreate ? 'Create Profile' : 'Edit Profile'}:{' '}
            {provider?.name || mode.providerId}
          </Text>

          {/* Profile name (only editable for create) */}
          <Box marginTop={1}>
            <Text
              color={isEditingName ? 'cyan' : 'white'}
              backgroundColor={isEditingName ? 'blue' : undefined}
            >
              Profile Name:{' '}
            </Text>
            <Text>
              {profileName}
              {isEditingName && <Text inverse> </Text>}
            </Text>
          </Box>

          {/* Form fields */}
          {FORM_FIELDS.map((field, idx) => {
            const isActive = !isEditingName && idx === formFieldIndex;
            const value = formValues[field.key];
            const displayValue =
              field.type === 'password' && value
                ? '•'.repeat(String(value).length)
                : String(value || '');

            return (
              <Box key={field.key} marginTop={idx === 0 ? 1 : 0}>
                <Text
                  color={isActive ? 'cyan' : 'white'}
                  backgroundColor={isActive ? 'blue' : undefined}
                >
                  {field.label}:{' '}
                </Text>
                <Text>
                  {displayValue || (
                    <Text dimColor>{field.placeholder}</Text>
                  )}
                  {isActive && <Text inverse> </Text>}
                </Text>
                {field.required && !value && (
                  <Text color="red"> *</Text>
                )}
              </Box>
            );
          })}

          <Box marginTop={2}>
            <Text dimColor>
              Tab: next field | Shift+Tab: prev | Enter: save | Esc: cancel
            </Text>
          </Box>
        </Box>
      </Box>
    );
  }

  // Render list mode
  return (
    <Box
      flexDirection="column"
      width={width}
      height={height}
      backgroundColor="black"
    >
      <Box flexDirection="column" padding={2} flexGrow={1}>
        {/* Header */}
        <Box marginBottom={1}>
          <Text bold color="yellow">
            Provider Settings
          </Text>
          <Text dimColor>
            {' '}
            ({navItems.length} items)
          </Text>
        </Box>

        {/* Filter */}
        {(isFilterMode || filter) && (
          <Box marginBottom={1}>
            <Text color="yellow">Filter: </Text>
            <Text>{filter}</Text>
            {isFilterMode && <Text inverse> </Text>}
          </Box>
        )}

        {/* List */}
        <Box flexDirection="column" flexGrow={1}>
          {navItems
            .slice(scrollOffset, scrollOffset + visibleHeight)
            .map((item, visibleIdx) => {
              const actualIdx = scrollOffset + visibleIdx;
              const isSelected = actualIdx === selectedIndex;
              const provider = profiles.state.providers.find(
                p => p.id === item.providerId
              );

              if (item.type === 'provider') {
                const status = provider?.status;
                const isExpanded = provider?.isExpanded;
                const profileCount = provider?.profiles.length || 0;

                return (
                  <Box key={`provider-${item.providerId}`} width={contentWidth}>
                    <Text
                      backgroundColor={isSelected ? 'yellow' : undefined}
                      color={isSelected ? 'black' : 'white'}
                      wrap="truncate"
                    >
                      {isSelected ? '> ' : '  '}
                      {isExpanded ? '▼ ' : '▶ '}
                      {item.providerName}
                      {status?.hasKey ? (
                        <Text color={isSelected ? 'black' : 'green'}>
                          {' '}
                          ✓ {status.maskedKey}
                          {status.source && (
                            <Text dimColor={!isSelected}>
                              {' '}
                              [{status.source}]
                            </Text>
                          )}
                        </Text>
                      ) : (
                        <Text color={isSelected ? 'black' : 'gray'}>
                          {' '}
                          (not configured)
                        </Text>
                      )}
                      {profileCount > 0 && (
                        <Text dimColor={!isSelected}>
                          {' '}
                          ({profileCount} profile{profileCount !== 1 ? 's' : ''})
                        </Text>
                      )}
                      {testResult?.providerId === item.providerId &&
                        !testResult.profileName && (
                          <Text
                            color={
                              isSelected
                                ? 'black'
                                : testResult.success
                                  ? 'green'
                                  : 'red'
                            }
                          >
                            {' '}
                            {testResult.message}
                          </Text>
                        )}
                    </Text>
                  </Box>
                );
              }

              if (item.type === 'profile') {
                const profile = provider?.profiles.find(
                  p => p.name === item.profileName
                );

                return (
                  <Box
                    key={`profile-${item.providerId}-${item.profileName}`}
                    width={contentWidth}
                  >
                    <Text
                      backgroundColor={isSelected ? 'cyan' : undefined}
                      color={isSelected ? 'black' : 'cyan'}
                      wrap="truncate"
                    >
                      {isSelected ? '> ' : '  '}
                      {'    '}📁 {item.profileName}
                      {profile?.config.baseUrl && (
                        <Text dimColor={!isSelected}>
                          {' '}
                          → {profile.config.baseUrl}
                        </Text>
                      )}
                      {testResult?.providerId === item.providerId &&
                        testResult.profileName === item.profileName && (
                          <Text
                            color={
                              isSelected
                                ? 'black'
                                : testResult.success
                                  ? 'green'
                                  : 'red'
                            }
                          >
                            {' '}
                            {testResult.message}
                          </Text>
                        )}
                    </Text>
                  </Box>
                );
              }

              // add-profile
              return (
                <Box
                  key={`add-profile-${item.providerId}`}
                  width={contentWidth}
                >
                  <Text
                    backgroundColor={isSelected ? 'green' : undefined}
                    color={isSelected ? 'black' : 'green'}
                    wrap="truncate"
                  >
                    {isSelected ? '> ' : '  '}
                    {'    '}+ Create new profile
                  </Text>
                </Box>
              );
            })}
        </Box>

        {/* Footer */}
        <Box marginTop={1}>
          <Text dimColor>
            Enter: expand/edit | e: edit | n: new profile | d: delete | t: test
            | Tab: models | / filter | Esc: close
          </Text>
        </Box>
      </Box>
    </Box>
  );
}
