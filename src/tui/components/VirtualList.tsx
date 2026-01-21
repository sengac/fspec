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
  reservedLines?: number;
  isFocused?: boolean;
  scrollToEnd?: boolean;
  selectionMode?: 'item' | 'scroll';
  fixedHeight?: number;
  
  // TUI-042/043/044: Group-based selection (for turn selection)
  // When provided, items are grouped by the returned identifier.
  // Navigation moves between groups, selection highlights entire group,
  // and selection is preserved by group ID when content changes.
  groupBy?: (item: T) => string | number;
  
  // Extra lines to include before the group when scrolling (for separator bars)
  groupPaddingBefore?: number;
  
  // Ref to expose selected index to parent (for /expand command)
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
  groupBy,
  groupPaddingBefore = 0,
  selectionRef,
}: VirtualListProps<T>): React.ReactElement {
  // Enable mouse tracking mode for button events only (not mouse movement)
  useEffect(() => {
    if (!isFocused) return;
    process.stdout.write('\x1b[?1000h');
    return () => {
      process.stdout.write('\x1b[?1000l');
    };
  }, [isFocused]);

  const [selectedIndex, setSelectedIndex] = useState(0);
  const [scrollOffset, setScrollOffset] = useState(0);
  const { height: terminalHeight } = useTerminalSize();
  
  // Track previous selectionMode to detect transitions
  const prevSelectionModeRef = useRef(selectionMode);

  // Track selected group ID to preserve selection when content changes
  const selectedGroupIdRef = useRef<string | number | null>(null);

  // Update selectionRef and track group ID
  useEffect(() => {
    if (selectionRef) {
      selectionRef.current = { selectedIndex };
    }
    if (groupBy && items[selectedIndex]) {
      selectedGroupIdRef.current = groupBy(items[selectedIndex]);
    }
  }, [selectedIndex, selectionRef, groupBy, items]);

  // Preserve selection by group ID when items change
  useEffect(() => {
    if (!groupBy || selectedGroupIdRef.current === null || items.length === 0) return;
    
    const targetGroupId = selectedGroupIdRef.current;
    // Find first item with same group ID
    for (let i = 0; i < items.length; i++) {
      if (groupBy(items[i]) === targetGroupId) {
        if (i !== selectedIndex) {
          setSelectedIndex(i);
        }
        return;
      }
    }
  }, [items, groupBy, selectedIndex]);

  // Measure container height
  const containerRef = useRef<DOMElement>(null);
  const [measuredHeight, setMeasuredHeight] = useState<number | null>(null);
  const lastScrollTime = useRef<number>(0);
  const scrollVelocity = useRef<number>(1);
  const [userScrolledAway, setUserScrolledAway] = useState(false);
  const measurementScheduled = useRef(false);

  useLayoutEffect(() => {
    if (fixedHeight !== undefined) {
      setMeasuredHeight(fixedHeight);
      return;
    }
    if (!containerRef.current || measurementScheduled.current) return;
    measurementScheduled.current = true;
    const timeoutId = setTimeout(() => {
      if (containerRef.current) {
        const dimensions = measureElement(containerRef.current);
        if (dimensions.height > 0) {
          const newHeight = Math.max(1, Math.floor(dimensions.height));
          setMeasuredHeight(prev => (prev !== newHeight ? newHeight : prev));
        }
      }
      measurementScheduled.current = false;
    }, 0);
    return () => {
      clearTimeout(timeoutId);
      measurementScheduled.current = false;
    };
  }, [terminalHeight, fixedHeight]);

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

  // Clamp selection if items shrink
  useEffect(() => {
    if (items.length > 0 && selectedIndex >= items.length) {
      setSelectedIndex(Math.max(0, items.length - 1));
    }
  }, [items.length, selectedIndex]);

  // Select last item when transitioning from scroll mode to item mode (with scrollToEnd)
  // This ensures the last turn is selected when entering turn selection mode
  // Note: Don't set scrollOffset here - the scroll adjustment effect (below) will
  // properly position the viewport using getVisibleRange which accounts for groupPaddingBefore
  useEffect(() => {
    const wasScrollMode = prevSelectionModeRef.current === 'scroll';
    prevSelectionModeRef.current = selectionMode;
    
    if (wasScrollMode && selectionMode === 'item' && scrollToEnd && items.length > 0) {
      setSelectedIndex(items.length - 1);
    }
  }, [selectionMode, scrollToEnd, items.length]);

  const maxScrollOffset = useMemo(
    () => Math.max(0, items.length - visibleHeight),
    [items.length, visibleHeight]
  );

  // Auto-scroll to end (only in scroll mode to preserve selection in item mode)
  useEffect(() => {
    if (scrollToEnd && items.length > 0 && !userScrolledAway && selectionMode !== 'item') {
      const lastIndex = items.length - 1;
      const newOffset = Math.max(0, lastIndex - visibleHeight + 1);
      setScrollOffset(newOffset);
    }
  }, [scrollToEnd, items.length, visibleHeight, selectionMode, userScrolledAway]);

  // Get visible range for current selection (including group padding)
  const getVisibleRange = useCallback((index: number): [number, number] => {
    if (!groupBy || items.length === 0) {
      return [index, index];
    }
    
    const groupId = items[index] ? groupBy(items[index]) : null;
    if (groupId === null) return [index, index];
    
    // Find first and last item in group
    let rangeStart = index;
    let rangeEnd = index;
    
    for (let i = index - 1; i >= 0; i--) {
      if (groupBy(items[i]) === groupId) {
        rangeStart = i;
      } else {
        break;
      }
    }
    
    for (let i = index + 1; i < items.length; i++) {
      if (groupBy(items[i]) === groupId) {
        rangeEnd = i;
      } else {
        break;
      }
    }
    
    // Apply padding before (for separator lines)
    rangeStart = Math.max(0, rangeStart - groupPaddingBefore);
    
    return [rangeStart, rangeEnd];
  }, [groupBy, items, groupPaddingBefore]);

  // Adjust scroll to keep selection visible
  useEffect(() => {
    if (selectionMode !== 'item') return;
    
    const [rangeStart, rangeEnd] = getVisibleRange(selectedIndex);
    
    if (rangeStart < scrollOffset) {
      setScrollOffset(rangeStart);
    } else if (rangeEnd >= scrollOffset + visibleHeight) {
      const rangeSize = rangeEnd - rangeStart + 1;
      if (rangeSize <= visibleHeight) {
        setScrollOffset(rangeEnd - visibleHeight + 1);
      } else {
        setScrollOffset(rangeStart);
      }
    }
  }, [selectedIndex, scrollOffset, visibleHeight, selectionMode, getVisibleRange]);

  // Call onFocus when selection changes
  useEffect(() => {
    if (selectionMode === 'item' && items.length > 0 && selectedIndex >= 0 && selectedIndex < items.length) {
      onFocus?.(items[selectedIndex], selectedIndex);
    }
  }, [selectedIndex, items, onFocus, selectionMode]);

  // Navigate to previous/next group (or item if no groupBy)
  const navigateToGroup = useCallback((direction: 'up' | 'down'): void => {
    if (items.length === 0) return;
    
    if (!groupBy) {
      // No grouping - standard single item navigation
      const newIndex = direction === 'up' 
        ? Math.max(0, selectedIndex - 1)
        : Math.min(items.length - 1, selectedIndex + 1);
      setSelectedIndex(newIndex);
      return;
    }
    
    const currentGroupId = items[selectedIndex] ? groupBy(items[selectedIndex]) : null;
    
    if (direction === 'up') {
      // Find first line of previous group
      for (let i = selectedIndex - 1; i >= 0; i--) {
        if (groupBy(items[i]) !== currentGroupId) {
          const prevGroupId = groupBy(items[i]);
          // Find first line of that group
          for (let j = i; j >= 0; j--) {
            if (groupBy(items[j]) !== prevGroupId) {
              setSelectedIndex(j + 1);
              return;
            }
          }
          setSelectedIndex(0);
          return;
        }
      }
      // Already at first group - stay at first line of current group
      for (let i = 0; i < items.length; i++) {
        if (groupBy(items[i]) === currentGroupId) {
          setSelectedIndex(i);
          return;
        }
      }
    } else {
      // Find first line of next group
      for (let i = selectedIndex + 1; i < items.length; i++) {
        if (groupBy(items[i]) !== currentGroupId) {
          setSelectedIndex(i);
          return;
        }
      }
      // Already at last group - stay at first line of current group
      for (let i = 0; i < items.length; i++) {
        if (groupBy(items[i]) === currentGroupId) {
          setSelectedIndex(i);
          return;
        }
      }
    }
  }, [items, selectedIndex, groupBy]);

  const navigateTo = useCallback((newIndex: number): void => {
    if (items.length === 0) return;
    let targetIndex = newIndex;
    if (enableWrapAround) {
      if (targetIndex < 0) targetIndex = items.length - 1;
      else if (targetIndex >= items.length) targetIndex = 0;
    } else {
      targetIndex = Math.max(0, Math.min(items.length - 1, targetIndex));
    }
    setSelectedIndex(targetIndex);
  }, [items.length, enableWrapAround]);

  // Mouse scroll handler with acceleration
  const handleScroll = useCallback((direction: 'up' | 'down'): void => {
    const now = Date.now();
    const timeDelta = now - lastScrollTime.current;
    if (timeDelta < 150) {
      scrollVelocity.current = Math.min(scrollVelocity.current + 1, 5);
    } else {
      scrollVelocity.current = 1;
    }
    lastScrollTime.current = now;
    const scrollAmount = scrollVelocity.current;
    const delta = direction === 'down' ? scrollAmount : -scrollAmount;

    if (selectionMode === 'scroll') {
      const newOffset = Math.max(0, Math.min(maxScrollOffset, scrollOffset + delta));
      setScrollOffset(newOffset);
      if (scrollToEnd) {
        const isAtBottom = newOffset >= maxScrollOffset - 1;
        if (direction === 'up' && !isAtBottom) setUserScrolledAway(true);
        else if (isAtBottom) setUserScrolledAway(false);
      }
    } else {
      if (groupBy) {
        navigateToGroup(direction);
      } else {
        navigateTo(selectedIndex + delta);
      }
    }
  }, [scrollOffset, selectedIndex, selectionMode, maxScrollOffset, navigateTo, navigateToGroup, scrollToEnd, groupBy]);

  // Mouse scroll input handler
  useInput((input, key) => {
    if (items.length === 0) return;
    if (input.startsWith('[M')) {
      const buttonByte = input.charCodeAt(2);
      if (buttonByte === 96) { handleScroll('up'); return; }
      if (buttonByte === 97) { handleScroll('down'); return; }
    }
    if (key.mouse) {
      if (key.mouse.button === 'wheelDown') { handleScroll('down'); return; }
      if (key.mouse.button === 'wheelUp') { handleScroll('up'); return; }
    }
  }, { isActive: isFocused });

  // Scroll mode navigation
  const handleScrollNavigation = (key: {
    upArrow?: boolean; downArrow?: boolean;
    pageUp?: boolean; pageDown?: boolean;
    home?: boolean; end?: boolean;
  }): void => {
    let newOffset = scrollOffset;
    let isScrollingUp = false;
    
    if (key.upArrow) { newOffset = Math.max(0, scrollOffset - 1); isScrollingUp = true; }
    else if (key.downArrow) { newOffset = Math.min(maxScrollOffset, scrollOffset + 1); }
    else if (key.pageUp) { newOffset = Math.max(0, scrollOffset - visibleHeight); isScrollingUp = true; }
    else if (key.pageDown) { newOffset = Math.min(maxScrollOffset, scrollOffset + visibleHeight); }
    else if (key.home) { newOffset = 0; isScrollingUp = true; }
    else if (key.end) { newOffset = maxScrollOffset; }
    
    setScrollOffset(newOffset);
    if (scrollToEnd) {
      const isAtBottom = newOffset >= maxScrollOffset - 1;
      if (isScrollingUp && !isAtBottom) setUserScrolledAway(true);
      else if (isAtBottom) setUserScrolledAway(false);
    }
  };

  // Item mode navigation
  const handleItemNavigation = (key: {
    upArrow?: boolean; downArrow?: boolean;
    pageUp?: boolean; pageDown?: boolean;
    home?: boolean; end?: boolean; return?: boolean;
  }): void => {
    if (key.upArrow) {
      if (groupBy) navigateToGroup('up');
      else navigateTo(selectedIndex - 1);
    } else if (key.downArrow) {
      if (groupBy) navigateToGroup('down');
      else navigateTo(selectedIndex + 1);
    } else if (key.pageUp) {
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

  // Keyboard navigation
  useInput((input, key) => {
    if (items.length === 0) return;
    if (input.startsWith('[M') || key.mouse) return;
    if (key.shift && (key.upArrow || key.downArrow)) return;

    if (selectionMode === 'scroll') {
      handleScrollNavigation(key);
    } else {
      handleItemNavigation(key);
    }
  }, { isActive: isFocused });

  // Check if item is selected (for group selection, all items in group are selected)
  const isItemSelected = useCallback((index: number): boolean => {
    if (selectionMode !== 'item') return false;
    if (!groupBy) return index === selectedIndex;
    if (!items[index] || !items[selectedIndex]) return false;
    return groupBy(items[index]) === groupBy(items[selectedIndex]);
  }, [selectionMode, groupBy, items, selectedIndex]);

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
          const isSelected = isItemSelected(actualIndex);
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
