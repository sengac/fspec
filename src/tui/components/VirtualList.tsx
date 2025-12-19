import React, {
  useState,
  useEffect,
  useMemo,
  useRef,
  useLayoutEffect,
} from 'react';
import { Box, Text, useInput, measureElement, type DOMElement } from 'ink';
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
  isFocused?: boolean; // Whether this VirtualList should respond to keyboard input (default: true)
  scrollToEnd?: boolean; // Auto-scroll to end when items change (default: false)
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
  scrollToEnd = false,
}: VirtualListProps<T>): React.ReactElement {
  // Enable mouse tracking mode for button events only (not mouse movement)
  useEffect(() => {
    // \x1b[?1000h enables "button event" tracking (clicks and scroll only)
    process.stdout.write('\x1b[?1000h');

    return () => {
      // Disable mouse tracking mode on unmount
      process.stdout.write('\x1b[?1000l');
    };
  }, []);

  const [selectedIndex, setSelectedIndex] = useState(0);
  const [scrollOffset, setScrollOffset] = useState(0);
  const { height: terminalHeight } = useTerminalSize();

  // Measure actual container height after flexbox layout
  const containerRef = useRef<DOMElement>(null);
  const [measuredHeight, setMeasuredHeight] = useState<number | null>(null);

  // Track last scroll time for acceleration
  const lastScrollTime = useRef<number>(0);
  const scrollVelocity = useRef<number>(0);

  // Measure container height after layout using Yoga layout engine
  useLayoutEffect(() => {
    if (containerRef.current) {
      const dimensions = measureElement(containerRef.current);
      // Use measured height (lines = height, since each item is 1 line in terminal)
      // This respects flexbox layout and gives us the ACTUAL allocated space
      // Floor first to handle fractional heights, then subtract 2 for safety margin
      // This prevents overflow from accumulated rounding errors in nested flexbox containers
      // with borders that may consume partial lines
      if (dimensions.height > 0) {
        setMeasuredHeight(Math.max(1, Math.floor(dimensions.height) - 2));
      }
    }
  }, [items.length, terminalHeight]); // Re-measure when items or terminal changes

  // Calculate visible window from measured container height (if available) or terminal size
  // Priority: measured container height > terminal height calculation
  const visibleHeight =
    measuredHeight !== null
      ? Math.max(1, measuredHeight)
      : Math.max(1, terminalHeight - reservedLines);
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

  // Auto-scroll to end when items change (for chat-style interfaces)
  useEffect(() => {
    if (scrollToEnd && items.length > 0) {
      const lastIndex = items.length - 1;
      setSelectedIndex(lastIndex);
      // Scroll to show the last item at the bottom
      const newOffset = Math.max(0, lastIndex - visibleHeight + 1);
      setScrollOffset(newOffset);
    }
  }, [scrollToEnd, items.length, visibleHeight]);

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
    if (
      items.length > 0 &&
      selectedIndex >= 0 &&
      selectedIndex < items.length
    ) {
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

  // Mouse scroll handler with acceleration
  // Scroll faster when scrolling rapidly, slower for precise positioning
  const handleScroll = (direction: 'up' | 'down'): void => {
    const now = Date.now();
    const timeDelta = now - lastScrollTime.current;

    // If scrolling within 150ms, increase velocity (acceleration)
    // If paused longer, reset to base speed for precise control
    if (timeDelta < 150) {
      scrollVelocity.current = Math.min(scrollVelocity.current + 1, 5); // Max 5 items
    } else {
      scrollVelocity.current = 1; // Reset to 1 item for precise control
    }

    lastScrollTime.current = now;
    const scrollAmount = scrollVelocity.current;

    if (direction === 'down') {
      navigateTo(selectedIndex + scrollAmount);
    } else if (direction === 'up') {
      navigateTo(selectedIndex - scrollAmount);
    }
  };

  // Mouse scroll handling - always active (separate from keyboard focus)
  useInput(
    (input, key) => {
      if (items.length === 0) {
        return;
      }

      // Parse raw mouse escape sequences manually (for terminals where Ink doesn't parse them)
      // Format: ESC[M<btn><x><y> where btn encodes button and action
      if (input.startsWith('[M')) {
        const buttonByte = input.charCodeAt(2);

        // Button encoding for scroll wheel (standard xterm):
        // Button codes: 64 = scroll up, 65 = scroll down
        // With 32-byte offset in escape sequence: 96 = scroll up, 97 = scroll down
        if (buttonByte === 96) {
          // ASCII 96 = '`' = button 64 = scroll up
          handleScroll('up');
          return;
        } else if (buttonByte === 97) {
          // ASCII 97 = 'a' = button 65 = scroll down
          handleScroll('down');
          return;
        }
      }

      // Mouse scroll handling (key.mouse exists for mouse events - when Ink parses them)
      if (key.mouse) {
        // Traditional scrolling: scroll down shows items below, scroll up shows items above
        if (key.mouse.button === 'wheelDown') {
          handleScroll('down');
          return;
        } else if (key.mouse.button === 'wheelUp') {
          handleScroll('up');
          return;
        }
      }
    },
    { isActive: true } // Mouse always active
  );

  // Keyboard navigation - respects isFocused
  useInput(
    (input, key) => {
      if (items.length === 0) {
        return;
      }

      // Skip mouse events (handled above)
      if (input.startsWith('[M') || key.mouse) {
        return;
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
    const thumbHeight = Math.max(
      1,
      Math.floor((visibleHeight / items.length) * scrollbarHeight)
    );
    const thumbPosition = Math.floor(
      (scrollOffset / items.length) * scrollbarHeight
    );

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
      <Box ref={containerRef} flexGrow={1} flexDirection="column">
        <Text dimColor>{emptyMessage}</Text>
      </Box>
    );
  }

  return (
    <Box ref={containerRef} flexDirection="row" flexGrow={1}>
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
