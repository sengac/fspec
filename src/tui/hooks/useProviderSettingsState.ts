/**
 * useProviderSettingsState - Hook for provider settings panel state
 *
 * PROV-007: Manages the state needed for ProviderSettingsPanel.
 * Handles provider list, profiles, expansion, and navigation.
 */

import { useState, useCallback, useEffect, useMemo, useRef } from 'react';
import {
  loadProviderProfiles,
  saveProfile,
  deleteProfile,
  getProfile,
  getProviderRegistry,
  getProviderRegistryEntry,
  isOAuthProvider,
  type ProfileConfig,
} from '../../utils/provider-config';
import {
  getProviderConfig,
  setProviderCredential,
  deleteProviderCredential,
  maskApiKey,
} from '../../utils/credentials';
import {
  modelsListLocalOpenai,
  testProviderConnection,
  codexOauthGetTokens,
  codexOauthBrowserLogin,
  codexOauthDeviceLoginStart,
  codexOauthDeviceLoginPoll,
  codexOauthClearTokens,
} from '@sengac/codelet-napi';
import { logger } from '../../utils/logger';
import type {
  ProviderDisplayInfo,
  ProviderDisplayStatus,
  ProfileDisplayInfo,
  SettingsNavItem,
  PanelMode,
  TestResult,
} from '../components/ProviderSettingsPanel';

/**
 * Hook return type
 */
export interface UseProviderSettingsStateReturn {
  // Data
  providers: ProviderDisplayInfo[];
  navItems: SettingsNavItem[];
  isLoading: boolean;

  // Navigation state
  selectedIndex: number;
  scrollOffset: number;

  // Mode state
  mode: PanelMode;
  filter: string;
  isFilterMode: boolean;
  testResult: TestResult | null;

  // Form state (for profile form mode)
  formValues: Partial<ProfileConfig>;
  profileName: string;
  formFieldIndex: number;
  isEditingName: boolean;

  // API key edit state
  editingApiKey: string;

  // OAuth retry context (needed to know which flow to restart)
  oauthLastMethod: 'browser' | 'device' | null;

  // Actions
  reload: () => Promise<void>;
  setSelectedIndex: (index: number) => void;
  setScrollOffset: (offset: number) => void;
  setMode: (mode: PanelMode) => void;
  setFilter: (filter: string) => void;
  setIsFilterMode: (isFilterMode: boolean) => void;
  setTestResult: (result: TestResult | null) => void;
  setFormValues: (
    values:
      | Partial<ProfileConfig>
      | ((prev: Partial<ProfileConfig>) => Partial<ProfileConfig>)
  ) => void;
  setProfileName: (name: string | ((prev: string) => string)) => void;
  setFormFieldIndex: (index: number | ((prev: number) => number)) => void;
  setIsEditingName: (isEditing: boolean) => void;
  setEditingApiKey: (key: string | ((prev: string) => string)) => void;

  // Operations
  toggleProviderExpansion: (providerId: string) => void;
  saveApiKey: (providerId: string, apiKey: string) => Promise<void>;
  removeApiKey: (providerId: string) => Promise<void>;
  saveProfileConfig: (
    providerId: string,
    name: string,
    config: ProfileConfig
  ) => Promise<void>;
  removeProfile: (providerId: string, name: string) => Promise<void>;
  testConnection: (
    providerId: string,
    profileName?: string
  ) => Promise<TestResult>;
  getCurrentItem: () => SettingsNavItem | undefined;
  getCurrentProvider: () => ProviderDisplayInfo | undefined;
  getCurrentProfile: () => ProfileDisplayInfo | undefined;

  // OAuth operations
  startBrowserLogin: (providerId: string) => void;
  startDeviceLogin: (providerId: string) => void;
  cancelOauth: () => void;
  retryOauth: () => void;
  disconnectOauth: (providerId: string) => Promise<void>;
}

/**
 * Hook for managing provider settings state
 */
