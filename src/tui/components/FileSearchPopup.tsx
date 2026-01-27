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

  // Get the currently selected file for preview
  const selectedFile = files && files.length > 0 ? files[safeSelectedIndex] : null;

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
        borderColor="cyan"
        paddingX={1}
        backgroundColor="black"
        width={dialogWidth + 4} // +4 for border and padding
      >
        {/* Header */}
        <Box>
          <Text bold color="cyan">
            File Search
          </Text>
          {filter && (
            <Text dimColor>
              {' '}
              (filter: {filter})
            </Text>
          )}
        </Box>

        {/* Separator - use dialog width for consistency */}
        <Box>
          <Text dimColor>{'─'.repeat(dialogWidth)}</Text>
        </Box>

        {/* File list */}
        {isLoading ? (
          <Box>
            <Text dimColor italic>
              Searching files...
            </Text>
          </Box>
        ) : (files?.length || 0) === 0 ? (
          <Box>
            <Text dimColor italic>
              No files found
            </Text>
          </Box>
        ) : (
          visibleFiles.map((file, idx) => {
            const actualIndex = scrollOffset + idx;
            const isSelected = actualIndex === safeSelectedIndex;

            // Just show filename (no path preview in list)
            const fileName = file.path.split('/').pop() || file.path;

            return (
              <Box key={file.path}>
                {/* Selection indicator */}
                <Text color={isSelected ? 'cyan' : undefined}>
                  {isSelected ? '▸ ' : '  '}
                </Text>

                {/* File name only */}
                <Text
                  bold={isSelected}
                  color={isSelected ? 'white' : 'green'}
                  backgroundColor={isSelected ? 'blue' : undefined}
                >
                  {fileName}
                </Text>
              </Box>
            );
          })
        )}

        {/* Preview section for currently selected file */}
        {selectedFile && (
          <>
            {/* Separator before preview */}
            <Box>
              <Text dimColor>{'─'.repeat(dialogWidth)}</Text>
            </Box>

            {/* Selected file full path */}
            <Box>
              <Text dimColor>
                {selectedFile.path.length > dialogWidth - 2
                  ? selectedFile.path.slice(0, dialogWidth - 3) + '…'
                  : selectedFile.path
                }
              </Text>
            </Box>
          </>
        )}

        {/* Footer with keyboard hints */}
        <Box>
          <Text dimColor>{'─'.repeat(dialogWidth)}</Text>
        </Box>
        <Box>
          <Text dimColor>↑↓ Navigate │ Tab/Enter Select │ Esc Close</Text>
        </Box>

        {/* Scroll indicator if needed */}
        {(files?.length || 0) > maxVisibleItems && (
          <Box>
            <Text dimColor>
              ({scrollOffset + 1}-
              {Math.min(scrollOffset + maxVisibleItems, files?.length || 0)} of{' '}
              {files?.length || 0})
            </Text>
          </Box>
        )}
      </Box>
    </Box>
  );
};