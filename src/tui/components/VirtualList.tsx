import React, { useState, useEffect, useMemo } from 'react';
import { Box, Text, useInput } from 'ink';
import { useTerminalSize } from '../hooks/useTerminalSize';
import { logger } from '../../utils/logger';

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
  isFocused?: boolean; // Whether this VirtualList should respond to keyboard input (default: true)
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
  isFocused = true,
}: VirtualListProps<T>): React.ReactElement {
  // Log component mount and props
  useEffect(() => {
    logger.info(`[VirtualList] Component mounted - items: ${items.length}, isFocused: ${isFocused}, showScrollbar: ${showScrollbar}`);

    // Enable mouse tracking mode for button events only (not mouse movement)
    // \x1b[?1000h enables "button event" tracking (clicks and scroll only)
    // \x1b[?1002h would include drag events
    // \x1b[?1003h includes all movements (too noisy)
    process.stdout.write('\x1b[?1000h');
    logger.info(`[VirtualList] Enabled mouse button tracking mode`);

    return () => {
      // Disable mouse tracking mode on unmount
      process.stdout.write('\x1b[?1000l');
      logger.info(`[VirtualList] Disabled mouse button tracking mode`);
      logger.info(`[VirtualList] Component unmounting`);
    };
  }, []);

  // Log when focus changes
  useEffect(() => {
    logger.info(`[VirtualList] Focus changed - isFocused: ${isFocused}`);
  }, [isFocused]);

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
    logger.debug(`[VirtualList] navigateTo called - newIndex: ${newIndex}, current: ${selectedIndex}, items.length: ${items.length}`);

    if (items.length === 0) {
      logger.info(`[VirtualList] navigateTo aborted - no items`);
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

    logger.info(`[VirtualList] Setting selectedIndex to ${targetIndex} (from ${selectedIndex})`);
    setSelectedIndex(targetIndex);
  };

  // Mouse scroll handler (no throttling - React's render batching handles this)
  const handleScroll = (direction: 'up' | 'down'): void => {
    logger.info(`[VirtualList] Scroll handler fired - direction: ${direction}, selectedIndex: ${selectedIndex}`);
    if (direction === 'down') {
      logger.info(`[VirtualList] Scrolling DOWN - navigating from ${selectedIndex} to ${selectedIndex + 1}`);
      navigateTo(selectedIndex + 1);
    } else if (direction === 'up') {
      logger.info(`[VirtualList] Scrolling UP - navigating from ${selectedIndex} to ${selectedIndex - 1}`);
      navigateTo(selectedIndex - 1);
    }
  };

  useInput(
    (input, key) => {
      // DEBUG: Log ALL input events
      logger.info(`[VirtualList] Input event - input: "${input}", key: ${JSON.stringify(key)}`);

      if (items.length === 0) {
        logger.info(`[VirtualList] Ignoring input - items array is empty`);
        return;
      }

      // Parse raw mouse escape sequences manually (for terminals where Ink doesn't parse them)
      // Format: ESC[M<btn><x><y> where btn encodes button and action
      if (input.startsWith('[M')) {
        // Extract button byte (3rd character after ESC[M)
        const buttonByte = input.charCodeAt(2);
        logger.info(`[VirtualList] Raw mouse escape sequence detected - buttonByte: ${buttonByte} (char: '${String.fromCharCode(buttonByte)}')`);

        // Button encoding for scroll wheel (standard xterm):
        // Button codes: 64 = scroll up, 65 = scroll down
        // With 32-byte offset in escape sequence: 96 = scroll up, 97 = scroll down
        if (buttonByte === 96) { // ASCII 96 = '`' = button 64 = scroll up
          logger.info(`[VirtualList] Scroll UP detected from escape sequence`);
          handleScroll('up');
          return;
        } else if (buttonByte === 97) { // ASCII 97 = 'a' = button 65 = scroll down
          logger.info(`[VirtualList] Scroll DOWN detected from escape sequence`);
          handleScroll('down');
          return;
        } else {
          logger.debug(`[VirtualList] Mouse event with unhandled buttonByte: ${buttonByte}`);
        }
      }

      // Mouse scroll handling (key.mouse exists for mouse events - when Ink parses them)
      if (key.mouse) {
        logger.info(`[VirtualList] Mouse event detected - button: ${key.mouse.button}, action: ${key.mouse.action}, x: ${key.mouse.x}, y: ${key.mouse.y}`);

        // Scroll events detected when button === 'none' (per Ink mouse event docs)
        // Traditional scrolling: scroll down shows items below, scroll up shows items above
        if (key.mouse.button === 'wheelDown') {
          logger.info(`[VirtualList] Mouse wheel DOWN detected - calling scroll handler`);
          handleScroll('down');
          return;
        } else if (key.mouse.button === 'wheelUp') {
          logger.info(`[VirtualList] Mouse wheel UP detected - calling scroll handler`);
          handleScroll('up');
          return;
        } else {
          logger.info(`[VirtualList] Mouse event with unhandled button: ${key.mouse.button}`);
        }
      } else {
        logger.debug(`[VirtualList] Non-mouse event - input: "${input}"`);
      }

      // Keyboard Navigation
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
    },
    { isActive: isFocused }
  );

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
