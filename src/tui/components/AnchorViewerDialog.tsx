/**
 * AnchorViewerDialog Component
 * 
 * TUI-056: Interactive anchor point viewer with conversation navigation
 * 
 * A modal dialog that displays anchor points from the current session using VirtualList.
 * Supports keyboard navigation, type-based filtering, and detail viewing.
 */

import React, { useState, useCallback, useMemo } from 'react';
import { Box, Text } from 'ink';
import { VirtualList } from './VirtualList';
import { TurnContentModal } from './TurnContentModal';
import { Dialog } from '../../components/Dialog';
import { useInputCompat, InputPriority } from '../input/index';
import type { AnchorPoint, AnchorType, AnchorTurnDetails } from '../types/anchor';

export interface AnchorViewerDialogProps {
  /** Whether the dialog is visible */
  isVisible: boolean;
  /** Array of anchor points to display */
  anchorPoints: AnchorPoint[];
  /** Callback to close the dialog */
  onClose: () => void;
  /** Callback to get turn details for a specific turn index */
  onGetTurnDetails: (turnIndex: number) => Promise<AnchorTurnDetails | null>;
}

// Icons for different anchor types (TUI-056: visual indicators)
const ANCHOR_TYPE_ICONS: Record<AnchorType, string> = {
  ErrorResolution: '🔧',
  TaskCompletion: '✅',
  UserCheckpoint: '📍',
  FeatureMilestone: '🏁',
};

// Type shortcuts for quick navigation (TUI-056: keyboard shortcuts)
const TYPE_SHORTCUTS: Record<string, AnchorType> = {
  e: 'ErrorResolution',
  t: 'TaskCompletion',
  u: 'UserCheckpoint',
  f: 'FeatureMilestone',
};

