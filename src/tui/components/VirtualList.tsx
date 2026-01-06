import React, {
  useState,
  useEffect,
  useMemo,
  useRef,
  useLayoutEffect,
  useCallback,
  memo,
} from 'react';
import { Box, Text, useInput, measureElement, type DOMElement } from 'ink';
import { logger } from '../../utils/logger';
import { useTerminalSize } from '../hooks/useTerminalSize';

// Unicode characters for scrollbar
const SCROLLBAR_CHARS = {
  square: '■',
  line: '│',
} as const;

// Pre-computed scrollbar string cache - avoids creating new strings on each render
const scrollbarCache = new Map<string, string>();
function getScrollbarString(
  height: number,
  thumbPos: number,
  thumbHeight: number
): string {
  const key = `${height}-${thumbPos}-${thumbHeight}`;
  const cached = scrollbarCache.get(key);
  if (cached !== undefined) {
    return cached;
  }

  const chars: string[] = [];
  for (let i = 0; i < height; i++) {
    if (i >= thumbPos && i < thumbPos + thumbHeight) {
      chars.push(SCROLLBAR_CHARS.square);
    } else {
      chars.push(SCROLLBAR_CHARS.line);
    }
  }
  const result = chars.join('\n');

  // Limit cache size to prevent memory leaks
  if (scrollbarCache.size > 1000) {
    const firstKey = scrollbarCache.keys().next().value;
    if (firstKey) scrollbarCache.delete(firstKey);
  }
  scrollbarCache.set(key, result);
  return result;
}

// Optimized scrollbar component - single Text element instead of N elements
const Scrollbar = memo(function Scrollbar({
  itemCount,
  visibleHeight,
  scrollOffset,
}: {
  itemCount: number;
  visibleHeight: number;
  scrollOffset: number;
}): React.ReactElement | null {
  if (itemCount <= visibleHeight) {
    return null;
  }

  const scrollbarHeight = visibleHeight;
  const thumbHeight = Math.max(
    1,
    Math.floor((visibleHeight / itemCount) * scrollbarHeight)
  );
  const thumbPosition = Math.floor(
    (scrollOffset / itemCount) * scrollbarHeight
  );

  const scrollbarText = getScrollbarString(
    scrollbarHeight,
    thumbPosition,
    thumbHeight
  );

  return (
    <Box flexDirection="column" marginLeft={1}>
      <Text dimColor>{scrollbarText}</Text>
    </Box>
  );
});

