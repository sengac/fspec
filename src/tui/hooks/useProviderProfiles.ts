/**
 * useProviderProfiles - Hook for provider profile management
 *
 * PROV-007: Manages profile CRUD operations and state for ProviderSettingsView.
 * Follows SOLID principles with single responsibility for profile operations.
 */

import { useState, useCallback, useEffect } from 'react';
import {
  loadProviderProfiles,
  saveProfile,
  deleteProfile,
  getProfile,
  getProviderRegistry,
  getProviderRegistryEntry,
  type ProfileConfig,
} from '../../utils/provider-config';
import {
  getProviderConfig,
  setProviderCredential,
  deleteProviderCredential,
} from '../../utils/credentials';
import {
  modelsListLocalOpenai,
  testProviderConnection,
} from '@sengac/codelet-napi';
import { logger } from '../../utils/logger';
import type {
  ProviderStatus,
  ProfileDisplay,
  ProviderWithProfiles,
  ConnectionTestResult,
} from '../types/provider';

/**
 * Profile loading state
 */
interface ProfileState {
  providers: ProviderWithProfiles[];
  isLoading: boolean;
  error: string | null;
}

/**
 * Hook return type
 */
interface UseProviderProfilesReturn {
  /** Current state */
  state: ProfileState;
  /** Reload all provider and profile data */
  reload: () => Promise<void>;
  /** Create a new profile */
  createProfile: (
    providerId: string,
    profileName: string,
    config: ProfileConfig
  ) => Promise<void>;
  /** Update an existing profile */
  updateProfile: (
    providerId: string,
    profileName: string,
    config: ProfileConfig
  ) => Promise<void>;
  /** Delete a profile */
  removeProfile: (providerId: string, profileName: string) => Promise<void>;
  /** Get a specific profile */
  getProfileConfig: (
    providerId: string,
    profileName: string
  ) => Promise<ProfileConfig | undefined>;
  /** Save API key for provider */
  saveApiKey: (providerId: string, apiKey: string) => Promise<void>;
  /** Delete API key for provider */
  removeApiKey: (providerId: string) => Promise<void>;
  /** Test connection to provider or profile */
  testConnection: (
    providerId: string,
    profileName?: string
  ) => Promise<ConnectionTestResult>;
  /** Toggle provider expansion */
  toggleProviderExpansion: (providerId: string) => void;
  /** Toggle profile expansion */
  toggleProfileExpansion: (providerId: string, profileName: string) => void;
}

/**
 * Hook for managing provider profiles
 */