export function useProviderSettingsState(): UseProviderSettingsStateReturn {
  // Provider data
  const [providers, setProviders] = useState<ProviderDisplayInfo[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  // Navigation state
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [scrollOffset, setScrollOffset] = useState(0);

  // Mode state
  const [mode, setMode] = useState<PanelMode>({ type: 'list' });
  const [filter, setFilter] = useState('');
  const [isFilterMode, setIsFilterMode] = useState(false);
  const [testResult, setTestResult] = useState<TestResult | null>(null);

  // Form state
  const [formValues, setFormValues] = useState<Partial<ProfileConfig>>({});
  const [profileName, setProfileName] = useState('');
  const [formFieldIndex, setFormFieldIndex] = useState(0);
  const [isEditingName, setIsEditingName] = useState(false);

  // API key edit state
  const [editingApiKey, setEditingApiKey] = useState('');

  // OAuth retry context (not used for rendering — refs avoid unnecessary re-renders)
  const oauthLastMethodRef = useRef<'browser' | 'device' | null>(null);
  const oauthProviderIdRef = useRef<string | null>(null);

  // Generation counter to invalidate stale OAuth promises after cancel
  const oauthGeneration = useRef(0);

  /**
   * Load all providers and their profiles
   */
  const reload = useCallback(async () => {
    setIsLoading(true);

    try {
      const providerIds = getProviderRegistry();
      const loadedProviders: ProviderDisplayInfo[] = [];

      for (const providerId of providerIds) {
        const registryEntry = getProviderRegistryEntry(providerId);
        if (!registryEntry) {
          continue;
        }

        // Get provider credentials/status
        const providerConfig = await getProviderConfig(providerId);
        let status: ProviderDisplayStatus = {
          hasKey: !!providerConfig.apiKey,
          maskedKey: providerConfig.apiKey
            ? maskApiKey(providerConfig.apiKey)
            : undefined,
          source: providerConfig.source as
            | 'env'
            | 'file'
            | 'dotenv'
            | undefined,
        };

        // Check for OAuth tokens on OAuth providers
        let hasOAuthTokens = false;
        if (isOAuthProvider(providerId)) {
          try {
            const tokens = codexOauthGetTokens();
            if (tokens) {
              hasOAuthTokens = true;
              status = {
                hasKey: true,
                maskedKey: 'OAuth',
                source: 'ChatGPT',
              };
            }
          } catch {
            // OAuth token check failed, continue with normal status
          }
        }

        // Load profiles for this provider
        const profiles = await loadProviderProfiles(providerId);
        const profileInfos: ProfileDisplayInfo[] = Object.entries(profiles).map(
          ([name, config]) => ({
            name,
            config,
          })
        );

        loadedProviders.push({
          id: providerId,
          name: registryEntry.name,
          status,
          profiles: profileInfos,
          isExpanded: false,
          hasOAuthTokens,
        });
      }

      setProviders(loadedProviders);
    } catch (err) {
      logger.error('Failed to load provider settings:', err);
    } finally {
      setIsLoading(false);
    }
  }, []);

  // Load on mount
  useEffect(() => {
    void reload();
  }, [reload]);

  /**
   * Build navigation items from providers
   */
  const navItems = useMemo((): SettingsNavItem[] => {
    const items: SettingsNavItem[] = [];
    const filterLower = filter.toLowerCase();

    for (const provider of providers) {
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
        name: provider.name,
      });

      // Add profiles if expanded
      if (provider.isExpanded) {
        // Add OAuth login options for OAuth providers when no tokens exist
        if (isOAuthProvider(provider.id) && !provider.hasOAuthTokens) {
          items.push({
            type: 'oauth-login',
            providerId: provider.id,
            method: 'browser',
            label: 'Login with ChatGPT (browser)',
          });
          items.push({
            type: 'oauth-login',
            providerId: provider.id,
            method: 'headless',
            label: 'Login with ChatGPT (headless)',
          });
        }

        for (const profile of provider.profiles) {
          items.push({
            type: 'profile',
            providerId: provider.id,
            profileName: profile.name,
          });
        }
        // Add "Create Profile" option (not for OAuth providers — they use OAuth, not profiles)
        if (!isOAuthProvider(provider.id)) {
          items.push({
            type: 'add-profile',
            providerId: provider.id,
          });
        }
      }
    }

    return items;
  }, [providers, filter]);

  /**
   * Toggle provider expansion
   */
  const toggleProviderExpansion = useCallback((providerId: string) => {
    setProviders(prev =>
      prev.map(p =>
        p.id === providerId ? { ...p, isExpanded: !p.isExpanded } : p
      )
    );
  }, []);

  /**
   * Save API key
   */
  const saveApiKey = useCallback(
    async (providerId: string, apiKey: string): Promise<void> => {
      await setProviderCredential(providerId, apiKey);
      await reload();
    },
    [reload]
  );

  /**
   * Remove API key
   */
  const removeApiKey = useCallback(
    async (providerId: string): Promise<void> => {
      await deleteProviderCredential(providerId);
      await reload();
    },
    [reload]
  );

  /**
   * Save profile config
   */
  const saveProfileConfig = useCallback(
    async (
      providerId: string,
      name: string,
      config: ProfileConfig
    ): Promise<void> => {
      await saveProfile(providerId, name, config);
      await reload();
    },
    [reload]
  );

  /**
   * Remove profile
   */
  const removeProfile = useCallback(
    async (providerId: string, name: string): Promise<void> => {
      await deleteProfile(providerId, name);
      await reload();
    },
    [reload]
  );

  /**
   * Test connection
   */
  const testConnection = useCallback(
    async (
      providerId: string,
      profileNameParam?: string
    ): Promise<TestResult> => {
      try {
        if (profileNameParam) {
          const profile = await getProfile(providerId, profileNameParam);
          if (!profile) {
            return {
              providerId,
              profileName: profileNameParam,
              success: false,
              message: 'Profile not found',
            };
          }

          try {
            const models = await modelsListLocalOpenai(profile.baseUrl);
            return {
              providerId,
              profileName: profileNameParam,
              success: true,
              message: `✓ Connected (${models.length} models)`,
            };
          } catch (err) {
            return {
              providerId,
              profileName: profileNameParam,
              success: false,
              message: `✗ ${err instanceof Error ? err.message : 'Connection failed'}`,
            };
          }
        } else {
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
          profileName: profileNameParam,
          success: false,
          message: `✗ ${err instanceof Error ? err.message : 'Test failed'}`,
        };
      }
    },
    []
  );

  /**
   * Get current navigation item
   */
  const getCurrentItem = useCallback((): SettingsNavItem | undefined => {
    return navItems[selectedIndex];
  }, [navItems, selectedIndex]);

  /**
   * Get current provider
   */
  const getCurrentProvider = useCallback(():
    | ProviderDisplayInfo
    | undefined => {
    const item = navItems[selectedIndex];
    if (!item) {
      return undefined;
    }
    return providers.find(p => p.id === item.providerId);
  }, [navItems, selectedIndex, providers]);

  /**
   * Get current profile
   */
  const getCurrentProfile = useCallback((): ProfileDisplayInfo | undefined => {
    const item = navItems[selectedIndex];
    if (!item || item.type !== 'profile') {
      return undefined;
    }
    const provider = providers.find(p => p.id === item.providerId);
    return provider?.profiles.find(pr => pr.name === item.profileName);
  }, [navItems, selectedIndex, providers]);

  /**
   * Reset OAuth retry context
   */
  const resetOauthState = useCallback(() => {
    oauthLastMethodRef.current = null;
    oauthProviderIdRef.current = null;
  }, []);

  /**
   * Start browser OAuth login flow
   */
  const startBrowserLogin = useCallback(
    (providerId: string) => {
      const thisGeneration = ++oauthGeneration.current;
      oauthLastMethodRef.current = 'browser';
      oauthProviderIdRef.current = providerId;
      setMode({ type: 'oauth-browser-waiting', providerId });

      void (async () => {
        try {
          await codexOauthBrowserLogin();
          if (oauthGeneration.current !== thisGeneration) {
            return;
          }
          setMode({ type: 'oauth-success', providerId });
          await reload();
        } catch (err) {
          if (oauthGeneration.current !== thisGeneration) {
            return;
          }
          const errorMsg =
            err instanceof Error ? err.message : 'OAuth login failed';
          setMode({ type: 'oauth-error', providerId, error: errorMsg });
        }
      })();
    },
    [reload]
  );

  /**
   * Start device auth login flow
   */
  const startDeviceLogin = useCallback(
    (providerId: string) => {
      const thisGeneration = ++oauthGeneration.current;
      oauthLastMethodRef.current = 'device';
      oauthProviderIdRef.current = providerId;

      void (async () => {
        try {
          const result = await codexOauthDeviceLoginStart();
          if (oauthGeneration.current !== thisGeneration) {
            return;
          }
          setMode({
            type: 'oauth-device-waiting',
            providerId,
            userCode: result.userCode,
            verificationUrl: result.verificationUrl,
          });

          // Start polling
          await codexOauthDeviceLoginPoll(result.deviceAuthId, result.interval);
          if (oauthGeneration.current !== thisGeneration) {
            return;
          }
          setMode({ type: 'oauth-success', providerId });
          await reload();
        } catch (err) {
          if (oauthGeneration.current !== thisGeneration) {
            return;
          }
          const errorMsg =
            err instanceof Error ? err.message : 'Device auth failed';
          setMode({ type: 'oauth-error', providerId, error: errorMsg });
        }
      })();
    },
    [reload]
  );

  /**
   * Cancel OAuth flow
   */
  const cancelOauth = useCallback(() => {
    oauthGeneration.current++; // Invalidate any running OAuth promise
    resetOauthState();
    setMode({ type: 'list' });
  }, [resetOauthState]);

  /**
   * Retry OAuth flow
   */
  const retryOauth = useCallback(() => {
    const pid = oauthProviderIdRef.current;
    const method = oauthLastMethodRef.current;
    if (!pid) {
      return;
    }
    resetOauthState();
    if (method === 'browser') {
      startBrowserLogin(pid);
    } else if (method === 'device') {
      startDeviceLogin(pid);
    }
  }, [resetOauthState, startBrowserLogin, startDeviceLogin]);

  /**
   * Disconnect OAuth (clear stored tokens)
   */
  const disconnectOauth = useCallback(
    async (providerId: string): Promise<void> => {
      try {
        if (isOAuthProvider(providerId)) {
          codexOauthClearTokens();
        }
        await reload();
      } catch (err) {
        logger.error('Failed to disconnect OAuth:', err);
      }
    },
    [reload]
  );

  return {
    providers,
    navItems,
    isLoading,
    selectedIndex,
    scrollOffset,
    mode,
    filter,
    isFilterMode,
    testResult,
    formValues,
    profileName,
    formFieldIndex,
    isEditingName,
    editingApiKey,
    oauthLastMethod: oauthLastMethodRef.current,
    reload,
    setSelectedIndex,
    setScrollOffset,
    setMode,
    setFilter,
    setIsFilterMode,
    setTestResult,
    setFormValues,
    setProfileName,
    setFormFieldIndex,
    setIsEditingName,
    setEditingApiKey,
    toggleProviderExpansion,
    saveApiKey,
    removeApiKey,
    saveProfileConfig,
    removeProfile,
    testConnection,
    getCurrentItem,
    getCurrentProvider,
    getCurrentProfile,
    startBrowserLogin,
    startDeviceLogin,
    cancelOauth,
    retryOauth,
    disconnectOauth,
  };
}
