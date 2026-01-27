/**
 * FileSearchPopup Component
 *
 * A floating popup that appears when "@" is typed,
 * showing matching files with fuzzy search capabilities.
 *
 * Work Unit: TUI-055
 */

import React, { useMemo } from 'react';
import { Box, Text } from 'ink';

export interface FileSearchResult {
  /** File path relative to project root */
  path: string;
  /** Display name (can be truncated path) */
  displayName?: string;
}

export interface FileSearchPopupProps {
  /** Whether the popup is visible */
  isVisible: boolean;
  /** Current filter text (after "@") */
  filter: string;
  /** Filtered list of files to display */
  files: FileSearchResult[];
  /** Currently selected file index */
  selectedIndex: number;
  /** Fixed dialog width (calculated from available files) */
  dialogWidth: number;
  /** Maximum height of the popup (number of visible items) */
  maxVisibleItems?: number;
  /** Loading state while searching */
  isLoading?: boolean;
}

export const FileSearchPopup: React.FC<FileSearchPopupProps> = ({
  isVisible,
  filter,
  files,
  selectedIndex,
  dialogWidth,
  maxVisibleItems = 8,
  isLoading = false,
}) => {
  // Calculate max path width for alignment
  const maxPathWidth = useMemo(() => {
    return files && files.length > 0
      ? Math.max(...files.map((f) => (f.displayName || f.path).length), 8)
      : 8;
  }, [files]);

  if (!isVisible) {
    return null;
  }

  // Ensure selectedIndex is within bounds (defensive check)
  const safeSelectedIndex = files && files.length > 0 
    ? Math.max(0, Math.min(selectedIndex, files.length - 1))
    : 0;

  // Calculate scroll offset to keep selected item visible
  const scrollOffset = Math.max(
    0,
    Math.min(
      safeSelectedIndex - Math.floor(maxVisibleItems / 2),
      Math.max(0, (files?.length || 0) - maxVisibleItems)
    )
  );

  const visibleFiles = (files || []).slice(
    scrollOffset,
    scrollOffset + maxVisibleItems
  );

  return (
    // Full-screen overlay for centering (matches SlashCommandPalette pattern)
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
        borderColor="green"
        paddingX={1}
        backgroundColor="black"
        width={dialogWidth + 4} // +4 for border and padding
      >
        {/* Header */}
        <Box>
          <Text bold color="green">
            🔍 File Search
          </Text>
          {filter && (
            <Text dimColor>
              {' '}
              for "{filter}"
            </Text>
          )}
        </Box>

        {/* File list */}
        {isLoading ? (
          <Text dimColor italic>
            Searching files...
          </Text>
        ) : (files?.length || 0) === 0 ? (
          <Text dimColor italic>
            No files found
          </Text>
        ) : (
          visibleFiles.map((file, idx) => {
            const actualIndex = scrollOffset + idx;
            const isSelected = actualIndex === safeSelectedIndex;
            const displayName = file.displayName || file.path;

            return (
              <Box key={file.path}>
                {/* Selection indicator */}
                <Text color={isSelected ? 'green' : undefined}>
                  {isSelected ? '▸ ' : '  '}
                </Text>

                {/* File path */}
                <Text
                  bold={isSelected}
                  color={isSelected ? 'white' : 'green'}
                >
                  {displayName}
                </Text>
              </Box>
            );
          })
        )}

        {/* Footer with keyboard hints */}
        <Text dimColor>↑↓ Navigate │ Enter Select │ Esc Close</Text>
      </Box>
    </Box>
  );
};