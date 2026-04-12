/**
 * DeleteCustomModelConfirmView - Confirmation prompt for deleting a custom model
 *
 * MODEL-004: Renders a deletion confirmation overlay within the Model Selector.
 * Follows the same pattern as delete confirmation in ProviderSettingsPanel.
 */

import React from 'react';
import { Box, Text } from 'ink';

export interface DeleteCustomModelConfirmViewProps {
  /** Model ID being deleted */
  modelId: string;
  /** Display name of the model */
  displayName: string;
  /** Profile name this model belongs to */
  profileName: string;
}

/**
 * DeleteCustomModelConfirmView Component
 */
export function DeleteCustomModelConfirmView({
  modelId,
  displayName,
  profileName,
}: DeleteCustomModelConfirmViewProps): React.ReactElement {
  return (
    <Box flexDirection="column" paddingX={2} paddingY={1}>
      <Box marginBottom={1}>
        <Text bold color="red">
          Delete Custom Model
        </Text>
      </Box>
      <Box marginBottom={1}>
        <Text>
          Are you sure you want to delete{' '}
          <Text bold color="yellow">
            {displayName || modelId}
          </Text>{' '}
          from profile{' '}
          <Text bold color="magenta">
            {profileName}
          </Text>
          ?
        </Text>
      </Box>
      <Box>
        <Text dimColor>
          y/Enter: confirm delete | n/Esc: cancel
        </Text>
      </Box>
    </Box>
  );
}
