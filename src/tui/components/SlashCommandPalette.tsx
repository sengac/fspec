/**
 * SlashCommandPalette Component
 *
 * A floating popup palette that appears centered on screen when "/" is typed,
 * showing matching commands with descriptions.
 *
 * Work Unit: TUI-050
 */

import React, { useMemo } from 'react';
import { Box, Text } from 'ink';
import type { SlashCommand } from '../utils/slashCommands';

/** Maximum width for the dialog (prevents extremely wide dialogs) */
const MAX_DIALOG_WIDTH = 70;

/** Minimum width for the dialog (ensures reasonable sizing when few/no commands) */
const MIN_DIALOG_WIDTH = 45;

export interface SlashCommandPaletteProps {
  /** Whether the palette is visible */
  isVisible: boolean;
  /** Current filter text (after "/") */
  filter: string;
  /** Filtered list of commands to display */
  commands: SlashCommand[];
  /** Currently selected command index */
  selectedIndex: number;
  /** Maximum height of the palette (number of visible items) */
  maxVisibleItems?: number;
}

export const SlashCommandPalette: React.FC<SlashCommandPaletteProps> = ({
  isVisible,
  filter,
  commands,
  selectedIndex,
  maxVisibleItems = 8,
}) => {
  // Calculate fixed dialog width based on ALL commands (not just visible ones)
  // This prevents the dialog from expanding/contracting during scroll
  const dialogWidth = useMemo(() => {
    if (commands.length === 0) {
      return MIN_DIALOG_WIDTH;
    }

    // Calculate max widths across ALL commands
    const maxNameWidth = Math.max(...commands.map((c) => c.name.length), 8);
    const maxDescriptionWidth = Math.max(...commands.map((c) => c.description.length), 10);

    // Total line width: "▸ " (2) + "/" (1) + name + " " (1) + description
    const contentWidth = 2 + 1 + maxNameWidth + 1 + maxDescriptionWidth;

    // Footer width: "↑↓ Navigate │ Tab/Enter Select │ Esc Close" = 43 chars
    const footerWidth = 43;

    // Take max of content and footer, then clamp to bounds
    const calculatedWidth = Math.max(contentWidth, footerWidth, MIN_DIALOG_WIDTH);
    return Math.min(calculatedWidth, MAX_DIALOG_WIDTH);
  }, [commands]);

  // Calculate max name width for alignment (used in rendering)
  const maxNameWidth = useMemo(() => {
    return commands.length > 0
      ? Math.max(...commands.map((c) => c.name.length), 8)
      : 8;
  }, [commands]);

  if (!isVisible) {
    return null;
  }

  // Ensure selectedIndex is within bounds (defensive check)
  const safeSelectedIndex = commands.length > 0 
    ? Math.max(0, Math.min(selectedIndex, commands.length - 1))
    : 0;

  // Calculate scroll offset to keep selected item visible
  const scrollOffset = Math.max(
    0,
    Math.min(
      safeSelectedIndex - Math.floor(maxVisibleItems / 2),
      Math.max(0, commands.length - maxVisibleItems)
    )
  );

  const visibleCommands = commands.slice(
    scrollOffset,
    scrollOffset + maxVisibleItems
  );

  // Calculate available width for description (truncate if needed)
  // Width breakdown: "▸ " (2) + "/" (1) + name (maxNameWidth) + " " (1) = 4 + maxNameWidth
  const descriptionMaxWidth = dialogWidth - 4 - maxNameWidth;

  return (
    // Full-screen overlay for centering (matches Dialog pattern)
    <Box
      position="absolute"
      width="100%"
      height="100%"
      justifyContent="center"
      alignItems="center"
      flexDirection="column"
    >
      {/* Inner dialog box with fixed width and black background */}
      <Box
        flexDirection="column"
        borderStyle="round"
        borderColor="cyan"
        paddingX={1}
        backgroundColor="black"
        width={dialogWidth + 4} // +4 for border and padding
      >
        {/* Header */}
        <Box>
          <Text bold color="cyan">
            Slash Commands
          </Text>
          {filter && (
            <Text dimColor>
              {' '}
              (filter: {filter})
            </Text>
          )}
        </Box>

        {/* Separator - use dialog width for consistency */}
        <Box marginY={0}>
          <Text dimColor>{'─'.repeat(dialogWidth)}</Text>
        </Box>

        {/* Command list */}
        {commands.length === 0 ? (
          <Box paddingY={0}>
            <Text dimColor italic>
              No matching commands
            </Text>
          </Box>
        ) : (
          visibleCommands.map((command, idx) => {
            const actualIndex = scrollOffset + idx;
            const isSelected = actualIndex === safeSelectedIndex;

            // Truncate description if it exceeds available width
            const description = command.description.length > descriptionMaxWidth
              ? command.description.slice(0, descriptionMaxWidth - 1) + '…'
              : command.description;

            return (
              <Box key={command.name}>
                {/* Selection indicator */}
                <Text color={isSelected ? 'cyan' : undefined}>
                  {isSelected ? '▸ ' : '  '}
                </Text>

                {/* Command name */}
                <Text
                  bold={isSelected}
                  color={isSelected ? 'white' : 'green'}
                  backgroundColor={isSelected ? 'blue' : undefined}
                >
                  /{command.name.padEnd(maxNameWidth)}
                </Text>

                {/* Separator */}
                <Text> </Text>

                {/* Description (truncated if needed) */}
                <Text dimColor={!isSelected}>{description}</Text>
              </Box>
            );
          })
        )}

        {/* Footer with keyboard hints */}
        <Box marginY={0}>
          <Text dimColor>{'─'.repeat(dialogWidth)}</Text>
        </Box>
        <Box>
          <Text dimColor>↑↓ Navigate │ Tab/Enter Select │ Esc Close</Text>
        </Box>

        {/* Scroll indicator if needed */}
        {commands.length > maxVisibleItems && (
          <Box>
            <Text dimColor>
              ({scrollOffset + 1}-
              {Math.min(scrollOffset + maxVisibleItems, commands.length)} of{' '}
              {commands.length})
            </Text>
          </Box>
        )}
      </Box>
    </Box>
  );
};
