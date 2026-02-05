/**
 * AnchorView - Full-screen anchor point viewer with split-pane layout
 *
 * Displays anchor points from the current session with:
 * - Left pane: Anchor list with rich metadata (type, turn, score, timestamp)
 * - Right pane: Selected anchor's turn content (scrollable)
 *
 * Follows WatcherCreateView pattern for full-screen input isolation.
 * Uses CRITICAL priority input handler that consumes ALL keystrokes.
 *
 * Composed of:
 * - AnchorListPane: Left pane with anchor list
 * - AnchorPreviewPane: Right pane with turn details
 * - AnchorListItem: Individual anchor item rendering
 */

import React, { useState, useCallback, useEffect } from 'react';
import { Box, Text } from 'ink';
import { AnchorListPane } from './AnchorListPane';
import { AnchorPreviewPane } from './AnchorPreviewPane';
import { useInputCompat, InputPriority } from '../input/index';
import { useTerminalSize } from '../hooks/useTerminalSize';
import type { AnchorPoint, AnchorTurnDetails } from '../types/anchor';

export interface AnchorViewProps {
  /** Whether the view is visible */
  isVisible: boolean;
  /** Array of anchor points to display */
  anchorPoints: AnchorPoint[];
  /** Callback to close the view */
  onClose: () => void;
  /** Callback to get turn details for a specific turn index */
  onGetTurnDetails: (turnIndex: number) => Promise<AnchorTurnDetails | null>;
  /** For testing: override terminal width */
  _terminalWidth?: number;
  /** For testing: override terminal height */
  _terminalHeight?: number;
}

// Layout constants
const LEFT_PANE_RATIO = 0.4;
const HEADER_FOOTER_HEIGHT = 4; // Header + 2 separators + footer
const PANE_CHROME_HEIGHT = 4; // Border + header margin

export function AnchorView({
  isVisible,
  anchorPoints,
  onClose,
  onGetTurnDetails,
  _terminalWidth,
  _terminalHeight,
}: AnchorViewProps): React.ReactElement | null {
  const terminalSize = useTerminalSize();
  const terminalWidth = _terminalWidth ?? terminalSize.width;
  const terminalHeight = _terminalHeight ?? terminalSize.height;

  const [selectedAnchor, setSelectedAnchor] = useState<AnchorPoint | null>(
    anchorPoints.length > 0 ? anchorPoints[0] : null
  );
  const [turnDetails, setTurnDetails] = useState<AnchorTurnDetails | null>(null);
  const [isLoadingDetails, setIsLoadingDetails] = useState(false);

  // Reset selection when view becomes visible
  useEffect(() => {
    if (isVisible && anchorPoints.length > 0) {
      setSelectedAnchor(anchorPoints[0]);
      setTurnDetails(null);
    }
  }, [isVisible, anchorPoints]);

  // Load turn details when selection changes
  useEffect(() => {
    if (!isVisible || !selectedAnchor) {
      return;
    }

    setIsLoadingDetails(true);
    onGetTurnDetails(selectedAnchor.turnIndex)
      .then(details => {
        setTurnDetails(details);
        setIsLoadingDetails(false);
      })
      .catch(() => {
        setTurnDetails(null);
        setIsLoadingDetails(false);
      });
  }, [isVisible, selectedAnchor, onGetTurnDetails]);

  // Handler for anchor selection from AnchorListPane
  const handleAnchorSelect = useCallback((anchor: AnchorPoint) => {
    setSelectedAnchor(anchor);
  }, []);

  // Keyboard input handling - only handle Escape and consume other keys
  // Navigation is handled by VirtualList internally via AnchorListPane
  useInputCompat({
    id: 'anchor-view',
    priority: InputPriority.CRITICAL,
    description: 'Anchor view full-screen input handler',
    isActive: isVisible,
    handler: (_input, key) => {
      if (key.escape) {
        onClose();
        return true;
      }

      // Let VirtualList handle navigation keys
      if (key.upArrow || key.downArrow || key.pageUp || key.pageDown || key.home || key.end) {
        return false; // Pass to VirtualList
      }

      // Consume all other input to prevent leaks
      return true;
    },
  });

  if (!isVisible) {
    return null;
  }

  // Calculate pane dimensions
  const leftPaneWidth = Math.floor(terminalWidth * LEFT_PANE_RATIO) - 2;
  const rightPaneWidth = terminalWidth - leftPaneWidth - 4;
  const contentHeight = terminalHeight - HEADER_FOOTER_HEIGHT - PANE_CHROME_HEIGHT;

  return (
    <Box
      position="absolute"
      flexDirection="column"
      width={terminalWidth}
      height={terminalHeight}
      backgroundColor="black"
    >
      {/* Header */}
      <Box paddingX={1} justifyContent="space-between">
        <Text bold color="cyan">
          Conversation Anchors
        </Text>
        <Text dimColor>
          {anchorPoints.length > 0
            ? `${anchorPoints.length} anchor${anchorPoints.length === 1 ? '' : 's'}`
            : ''}
        </Text>
        <Text dimColor>ESC</Text>
      </Box>

      {/* Separator */}
      <Box>
        <Text dimColor>{'─'.repeat(terminalWidth)}</Text>
      </Box>

      {/* Main content area */}
      {anchorPoints.length === 0 ? (
        <Box flexGrow={1} paddingX={2} paddingY={1}>
          <Text dimColor italic>
            No anchor points found in this session
          </Text>
        </Box>
      ) : (
        <Box flexGrow={1} flexDirection="row">
          <AnchorListPane
            anchorPoints={anchorPoints}
            width={leftPaneWidth}
            contentHeight={contentHeight}
            onAnchorSelect={handleAnchorSelect}
          />
          <AnchorPreviewPane
            selectedAnchor={selectedAnchor}
            turnDetails={turnDetails}
            isLoading={isLoadingDetails}
            width={rightPaneWidth}
            contentHeight={contentHeight}
          />
        </Box>
      )}

      {/* Footer */}
      <Box>
        <Text dimColor>{'─'.repeat(terminalWidth)}</Text>
      </Box>
      <Box paddingX={1}>
        <Text dimColor>Up/Down Navigate | Esc Close</Text>
      </Box>
    </Box>
  );
}