export function useProviderProfiles(): UseProviderProfilesReturn {
  const [state, setState] = useState<ProfileState>({
    providers: [],
    isLoading: true,
    error: null,
  });

  /**
   * Load all providers and their profiles
   */
  const reload = useCallback(async () => {
    setState(prev => ({ ...prev, isLoading: true, error: null }));

    try {
      const providerIds = getProviderRegistry();
      const providers: ProviderWithProfiles[] = [];

      for (const providerId of providerIds) {
        const registryEntry = getProviderRegistryEntry(providerId);
        if (!registryEntry) {
          continue;
        }

        // Get provider credentials/status
        const providerConfig = await getProviderConfig(providerId);
        const status: ProviderStatus = {
          hasKey: !!providerConfig.apiKey,
          maskedKey: providerConfig.apiKey
            ? maskApiKey(providerConfig.apiKey)
            : undefined,
          source: providerConfig.source as
            | 'env'
            | 'config'
            | 'profile'
            | undefined,
        };

        // Load profiles for this provider
        const profiles = await loadProviderProfiles(providerId);
        const profileDisplays: ProfileDisplay[] = Object.entries(profiles).map(
          ([name, config]) => ({
            name,
            config,
            isExpanded: false,
          })
        );

        providers.push({
          id: providerId,
          name: registryEntry.name,
          status,
          profiles: profileDisplays,
          isExpanded: false,
        });
      }

      setState({ providers, isLoading: false, error: null });
    } catch (err) {
      logger.error('Failed to load provider profiles:', err);
      setState(prev => ({
        ...prev,
        isLoading: false,
        error: err instanceof Error ? err.message : 'Failed to load profiles',
      }));
    }
  }, []);

  // Load on mount
  useEffect(() => {
    void reload();
  }, [reload]);

  /**
   * Create a new profile
   */
  const createProfile = useCallback(
    async (
      providerId: string,
      profileName: string,
      config: ProfileConfig
    ): Promise<void> => {
      await saveProfile(providerId, profileName, config);
      await reload();
    },
    [reload]
  );

  /**
   * Update an existing profile
   */
  const updateProfile = useCallback(
    async (
      providerId: string,
      profileName: string,
      config: ProfileConfig
    ): Promise<void> => {
      await saveProfile(providerId, profileName, config);
      await reload();
    },
    [reload]
  );

  /**
   * Delete a profile
   */
  const removeProfile = useCallback(
    async (providerId: string, profileName: string): Promise<void> => {
      await deleteProfile(providerId, profileName);
      await reload();
    },
    [reload]
  );

  /**
   * Get a specific profile config
   */
  const getProfileConfig = useCallback(
    async (
      providerId: string,
      profileName: string
    ): Promise<ProfileConfig | undefined> => {
      return getProfile(providerId, profileName);
    },
    []
  );

  /**
   * Save API key for provider
   */
  const saveApiKey = useCallback(
    async (providerId: string, apiKey: string): Promise<void> => {
      await setProviderCredential(providerId, apiKey);
      await reload();
    },
    [reload]
  );

  /**
   * Delete API key for provider
   */
  const removeApiKey = useCallback(
    async (providerId: string): Promise<void> => {
      await deleteProviderCredential(providerId);
      await reload();
    },
    [reload]
  );

  /**
   * Test connection to provider or profile
   */
  const testConnection = useCallback(
    async (
      providerId: string,
      profileName?: string
    ): Promise<ConnectionTestResult> => {
      try {
        if (profileName) {
          // Test profile connection by fetching models
          const profile = await getProfile(providerId, profileName);
          if (!profile) {
            return {
              providerId,
              profileName,
              success: false,
              message: 'Profile not found',
            };
          }

          try {
            const models = await modelsListLocalOpenai(profile.baseUrl);
            return {
              providerId,
              profileName,
              success: true,
              message: `✓ Connected (${models.length} models)`,
            };
          } catch (err) {
            return {
              providerId,
              profileName,
              success: false,
              message: `✗ ${err instanceof Error ? err.message : 'Connection failed'}`,
            };
          }
        } else {
          // Test cloud provider connection
          const result = await testProviderConnection(providerId);
          return {
            providerId,
            success: result.success,
            message: result.success
              ? '✓ Connected'
              : `✗ ${result.error || 'Connection failed'}`,
          };
        }
      } catch (err) {
        return {
          providerId,
          profileName,
          success: false,
          message: `✗ ${err instanceof Error ? err.message : 'Test failed'}`,
        };
      }
    },
    []
  );

  /**
   * Toggle provider expansion
   */
  const toggleProviderExpansion = useCallback((providerId: string) => {
    setState(prev => ({
      ...prev,
      providers: prev.providers.map(p =>
        p.id === providerId ? { ...p, isExpanded: !p.isExpanded } : p
      ),
    }));
  }, []);

  /**
   * Toggle profile expansion
   */
  const toggleProfileExpansion = useCallback(
    (providerId: string, profileName: string) => {
      setState(prev => ({
        ...prev,
        providers: prev.providers.map(p =>
          p.id === providerId
            ? {
                ...p,
                profiles: p.profiles.map(pr =>
                  pr.name === profileName
                    ? { ...pr, isExpanded: !pr.isExpanded }
                    : pr
                ),
              }
            : p
        ),
      }));
    },
    []
  );

  return {
    state,
    reload,
    createProfile,
    updateProfile,
    removeProfile,
    getProfileConfig,
    saveApiKey,
    removeApiKey,
    testConnection,
    toggleProviderExpansion,
    toggleProfileExpansion,
  };
}

/**
 * Mask API key for display
 */
function maskApiKey(key: string): string {
  if (key.length <= 8) {
    return '•'.repeat(key.length);
  }
  const start = key.slice(0, 4);
  const end = key.slice(-4);
  return `${start}${'•'.repeat(8)}${end}`;
}
