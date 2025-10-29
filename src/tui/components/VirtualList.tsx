import React, { useState, useEffect, useMemo } from 'react';
import { Box, Text, useInput } from 'ink';
import { useTerminalSize } from '../hooks/useTerminalSize';

// Unicode characters for scrollbar
const SCROLLBAR_CHARS = {
  square: '■',
  line: '│',
};

interface VirtualListProps<T> {
  items: T[];
  renderItem: (item: T, index: number, isSelected: boolean) => React.ReactNode;
  onSelect?: (item: T, index: number) => void;
  onFocus?: (item: T, index: number) => void;
  keyExtractor?: (item: T, index: number) => string;
  emptyMessage?: string;
  showScrollbar?: boolean;
  enableWrapAround?: boolean;
  reservedLines?: number; // Lines reserved for headers/footers (default: 4)
}

export function VirtualList<T>({
  items,
  renderItem,
  onSelect,
  onFocus,
  keyExtractor = (_item: T, index: number) => String(index),
  emptyMessage = 'No items',
  showScrollbar = true,
  enableWrapAround = false,
  reservedLines = 4,
}: VirtualListProps<T>): React.ReactElement {
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [scrollOffset, setScrollOffset] = useState(0);
  const { height: terminalHeight } = useTerminalSize();

  // Calculate visible window from terminal size
  const visibleHeight = Math.max(1, terminalHeight - reservedLines);
  const visibleItems = useMemo(() => {
    const start = scrollOffset;
    const end = Math.min(items.length, scrollOffset + visibleHeight);
    return items.slice(start, end);
  }, [items, scrollOffset, visibleHeight]);

  // Reset selection if items change
  useEffect(() => {
    if (items.length > 0 && selectedIndex >= items.length) {
      setSelectedIndex(Math.max(0, items.length - 1));
    }
  }, [items.length, selectedIndex]);

  // Adjust scroll offset to keep selected item visible
  useEffect(() => {
    if (selectedIndex < scrollOffset) {
      setScrollOffset(selectedIndex);
    } else if (selectedIndex >= scrollOffset + visibleHeight) {
      setScrollOffset(selectedIndex - visibleHeight + 1);
    }
  }, [selectedIndex, scrollOffset, visibleHeight]);

  // Call onFocus when selection changes
  useEffect(() => {
    if (items.length > 0 && selectedIndex >= 0 && selectedIndex < items.length) {
      onFocus?.(items[selectedIndex], selectedIndex);
    }
  }, [selectedIndex, items, onFocus]);

  const navigateTo = (newIndex: number): void => {
    if (items.length === 0) {
      return;
    }

    let targetIndex = newIndex;

    if (enableWrapAround) {
      if (targetIndex < 0) {
        targetIndex = items.length - 1;
      } else if (targetIndex >= items.length) {
        targetIndex = 0;
      }
    } else {
      targetIndex = Math.max(0, Math.min(items.length - 1, targetIndex));
    }

    setSelectedIndex(targetIndex);
  };

  useInput((input, key) => {
    if (items.length === 0) {
      return;
    }

    // Navigation
    if (key.upArrow || input === 'k') {
      navigateTo(selectedIndex - 1);
    } else if (key.downArrow || input === 'j') {
      navigateTo(selectedIndex + 1);
    } else if (key.pageUp) {
      navigateTo(Math.max(0, selectedIndex - visibleHeight));
    } else if (key.pageDown) {
      navigateTo(Math.min(items.length - 1, selectedIndex + visibleHeight));
    } else if (key.home || input === 'g') {
      navigateTo(0);
    } else if (key.end || input === 'G') {
      navigateTo(items.length - 1);
    } else if (key.return && onSelect) {
      onSelect(items[selectedIndex], selectedIndex);
    }
  });

  // Render scrollbar
  const renderScrollbar = (): React.ReactNode => {
    if (!showScrollbar || items.length <= visibleHeight) {
      return null;
    }

    const scrollbarHeight = visibleHeight;
    const thumbHeight = Math.max(1, Math.floor((visibleHeight / items.length) * scrollbarHeight));
    const thumbPosition = Math.floor((scrollOffset / items.length) * scrollbarHeight);

    const scrollbarChars: string[] = [];
    for (let i = 0; i < scrollbarHeight; i++) {
      if (i >= thumbPosition && i < thumbPosition + thumbHeight) {
        scrollbarChars.push(SCROLLBAR_CHARS.square);
      } else {
        scrollbarChars.push(SCROLLBAR_CHARS.line);
      }
    }

    return (
      <Box flexDirection="column" marginLeft={1}>
        {scrollbarChars.map((char, i) => (
          <Text key={i} dimColor>
            {char}
          </Text>
        ))}
      </Box>
    );
  };

  if (items.length === 0) {
    return (
      <Box flexGrow={1} flexDirection="column">
        <Text dimColor>{emptyMessage}</Text>
      </Box>
    );
  }

  return (
    <Box flexDirection="row" flexGrow={1}>
      <Box flexDirection="column" flexGrow={1}>
        {visibleItems.map((item, visibleIndex) => {
          const actualIndex = scrollOffset + visibleIndex;
          const isSelected = actualIndex === selectedIndex;
          return (
            <Box key={keyExtractor(item, actualIndex)}>
              {renderItem(item, actualIndex, isSelected)}
            </Box>
          );
        })}
      </Box>
      {renderScrollbar()}
    </Box>
  );
}
