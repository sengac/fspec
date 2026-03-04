/**
 * ModelSelectorView - Model Selection Screen (Presentational)
 *
 * TUI-073: Refactored to be purely presentational.
 * All state and keyboard handling is managed by ModelSelectorScreen.
 *
 * Features:
 * - Hierarchical view with collapsible provider sections
 * - Profile sections appear alongside cloud providers (e.g., "openai: work-vllm")
 * - Filter models by name
 * - Shows model capabilities: [R] reasoning, [V] vision
 * - Shows context window size
 */

import React from 'react';
import { Box, Text } from 'ink';
import type { ModelSelectorItem } from '../types/provider';

/**
 * Props for ModelSelectorView (presentational - receives all state from parent)
 */
export interface ModelSelectorViewProps {
  /** Terminal width */
  width: number;
  /** Terminal height */
  height: number;
  /** Flattened list of items to render */
  flatItems: ModelSelectorItem[];
  /** Currently selected section index */
  selectedSectionIdx: number;
  /** Currently selected model index (-1 = section header) */
  selectedModelIdx: number;
  /** Set of expanded provider IDs */
  expandedProviders: Set<string>;
  /** Current scroll offset */
  scrollOffset: number;
  /** Number of visible rows */
  visibleHeight: number;
  /** Current filter string */
  filter: string;
  /** Whether in filter mode */
  isFilterMode: boolean;
  /** Currently selected model ID (for highlighting) */
  currentModelId?: string;
  /** Whether models are being refreshed */
  isRefreshing: boolean;
}

/**
 * Format context window for display
 */
function formatContextWindow(tokens: number): string {
  if (tokens >= 1000000) {
    return `${(tokens / 1000000).toFixed(1)}M`;
  }
  if (tokens >= 1000) {
    return `${Math.round(tokens / 1000)}k`;
  }
  return String(tokens);
}

/**
 * Extract model ID for display (remove provider prefix if present)
 */
function extractModelIdForDisplay(modelId: string): string {
  // Remove common prefixes like "openai/", "anthropic/", etc.
  const slashIdx = modelId.indexOf('/');
  if (slashIdx > 0 && slashIdx < 20) {
    return modelId.slice(slashIdx + 1);
  }
  return modelId;
}

/**
 * Find current flat index from selection state
 */
function findCurrentFlatIndex(
  flatItems: ModelSelectorItem[],
  selectedSectionIdx: number,
  selectedModelIdx: number
): number {
  return flatItems.findIndex(item => {
    if (selectedModelIdx === -1) {
      return item.type === 'section' && item.sectionIdx === selectedSectionIdx;
    }
    return (
      item.type === 'model' &&
      item.sectionIdx === selectedSectionIdx &&
      item.modelIdx === selectedModelIdx
    );
  });
}

/**
 * ModelSelectorView Component (Presentational)
 */
export function ModelSelectorView({
  width,
  height,
  flatItems,
  selectedSectionIdx,
  selectedModelIdx,
  scrollOffset,
  visibleHeight,
  filter,
  isFilterMode,
  currentModelId,
  isRefreshing,
}: ModelSelectorViewProps): React.ReactElement {
  // Calculate content width
  const contentWidth = width - 4 - 3;

  // Find current selection in flat list
  const currentFlatIdx = findCurrentFlatIndex(
    flatItems,
    selectedSectionIdx,
    selectedModelIdx
  );

  // Render
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
          <Text bold color="cyan">
            Select Model
          </Text>
          {isRefreshing && <Text color="yellow"> (refreshing...)</Text>}
          <Text dimColor>
            {' '}
            ({flatItems.filter(i => i.type === 'model').length} models)
          </Text>
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
            {flatItems
              .slice(scrollOffset, scrollOffset + visibleHeight)
              .map((item, visibleIdx) => {
                const actualIdx = scrollOffset + visibleIdx;
                const isSelected = actualIdx === currentFlatIdx;

                if (item.type === 'section') {
                  const icon = item.isExpanded ? '▼' : '▶';
                  const isProfile = !!item.section.profileName;
                  const displayName = item.section.providerName;
                  const modelCount = item.section.models.length;

                  return (
                    <Box
                      key={`section-${item.section.providerId}-${item.section.profileName || 'cloud'}`}
                      width={contentWidth}
                    >
                      <Text
                        backgroundColor={isSelected ? 'cyan' : undefined}
                        color={isSelected ? 'black' : 'white'}
                        wrap="truncate"
                      >
                        {isSelected ? '> ' : '  '}
                        {icon}{' '}
                        {isProfile && (
                          <Text color={isSelected ? 'black' : 'magenta'}>
                            📁{' '}
                          </Text>
                        )}
                        {displayName}
                        <Text dimColor={!isSelected}>
                          {' '}
                          ({modelCount} model{modelCount !== 1 ? 's' : ''})
                        </Text>
                        {item.section.isUnreachable && (
                          <Text color={isSelected ? 'black' : 'red'}>
                            {' '}
                            (unreachable)
                          </Text>
                        )}
                      </Text>
                    </Box>
                  );
                }

                // Model item
                const isCurrent = currentModelId === item.model.id;
                const modelDisplay = extractModelIdForDisplay(item.model.id);

                return (
                  <Box key={`model-${item.model.id}`} width={contentWidth}>
                    <Text
                      backgroundColor={isSelected ? 'cyan' : undefined}
                      color={isSelected ? 'black' : 'white'}
                      wrap="truncate"
                    >
                      {isSelected ? '  > ' : '    '}
                      {modelDisplay}
                      {item.model.reasoning && (
                        <Text color={isSelected ? 'black' : 'magenta'}>
                          {' '}
                          [R]
                        </Text>
                      )}
                      {item.model.hasVision && (
                        <Text color={isSelected ? 'black' : 'blue'}> [V]</Text>
                      )}
                      <Text color={isSelected ? 'black' : 'gray'}>
                        {' '}
                        [{formatContextWindow(item.model.contextWindow)}]
                      </Text>
                      {isCurrent && (
                        <Text color={isSelected ? 'black' : 'green'}>
                          {' '}
                          (current)
                        </Text>
                      )}
                    </Text>
                  </Box>
                );
              })}
          </Box>

          {/* Scrollbar */}
          {flatItems.length > visibleHeight && (
            <Box flexDirection="column" marginLeft={1}>
              {Array.from({ length: visibleHeight }).map((_, i) => {
                const thumbHeight = Math.max(
                  1,
                  Math.floor((visibleHeight / flatItems.length) * visibleHeight)
                );
                const thumbPos = Math.floor(
                  (scrollOffset / flatItems.length) * visibleHeight
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
            {
              'Enter: select | ←→: collapse/expand | r: refresh | Tab: Switch to providers | / filter | Esc: close'
            }
          </Text>
        </Box>

        {/* Legend */}
        <Box marginTop={0}>
          <Text dimColor>
            [R] Reasoning | [V] Vision | 📁 Profile (local server)
          </Text>
        </Box>
      </Box>
    </Box>
  );
}
