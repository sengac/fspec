/**
 * ProviderSettingsPanel - Presentation component for provider settings
 *
 * PROV-007: Renders the provider settings UI. All state and input handling
 * is managed by the parent component (AgentView).
 *
 * This is a presentational component - no internal state management.
 */

import React from 'react';
import { Box, Text } from 'ink';
import type { ProfileConfig } from '../../utils/provider-config';
import { getProviderRegistryEntry } from '../../utils/provider-config';
import { getFooterHints } from '../utils/providerSettingsHelpers';

/**
 * Provider status for display
 */
export interface ProviderDisplayStatus {
  hasKey: boolean;
  maskedKey?: string;
  source?: 'env' | 'file' | 'dotenv' | 'ChatGPT' | 'Claude';
}

/**
 * Profile display info
 */
export interface ProfileDisplayInfo {
  name: string;
  config: ProfileConfig;
}

/**
 * Provider with profiles for display
 */
export interface ProviderDisplayInfo {
  id: string;
  name: string;
  status: ProviderDisplayStatus;
  profiles: ProfileDisplayInfo[];
  isExpanded: boolean;
  /** Whether this provider has existing OAuth tokens */
  hasOAuthTokens?: boolean;
}

/**
 * Connection test result
 */
export interface TestResult {
  providerId: string;
  profileName?: string;
  success: boolean;
  message: string;
}

/**
 * Panel mode
 */
export type PanelMode =
  | { type: 'list' }
  | { type: 'edit-api-key'; providerId: string; currentValue?: string }
  | { type: 'delete-api-key'; providerId: string }
  | { type: 'disconnect-oauth'; providerId: string }
  | {
      type: 'profile-form';
      providerId: string;
      profileName: string;
      isNew: boolean;
      values: Partial<ProfileConfig>;
      activeField: number;
      isEditingName: boolean;
    }
  | { type: 'delete-confirm'; providerId: string; profileName: string }
  | {
      type: 'oauth-browser-waiting';
      providerId: string;
    }
  | {
      type: 'oauth-device-waiting';
      providerId: string;
      userCode: string;
      verificationUrl: string;
    }
  | {
      type: 'oauth-success';
      providerId: string;
    }
  | {
      type: 'oauth-error';
      providerId: string;
      error: string;
    }
  | {
      type: 'oauth-headless-code-entry';
      providerId: string;
      authorizeUrl: string;
      pkceVerifier: string;
      codeInput: string;
    };

/**
 * Navigation item for flat list
 */
export type SettingsNavItem =
  | { type: 'provider'; providerId: string; name: string }
  | { type: 'profile'; providerId: string; profileName: string }
  | { type: 'add-profile'; providerId: string }
  | { type: 'api-key'; providerId: string }
  | {
      type: 'oauth-login';
      providerId: string;
      method: 'browser' | 'headless';
      label: string;
    }
  | {
      type: 'oauth-status';
      providerId: string;
      label: string;
    };

/**
 * Props for ProviderSettingsPanel
 */
interface ProviderSettingsPanelProps {
  width: number;
  height: number;
  providers: ProviderDisplayInfo[];
  navItems: SettingsNavItem[];
  selectedIndex: number;
  scrollOffset: number;
  visibleHeight: number;
  mode: PanelMode;
  filter: string;
  isFilterMode: boolean;
  testResult: TestResult | null;
}

/**
 * Profile form field labels
 */
const FIELD_LABELS: Array<{ key: keyof ProfileConfig; label: string }> = [
  { key: 'baseUrl', label: 'Base URL' },
  { key: 'apiKey', label: 'API Key' },
  { key: 'contextWindow', label: 'Context Window' },
  { key: 'maxOutputTokens', label: 'Max Output Tokens' },
];

/**
 * ProviderSettingsPanel Component
 */