export const AnchorViewerDialog: React.FC<AnchorViewerDialogProps> = ({
  isVisible,
  anchorPoints,
  onClose,
  onGetTurnDetails,
}) => {
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [showTurnDetails, setShowTurnDetails] = useState(false);
  const [turnDetails, setTurnDetails] = useState<AnchorTurnDetails | null>(null);

  // Reset selection when dialog becomes visible
  React.useEffect(() => {
    if (isVisible) {
      setSelectedIndex(0);
    }
  }, [isVisible]);

  // Format timestamp for display
  const formatTimestamp = useCallback((timestamp: number) => {
    const date = new Date(timestamp);
    return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }, []);

  // Find first anchor of specific type (for keyboard shortcuts)
  const findAnchorByType = useCallback((targetType: AnchorType) => {
    const index = anchorPoints.findIndex(anchor => anchor.anchorType === targetType);
    if (index >= 0) {
      setSelectedIndex(index);
    }
  }, [anchorPoints]);

  // Handle Enter key to view details
  const handleViewDetails = useCallback(async () => {
    if (anchorPoints.length === 0) return;
    
    const anchor = anchorPoints[selectedIndex];
    if (!anchor) return;

    try {
      const details = await onGetTurnDetails(anchor.turnIndex);
      if (details) {
        setTurnDetails(details);
        setShowTurnDetails(true);
      }
    } catch (error) {
      console.error('Failed to get turn details:', error);
    }
  }, [anchorPoints, selectedIndex, onGetTurnDetails]);

  // Handle input for navigation and shortcuts
  useInputCompat({
    id: 'anchor-viewer-dialog',
    priority: InputPriority.CRITICAL,  // Use CRITICAL like AttachmentDialog
    description: 'Anchor viewer dialog navigation',
    isActive: isVisible && !showTurnDetails,
    handler: (input, key) => {
      if (key.return) {
        void handleViewDetails(); // Fire and forget async call
        return true;
      }

      // Type shortcuts (E, T, F, U)
      const lowerInput = input.toLowerCase();
      if (TYPE_SHORTCUTS[lowerInput]) {
        findAnchorByType(TYPE_SHORTCUTS[lowerInput]);
        return true;
      }

      // Arrow keys - let VirtualList handle these
      if (key.upArrow || key.downArrow || key.pageUp || key.pageDown || key.home || key.end) {
        return false;
      }

      // Consume all other input when dialog is visible (like AttachmentDialog)
      return true;
    },
  });

  // Render individual anchor item
  const renderAnchorItem = useCallback((
    anchor: AnchorPoint,
    index: number,
    isSelected: boolean
  ) => {
    const icon = ANCHOR_TYPE_ICONS[anchor.anchorType];
    const timestamp = formatTimestamp(anchor.timestamp);
    
    return (
      <Box key={index}>
        {/* Selection indicator */}
        <Text color={isSelected ? 'cyan' : undefined}>
          {isSelected ? '▸ ' : '  '}
        </Text>
        
        {/* Type icon */}
        <Text>{icon} </Text>
        
        {/* Anchor type and weight */}
        <Box width={20}>
          <Text
            bold={isSelected}
            color={isSelected ? 'white' : 'green'}
            backgroundColor={isSelected ? 'blue' : undefined}
          >
            {anchor.anchorType}
          </Text>
        </Box>
        
        {/* Weight */}
        <Box width={6} marginLeft={1}>
          <Text dimColor={!isSelected}>
            ({anchor.weight.toFixed(1)})
          </Text>
        </Box>
        
        {/* Turn number */}
        <Box width={8} marginLeft={1}>
          <Text dimColor={!isSelected}>
            Turn {anchor.turnIndex}
          </Text>
        </Box>
        
        {/* Timestamp */}
        <Box width={8} marginLeft={1}>
          <Text dimColor={!isSelected}>
            {timestamp}
          </Text>
        </Box>
        
        {/* Description */}
        <Box marginLeft={1}>
          <Text dimColor={!isSelected}>
            {anchor.description}
          </Text>
        </Box>
      </Box>
    );
  }, [formatTimestamp]);

  if (!isVisible) {
    return null;
  }

  // Handle turn details modal
  if (showTurnDetails && turnDetails) {
    // Convert AnchorTurnDetails to format expected by TurnContentModal
    const formattedTurn = {
      type: 'assistant' as const,
      content: turnDetails.assistantResponse || 'No response content',
      timestamp: new Date().toISOString(), // Use current time as fallback
      metadata: {
        turnIndex: turnDetails.turnIndex,
        userMessage: turnDetails.userMessage || 'No user message',
        toolCalls: turnDetails.toolCalls || [],
        fileModifications: turnDetails.fileModifications || [],
        status: turnDetails.status || 'unknown',
        context: turnDetails.context || 'No context',
      },
    };

    return (
      <TurnContentModal
        isVisible={true}
        turn={formattedTurn}
        onClose={() => setShowTurnDetails(false)}
      />
    );
  }

  return (
    <Dialog onClose={onClose} borderColor="cyan" isActive={isVisible}>
      <Box
        flexDirection="column"
        paddingX={2}
        paddingY={1}
        width={90}
        height={25}
      >
        {/* Header */}
        <Box marginBottom={1}>
          <Text bold color="cyan">
            Conversation Anchor Points
          </Text>
          {anchorPoints.length > 0 && (
            <Text dimColor>
              {' '}
              ({anchorPoints.length} anchors found)
            </Text>
          )}
        </Box>

        {/* Separator */}
        <Box marginBottom={1}>
          <Text dimColor>{'─'.repeat(80)}</Text>
        </Box>

        {/* Content */}
        {anchorPoints.length === 0 ? (
          <Box paddingY={2}>
            <Text dimColor italic>
              No anchor points found in this session
            </Text>
          </Box>
        ) : (
          <Box flex={1}>
            <VirtualList
              items={anchorPoints}
              renderItem={renderAnchorItem}
              selectedIndex={selectedIndex}
              onSelectionChange={setSelectedIndex}
              height={15} // Fixed height for consistent display
              keyboardNavigation={true}
              showScrollbar={true}
            />
          </Box>
        )}

        {/* Footer with keyboard hints */}
        <Box marginTop={1}>
          <Text dimColor>{'─'.repeat(80)}</Text>
        </Box>
        <Box>
          <Text dimColor>
            ↑↓ Navigate │ Enter View Details │ E/T/F/U Jump to Type │ Esc Close
          </Text>
        </Box>
      </Box>
    </Dialog>
  );
};