interface VirtualListProps<T> {
  items: T[];
  renderItem: (item: T, index: number, isSelected: boolean, selectedIndex: number) => React.ReactNode;
  onSelect?: (item: T, index: number) => void;
  onFocus?: (item: T, index: number) => void;
  keyExtractor?: (item: T, index: number) => string;
  emptyMessage?: string;
  showScrollbar?: boolean;
  enableWrapAround?: boolean;
  reservedLines?: number; // Lines reserved for headers/footers (default: 4)
  isFocused?: boolean; // Whether this VirtualList should respond to keyboard input (default: true)
  scrollToEnd?: boolean; // Auto-scroll to end when items change (default: false)
  selectionMode?: 'item' | 'scroll'; // 'item' = individual item selection (default), 'scroll' = pure viewport scrolling (TUI-032)
  fixedHeight?: number; // Optional fixed height to skip measureElement overhead
  // TUI-042: Custom navigation logic - returns new index when navigating
  // If not provided, uses ±1 (single item navigation)
  getNextIndex?: (currentIndex: number, direction: 'up' | 'down', items: T[]) => number;
  // TUI-042: Custom selection check - returns true if item at index should be highlighted
  // If not provided, uses index === selectedIndex (single item selection)
  getIsSelected?: (index: number, selectedIndex: number, items: T[]) => boolean;
  // TUI-043: Ref to expose selected index to parent component (for /expand command)
  selectionRef?: React.MutableRefObject<{ selectedIndex: number }>;
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
  selectionMode = 'item',
  fixedHeight,
  getNextIndex,
  getIsSelected,
  selectionRef,
}: VirtualListProps<T>): React.ReactElement {
  // Enable mouse tracking mode for button events only (not mouse movement)
  // OPTIMIZATION: Only enable when focused to reduce overhead
  useEffect(() => {
    if (!isFocused) return;

    // \x1b[?1000h enables "button event" tracking (clicks and scroll only)
    process.stdout.write('\x1b[?1000h');

    return () => {
      // Disable mouse tracking mode on unmount
      process.stdout.write('\x1b[?1000l');
    };
  }, [isFocused]);

  const [selectedIndex, setSelectedIndex] = useState(0);
  const [scrollOffset, setScrollOffset] = useState(0);
  const { height: terminalHeight } = useTerminalSize();

  // TUI-043: Update selectionRef when selectedIndex changes
  useEffect(() => {
    if (selectionRef) {
      selectionRef.current = { selectedIndex };
    }
  }, [selectedIndex, selectionRef]);

  // Measure actual container height after flexbox layout
  const containerRef = useRef<DOMElement>(null);
  const [measuredHeight, setMeasuredHeight] = useState<number | null>(null);

  // Track last scroll time for acceleration
  const lastScrollTime = useRef<number>(0);
  const scrollVelocity = useRef<number>(1);

  // Track if user has manually scrolled away from bottom (for sticky scroll behavior)
  // When true, auto-scroll to end is disabled until user scrolls back to bottom
  const [userScrolledAway, setUserScrolledAway] = useState(false);

  // Track if measurement is scheduled to debounce
  const measurementScheduled = useRef(false);

  // Measure container height after layout using Yoga layout engine
  // OPTIMIZATION: Debounce measurements and skip when fixedHeight provided
  useLayoutEffect(() => {
    if (fixedHeight !== undefined) {
      // Skip measurement if fixed height provided
      setMeasuredHeight(fixedHeight);
      return;
    }

    if (!containerRef.current || measurementScheduled.current) return;

    // Debounce measurements using setTimeout(0) to coalesce multiple calls
    measurementScheduled.current = true;
    const timeoutId = setTimeout(() => {
      if (containerRef.current) {
        const dimensions = measureElement(containerRef.current);
        // Use measured height (lines = height, since each item is 1 line in terminal)
        // This respects flexbox layout and gives us the ACTUAL allocated space
        // Floor first to handle fractional heights, then subtract 2 for safety margin
        // This prevents overflow from accumulated rounding errors in nested flexbox containers
        // with borders that may consume partial lines
        if (dimensions.height > 0) {
          const newHeight = Math.max(1, Math.floor(dimensions.height) - 2);
          // Only update if changed to prevent unnecessary re-renders
          setMeasuredHeight(prev => (prev !== newHeight ? newHeight : prev));
        }
      }
      measurementScheduled.current = false;
    }, 0);

    return () => {
      clearTimeout(timeoutId);
      measurementScheduled.current = false;
    };
  }, [terminalHeight, fixedHeight]); // Removed items.length - only re-measure on terminal resize

  // Calculate visible window from measured container height (if available) or terminal size
  // Priority: fixed height > measured container height > terminal height calculation
  // OPTIMIZATION: Memoize visibleHeight calculation
  const visibleHeight = useMemo(() => {
    if (fixedHeight !== undefined) return fixedHeight;
    return measuredHeight !== null
      ? Math.max(1, measuredHeight)
      : Math.max(1, terminalHeight - reservedLines);
  }, [measuredHeight, terminalHeight, reservedLines, fixedHeight]);
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

  // Compute max scroll offset for scroll mode (TUI-032)
  // OPTIMIZATION: Memoize to avoid recalculation
  const maxScrollOffset = useMemo(
    () => Math.max(0, items.length - visibleHeight),
    [items.length, visibleHeight]
  );

  // Direct scroll offset manipulation for scroll mode (TUI-032)
  // This bypasses selection tracking and directly moves the viewport
  // OPTIMIZATION: useCallback for stable reference
  const scrollTo = useCallback(
    (offset: number): void => {
      setScrollOffset(Math.max(0, Math.min(maxScrollOffset, offset)));
    },
    [maxScrollOffset]
  );

  // Auto-scroll to end when items change (for chat-style interfaces)
  // Respects userScrolledAway: if user has scrolled away from bottom, don't auto-scroll
  useEffect(() => {
    if (scrollToEnd && items.length > 0 && !userScrolledAway) {
      const lastIndex = items.length - 1;
      // In scroll mode, only update scrollOffset (TUI-032)
      // In item mode, also update selectedIndex
      if (selectionMode === 'item') {
        setSelectedIndex(lastIndex);
      }
      // Scroll to show the last item at the bottom
      const newOffset = Math.max(0, lastIndex - visibleHeight + 1);
      setScrollOffset(newOffset);
    }
  }, [scrollToEnd, items.length, visibleHeight, selectionMode, userScrolledAway]);

  // Adjust scroll offset to keep selected item visible (TUI-032: only in item mode)
  // In scroll mode, selection doesn't drive scrolling - viewport scrolls independently
  useEffect(() => {
    if (selectionMode === 'item') {
      if (selectedIndex < scrollOffset) {
        setScrollOffset(selectedIndex);
      } else if (selectedIndex >= scrollOffset + visibleHeight) {
        setScrollOffset(selectedIndex - visibleHeight + 1);
      }
    }
  }, [selectedIndex, scrollOffset, visibleHeight, selectionMode]);

  // Call onFocus when selection changes (TUI-032: only in item mode)
  // In scroll mode, no item is ever focused so onFocus is never called
  useEffect(() => {
    if (selectionMode === 'item') {
      if (
        items.length > 0 &&
        selectedIndex >= 0 &&
        selectedIndex < items.length
      ) {
        onFocus?.(items[selectedIndex], selectedIndex);
      }
    }
  }, [selectedIndex, items, onFocus, selectionMode]);

  const navigateTo = useCallback(
    (newIndex: number): void => {
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
    },
    [items.length, enableWrapAround]
  );

  // Mouse scroll handler with acceleration (TUI-032: uses scrollTo in scroll mode)
  // Scroll faster when scrolling rapidly, slower for precise positioning
  // Also tracks userScrolledAway state for sticky scroll behavior
  // OPTIMIZATION: useCallback for stable reference
  const handleScroll = useCallback(
    (direction: 'up' | 'down'): void => {
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
      const delta = direction === 'down' ? scrollAmount : -scrollAmount;

      if (selectionMode === 'scroll') {
        // In scroll mode, directly adjust scrollOffset (TUI-032)
        const newOffset = Math.max(0, Math.min(maxScrollOffset, scrollOffset + delta));
        setScrollOffset(newOffset);
        
        // Track if user scrolled away from bottom (for sticky scroll behavior)
        // If scrolling up and not at bottom, mark as scrolled away
        // If at bottom (or within 1 line), re-enable auto-scroll
        if (scrollToEnd) {
          const isAtBottom = newOffset >= maxScrollOffset - 1;
          if (direction === 'up' && !isAtBottom) {
            setUserScrolledAway(true);
          } else if (isAtBottom) {
            setUserScrolledAway(false);
          }
        }
      } else {
        // In item mode, navigate through items
        // TUI-042: Use custom navigation if provided (e.g., for turn-based selection)
        if (getNextIndex) {
          const newIndex = getNextIndex(selectedIndex, direction, items);
          navigateTo(newIndex);
        } else {
          // Default: move by delta (with acceleration)
          navigateTo(selectedIndex + delta);
        }
      }
    },
    [scrollOffset, selectedIndex, selectionMode, maxScrollOffset, navigateTo, scrollToEnd, getNextIndex, items]
  );

  // Mouse scroll handling - respects isFocused to only scroll the focused list
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
    { isActive: isFocused }
  );

  // Scroll mode navigation handler (TUI-032)
  // Also tracks userScrolledAway state for sticky scroll behavior
  const handleScrollNavigation = (key: {
    upArrow?: boolean;
    downArrow?: boolean;
    pageUp?: boolean;
    pageDown?: boolean;
    home?: boolean;
    end?: boolean;
  }): void => {
    let newOffset = scrollOffset;
    let isScrollingUp = false;
    
    if (key.upArrow) {
      newOffset = Math.max(0, scrollOffset - 1);
      isScrollingUp = true;
    } else if (key.downArrow) {
      newOffset = Math.min(maxScrollOffset, scrollOffset + 1);
    } else if (key.pageUp) {
      newOffset = Math.max(0, scrollOffset - visibleHeight);
      isScrollingUp = true;
    } else if (key.pageDown) {
      newOffset = Math.min(maxScrollOffset, scrollOffset + visibleHeight);
    } else if (key.home) {
      newOffset = 0;
      isScrollingUp = true;
    } else if (key.end) {
      newOffset = maxScrollOffset;
    }
    
    setScrollOffset(newOffset);
    
    // Track if user scrolled away from bottom (for sticky scroll behavior)
    if (scrollToEnd) {
      const isAtBottom = newOffset >= maxScrollOffset - 1;
      if (isScrollingUp && !isAtBottom) {
        setUserScrolledAway(true);
      } else if (isAtBottom) {
        setUserScrolledAway(false);
      }
    }
  };

  // Item mode navigation handler
  // TUI-042: Uses getNextIndex for custom navigation (e.g., turn-based selection)
  const handleItemNavigation = (key: {
    upArrow?: boolean;
    downArrow?: boolean;
    pageUp?: boolean;
    pageDown?: boolean;
    home?: boolean;
    end?: boolean;
    return?: boolean;
  }): void => {
    if (key.upArrow) {
      if (getNextIndex) {
        navigateTo(getNextIndex(selectedIndex, 'up', items));
      } else {
        navigateTo(selectedIndex - 1);
      }
    } else if (key.downArrow) {
      if (getNextIndex) {
        navigateTo(getNextIndex(selectedIndex, 'down', items));
      } else {
        navigateTo(selectedIndex + 1);
      }
    } else if (key.pageUp) {
      // Page up/down always use line-based navigation for now
      navigateTo(Math.max(0, selectedIndex - visibleHeight));
    } else if (key.pageDown) {
      navigateTo(Math.min(items.length - 1, selectedIndex + visibleHeight));
    } else if (key.home) {
      navigateTo(0);
    } else if (key.end) {
      navigateTo(items.length - 1);
    } else if (key.return && onSelect) {
      onSelect(items[selectedIndex], selectedIndex);
    }
  };

  // Keyboard navigation - respects isFocused
  useInput(
    (input, key) => {
      // Log ALL keyboard input in VirtualList for debugging
      logger.info(`[VirtualList] useInput called: input=${JSON.stringify(input)} (length=${input.length}) key=${JSON.stringify(key)} isFocused=${isFocused} selectionMode=${selectionMode}`);
      
      if (items.length === 0) {
        logger.info('[VirtualList] No items, returning');
        return;
      }

      // Skip mouse events (handled above)
      if (input.startsWith('[M') || key.mouse) {
        logger.info('[VirtualList] Skipping mouse event');
        return;
      }

      // Skip Shift+Arrow (used for history navigation in AgentView)
      if (key.shift && (key.upArrow || key.downArrow)) {
        logger.info('[VirtualList] Skipping shift+arrow (history)');
        return;
      }

      if (selectionMode === 'scroll') {
        logger.info('[VirtualList] Handling scroll navigation');
        handleScrollNavigation(key);
      } else {
        logger.info('[VirtualList] Handling item navigation');
        handleItemNavigation(key);
      }
    },
    { isActive: isFocused }
  );

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
          // In scroll mode, isSelected is always false (TUI-032)
          // TUI-042: Use custom getIsSelected if provided (e.g., for turn-based selection)
          let isSelected: boolean;
          if (selectionMode !== 'item') {
            isSelected = false;
          } else if (getIsSelected) {
            isSelected = getIsSelected(actualIndex, selectedIndex, items);
          } else {
            isSelected = actualIndex === selectedIndex;
          }
          return (
            <Box key={keyExtractor(item, actualIndex)}>
              {renderItem(item, actualIndex, isSelected, selectedIndex)}
            </Box>
          );
        })}
      </Box>
      {showScrollbar && (
        <Scrollbar
          itemCount={items.length}
          visibleHeight={visibleHeight}
          scrollOffset={scrollOffset}
        />
      )}
    </Box>
  );
}