export function ProviderSettingsPanel({
  width,
  height,
  providers,
  navItems,
  selectedIndex,
  scrollOffset,
  visibleHeight,
  mode,
  filter,
  isFilterMode,
  testResult,
}: ProviderSettingsPanelProps): React.ReactElement {
  const contentWidth = width - 4 - 3;

  // Render delete confirmation
  if (mode.type === 'delete-confirm') {
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
              Delete profile {mode.profileName}? (y/n)
            </Text>
          </Box>
          <Box marginTop={1}>
            <Text dimColor>Press 'y' to confirm, 'n' or Esc to cancel</Text>
          </Box>
        </Box>
      </Box>
    );
  }

  // Render delete-api-key confirmation
  if (mode.type === 'delete-api-key') {
    const provider = providers.find(p => p.id === mode.providerId);
    return (
      <Box
        flexDirection="column"
        width={width}
        height={height}
        backgroundColor="black"
      >
        <Box flexDirection="column" padding={2}>
          <Text bold color="red">
            Delete API Key
          </Text>
          <Box marginTop={1}>
            <Text>
              Delete API key for {provider?.name || mode.providerId}? (y/n)
            </Text>
          </Box>
          <Box marginTop={1}>
            <Text dimColor>Press 'y' to confirm, 'n' or Esc to cancel</Text>
          </Box>
        </Box>
      </Box>
    );
  }

  // Render disconnect-oauth confirmation
  if (mode.type === 'disconnect-oauth') {
    const provider = providers.find(p => p.id === mode.providerId);
    const oauthLabel = mode.providerId === 'anthropic' ? 'Claude' : 'ChatGPT';
    return (
      <Box
        flexDirection="column"
        width={width}
        height={height}
        backgroundColor="black"
      >
        <Box flexDirection="column" padding={2}>
          <Text bold color="red">
            Disconnect OAuth
          </Text>
          <Box marginTop={1}>
            <Text>
              Disconnect {oauthLabel} OAuth? (y/n)
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
    const provider = providers.find(p => p.id === mode.providerId);
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
              {mode.currentValue ? '•'.repeat(mode.currentValue.length) : ''}
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

  // Render profile form
  if (mode.type === 'profile-form') {
    const provider = providers.find(p => p.id === mode.providerId);

    return (
      <Box
        flexDirection="column"
        width={width}
        height={height}
        backgroundColor="black"
      >
        <Box flexDirection="column" padding={2}>
          <Text bold color="yellow">
            {mode.isNew ? 'Create Profile' : 'Edit Profile'}:{' '}
            {provider?.name || mode.providerId}
          </Text>

          {/* Profile name */}
          <Box marginTop={1}>
            <Text
              color={mode.isEditingName ? 'cyan' : 'white'}
              backgroundColor={mode.isEditingName ? 'blue' : undefined}
            >
              Profile Name:{' '}
            </Text>
            <Text>
              {mode.profileName}
              {mode.isEditingName && <Text inverse> </Text>}
            </Text>
            {mode.isNew && !mode.profileName && (
              <Text color="red"> *</Text>
            )}
          </Box>

          {/* Form fields */}
          {FIELD_LABELS.map((field, idx) => {
            const isActive = !mode.isEditingName && idx === mode.activeField;
            const value = mode.values[field.key];
            const isPassword = field.key === 'apiKey';
            const displayValue =
              isPassword && value
                ? '•'.repeat(String(value).length)
                : String(value || '');
            const isRequired = field.key === 'baseUrl' || field.key === 'apiKey';

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
                    <Text dimColor>
                      {field.key === 'baseUrl'
                        ? 'http://localhost:8888'
                        : field.key === 'contextWindow'
                          ? '128000'
                          : field.key === 'maxOutputTokens'
                            ? '16384'
                            : 'Enter value'}
                    </Text>
                  )}
                  {isActive && <Text inverse> </Text>}
                </Text>
                {isRequired && !value && <Text color="red"> *</Text>}
              </Box>
            );
          })}

          <Box marginTop={2}>
            <Text dimColor>
              ↑/↓: switch field | Enter: save | Esc: cancel
            </Text>
          </Box>
        </Box>
      </Box>
    );
  }

  // Render OAuth browser waiting
  if (mode.type === 'oauth-browser-waiting') {
    const oauthTitle =
      mode.providerId === 'anthropic'
        ? 'Claude OAuth Login'
        : 'Codex OAuth Login';
    return (
      <Box
        flexDirection="column"
        width={width}
        height={height}
        backgroundColor="black"
      >
        <Box flexDirection="column" padding={2}>
          <Text bold color="yellow">
            {oauthTitle}
          </Text>
          <Box marginTop={1}>
            <Text color="cyan">⠋ </Text>
            <Text>Waiting for authorization...</Text>
          </Box>
          <Box marginTop={1}>
            <Text dimColor>Press Esc to cancel</Text>
          </Box>
        </Box>
      </Box>
    );
  }

  // Render OAuth device waiting
  if (mode.type === 'oauth-device-waiting') {
    return (
      <Box
        flexDirection="column"
        width={width}
        height={height}
        backgroundColor="black"
      >
        <Box flexDirection="column" padding={2}>
          <Text bold color="yellow">
            Codex Device Login
          </Text>
          <Box marginTop={1}>
            <Text>Your code: </Text>
            <Text bold color="cyan">
              {mode.userCode}
            </Text>
          </Box>
          <Box marginTop={1}>
            <Text>Visit: </Text>
            <Text color="blue">{mode.verificationUrl}</Text>
          </Box>
          <Box marginTop={1}>
            <Text color="cyan">⠋ </Text>
            <Text>Enter the code on another device</Text>
          </Box>
          <Box marginTop={1}>
            <Text dimColor>Press Esc to cancel</Text>
          </Box>
        </Box>
      </Box>
    );
  }

  // Render OAuth success
  if (mode.type === 'oauth-success') {
    const successLabel =
      mode.providerId === 'anthropic'
        ? '✓ Connected to Claude'
        : '✓ Connected to ChatGPT';
    return (
      <Box
        flexDirection="column"
        width={width}
        height={height}
        backgroundColor="black"
      >
        <Box flexDirection="column" padding={2}>
          <Text bold color="green">
            {successLabel}
          </Text>
          <Box marginTop={1}>
            <Text dimColor>Press Enter or Esc to continue</Text>
          </Box>
        </Box>
      </Box>
    );
  }

  // Render OAuth headless code entry (Claude only)
  if (mode.type === 'oauth-headless-code-entry') {
    return (
      <Box
        flexDirection="column"
        width={width}
        height={height}
        backgroundColor="black"
      >
        <Box flexDirection="column" padding={2}>
          <Text bold color="yellow">
            Claude Headless Login
          </Text>
          <Box marginTop={1}>
            <Text>Visit: </Text>
            <Text color="blue">{mode.authorizeUrl}</Text>
          </Box>
          <Box marginTop={1}>
            <Text>Authorize on claude.ai, then paste code#state below:</Text>
          </Box>
          <Box marginTop={1} width={Math.max(20, width - 12)}>
            <Text color="cyan">Code: </Text>
            <Text wrap="truncate">
              {mode.codeInput}
              <Text inverse> </Text>
            </Text>
          </Box>
          <Box marginTop={1}>
            <Text dimColor>Enter to submit | c: copy URL | o: open URL | Esc to cancel</Text>
          </Box>
        </Box>
      </Box>
    );
  }

  // Render OAuth error
  if (mode.type === 'oauth-error') {
    return (
      <Box
        flexDirection="column"
        width={width}
        height={height}
        backgroundColor="black"
      >
        <Box flexDirection="column" padding={2}>
          <Text bold color="red">
            OAuth Login error
          </Text>
          <Box marginTop={1}>
            <Text color="red">{mode.error}</Text>
          </Box>
          <Box marginTop={1}>
            <Text dimColor>Press Enter to retry | Esc to go back</Text>
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
          <Text dimColor> ({navItems.length} items)</Text>
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
        <Box flexDirection="row" flexGrow={1}>
          <Box flexDirection="column" flexGrow={1}>
            {navItems
              .slice(scrollOffset, scrollOffset + visibleHeight)
              .map((item, visibleIdx) => {
                const actualIdx = scrollOffset + visibleIdx;
                const isSelected = actualIdx === selectedIndex;
                const provider = providers.find(p => p.id === item.providerId);

                if (item.type === 'provider') {
                  const status = provider?.status;
                  const isExpanded = provider?.isExpanded;
                  const profileCount = provider?.profiles.length || 0;

                  return (
                    <Box
                      key={`provider-${item.providerId}`}
                      width={contentWidth}
                    >
                      <Text
                        backgroundColor={isSelected ? 'yellow' : undefined}
                        color={isSelected ? 'black' : 'white'}
                        wrap="truncate"
                      >
                        {isSelected ? '> ' : '  '}
                        {isExpanded ? '▼ ' : '▶ '}
                        {item.name}
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
                        {profileCount > 0 && item.providerId === 'openai' && (
                          <Text dimColor={!isSelected}>
                            {' '}
                            ({profileCount} profile
                            {profileCount !== 1 ? 's' : ''})
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

                // oauth-login
                if (item.type === 'oauth-login') {
                  return (
                    <Box
                      key={`oauth-${item.providerId}-${item.method}`}
                      width={contentWidth}
                    >
                      <Text
                        backgroundColor={isSelected ? 'magenta' : undefined}
                        color={isSelected ? 'black' : 'magenta'}
                        wrap="truncate"
                      >
                        {isSelected ? '> ' : '  '}
                        {'    '}🔑 {item.label}
                      </Text>
                    </Box>
                  );
                }

                // oauth-status (PROV-028: show auth status when OAuth provider has tokens)
                if (item.type === 'oauth-status') {
                  return (
                    <Box
                      key={`oauth-status-${item.providerId}`}
                      width={contentWidth}
                    >
                      <Text
                        backgroundColor={isSelected ? 'green' : undefined}
                        color={isSelected ? 'black' : 'green'}
                        wrap="truncate"
                      >
                        {isSelected ? '> ' : '  '}
                        {'    '}{item.label}
                      </Text>
                    </Box>
                  );
                }

                // api-key
                if (item.type === 'api-key') {
                  const apiStatus = provider?.status;
                  const registryEntry = getProviderRegistryEntry(item.providerId);
                  return (
                    <Box
                      key={`api-key-${item.providerId}`}
                      width={contentWidth}
                    >
                      <Text
                        backgroundColor={isSelected ? 'yellow' : undefined}
                        color={isSelected ? 'black' : 'yellow'}
                        wrap="truncate"
                      >
                        {isSelected ? '> ' : '  '}
                        {'    '}🔑 API key
                        {apiStatus?.hasKey ? (
                          <Text color={isSelected ? 'black' : 'green'}>
                            {' '}✓ {apiStatus.maskedKey}
                            {apiStatus.source && (
                              <Text dimColor={!isSelected}>
                                {' '}[{apiStatus.source}]
                              </Text>
                            )}
                          </Text>
                        ) : (
                          <Text color={isSelected ? 'black' : 'gray'}>
                            {' '}(not set)
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

          {/* Scrollbar */}
          {navItems.length > visibleHeight && (
            <Box flexDirection="column" marginLeft={1}>
              {Array.from({ length: visibleHeight }).map((_, i) => {
                const thumbHeight = Math.max(
                  1,
                  Math.floor(
                    (visibleHeight / navItems.length) * visibleHeight
                  )
                );
                const thumbPos = Math.floor(
                  (scrollOffset / navItems.length) * visibleHeight
                );
                const isThumb = i >= thumbPos && i < thumbPos + thumbHeight;
                return (
                  <Text key={i} dimColor>
                    {isThumb ? '■' : '│'}
                  </Text>
                );
              })}
            </Box>
          )}
        </Box>

        {/* Footer */}
        <Box marginTop={1}>
          <Text dimColor>
            {getFooterHints(navItems[selectedIndex]?.type ?? 'provider')}
          </Text>
        </Box>
      </Box>
    </Box>
  );
}
