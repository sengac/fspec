/**
 * CopilotOauthRender — render branches for the GitHub Copilot device flow
 * (PROV-054).
 *
 * Two new mode renderings:
 *
 * 1. `oauth-deployment-type-select` — radio-style list of github.com /
 *    enterprise so the user can pick which deployment to authenticate
 *    against.
 * 2. `oauth-enterprise-url-entry` — text input box (mirrors the headless
 *    code-entry layout) where the user types the enterprise host before
 *    the device-code request is issued.
 *
 * SoC: this file owns ONLY the JSX for these two new screens. Keyboard
 * input is handled in `inputHandlers/copilotOauthModeHandler.ts`. State
 * transitions are owned by `utils/copilotLoginFlow.ts`.
 */

import React from 'react';
import { Box, Text } from 'ink';

export interface DeploymentTypeSelectProps {
  width: number;
  height: number;
  selectedIndex: 0 | 1;
}

export interface EnterpriseUrlEntryProps {
  width: number;
  height: number;
  urlInput: string;
  validationError?: string;
}

const DEPLOYMENT_OPTIONS: ReadonlyArray<{ value: string; hint: string }> = [
  { value: 'GitHub.com', hint: 'Public' },
  { value: 'GitHub Enterprise', hint: 'Self-hosted / data residency' },
];

/**
 * Render the deployment-type radio prompt.
 */
export function renderCopilotDeploymentTypeSelect({
  width,
  height,
  selectedIndex,
}: DeploymentTypeSelectProps): React.ReactElement {
  return (
    <Box flexDirection="column" width={width} height={height} backgroundColor="black">
      <Box flexDirection="column" padding={2}>
        <Text bold color="yellow">
          GitHub Copilot Login — Select deployment type
        </Text>
        <Box marginTop={1} flexDirection="column">
          {DEPLOYMENT_OPTIONS.map((opt, idx) => {
            const isActive = idx === selectedIndex;
            return (
              <Text
                key={opt.value}
                color={isActive ? 'black' : 'white'}
                backgroundColor={isActive ? 'cyan' : undefined}
              >
                {isActive ? '▶ ' : '  '}
                {opt.value}
                <Text dimColor={!isActive}> — {opt.hint}</Text>
              </Text>
            );
          })}
        </Box>
        <Box marginTop={1}>
          <Text dimColor>↑/↓: switch · Enter: select · Esc: cancel</Text>
        </Box>
      </Box>
    </Box>
  );
}

/**
 * Render the enterprise URL text input prompt.
 */
export function renderCopilotEnterpriseUrlEntry({
  width,
  height,
  urlInput,
  validationError,
}: EnterpriseUrlEntryProps): React.ReactElement {
  return (
    <Box flexDirection="column" width={width} height={height} backgroundColor="black">
      <Box flexDirection="column" padding={2}>
        <Text bold color="yellow">
          GitHub Copilot Login — Enter Enterprise URL
        </Text>
        <Box marginTop={1}>
          <Text>Type your enterprise host (e.g. company.ghe.com):</Text>
        </Box>
        <Box marginTop={1} width={Math.max(20, width - 12)}>
          <Text color="cyan">URL: </Text>
          <Text wrap="truncate">
            {urlInput || <Text dimColor>company.ghe.com</Text>}
            <Text inverse> </Text>
          </Text>
        </Box>
        {validationError && (
          <Box marginTop={1}>
            <Text color="red">{validationError}</Text>
          </Box>
        )}
        <Box marginTop={1}>
          <Text dimColor>Enter: submit · Esc: cancel</Text>
        </Box>
      </Box>
    </Box>
  );
}
