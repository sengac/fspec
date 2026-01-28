import React, {
  useState,
  useEffect,
  useMemo,
  useRef,
  useLayoutEffect,
  useCallback,
  memo,
  useId,
} from 'react';
import { Box, Text, measureElement, type DOMElement } from 'ink';
import { useTerminalSize } from '../hooks/useTerminalSize';
import { useInputCompat, InputPriority } from '../input/index';

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
  // Standard mode: provide all items upfront
  items: T[];
  
  // PERF-004: Lazy mode - provide item count and accessor function
  // When provided, getItems is used instead of items.slice() for viewport access.
  // This enables viewport-aware lazy computation for very large lists.
  itemCount?: number;
  getItems?: (startIndex: number, endIndex: number) => T[];
  
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
  
  // Height adjustment for bordered containers. When VirtualList is inside
  // a pane with borders (e.g., CheckpointViewer), Yoga's measured height
  // may be slightly larger than the actual available space. Use -1 or -2
  // to compensate. Default 0 for unbounded containers (e.g., AgentView).
  heightAdjustment?: number;
  
  // TUI-042/043/044: Group-based selection (for turn selection)
  // When provided, items are grouped by the returned identifier.
  // Navigation moves between groups, selection highlights entire group,
  // and selection is preserved by group ID when content changes.
  groupBy?: (item: T) => string | number;
  
  // PERF-004: Lazy mode alternative - provide groupBy that works with index
  // instead of item. Use when items aren't readily available (lazy loading).
  groupByIndex?: (index: number) => string | number;
  
  // Extra lines to include before the group when scrolling (for separator bars)
  groupPaddingBefore?: number;
  
  // Ref to expose selected index to parent (for /expand command)
  selectionRef?: React.MutableRefObject<{ selectedIndex: number }>;
}

export function VirtualList<T>({
  items,
  itemCount: itemCountProp,
  getItems,
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
  heightAdjustment = 0,
  groupBy,
  groupByIndex,
  groupPaddingBefore = 0,
  selectionRef,
}: VirtualListProps<T>): React.ReactElement {
  // PERF-004: Determine total item count
  // Use itemCount prop for lazy mode, otherwise use items.length
  const totalItemCount = itemCountProp ?? items.length;
  
  // PERF-004: Determine if lazy mode is active
  const isLazyMode = getItems !== undefined && itemCountProp !== undefined;
  
  // PERF-004: Unified group function that works with either groupBy or groupByIndex
  // This allows lazy mode to use groupByIndex without needing to fetch items
  const getGroupId = useCallback((index: number): string | number | null => {
    if (groupByIndex) {
      return groupByIndex(index);
    }
    if (groupBy && items[index]) {
      return groupBy(items[index]);
    }
    return null;
  }, [groupBy, groupByIndex, items]);
  
  // Generate unique ID for this component instance
  const instanceId = useId();

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
    // PERF-004: Use unified getGroupId for group tracking
    const groupId = getGroupId(selectedIndex);
    if (groupId !== null) {
      selectedGroupIdRef.current = groupId;
    }
  }, [selectedIndex, selectionRef, getGroupId]);

  // Preserve selection by group ID when items change
  useEffect(() => {
    // PERF-004: Skip group preservation in lazy mode (handled by parent)
    if (isLazyMode) return;
    if (!groupBy || selectedGroupIdRef.current === null || totalItemCount === 0) return;
    
    const targetGroupId = selectedGroupIdRef.current;
    // Find first item with same group ID
    for (let i = 0; i < totalItemCount; i++) {
      if (getGroupId(i) === targetGroupId) {
        if (i !== selectedIndex) {
          setSelectedIndex(i);
        }
        return;
      }
    }
  }, [items, groupBy, selectedIndex, totalItemCount, getGroupId, isLazyMode]);

  // Measure container height
  const containerRef = useRef<DOMElement>(null);
  const [measuredHeight, setMeasuredHeight] = useState<number | null>(null);
  const lastScrollTime = useRef<number>(0);
  const scrollVelocity = useRef<number>(1);
  const [userScrolledAway, setUserScrolledAway] = useState(false);
  const measurementScheduled = useRef(false);

  // Re-measure container height on every render (no dependency array).
  // This ensures VirtualList adapts when sibling components change size
  // (e.g., multi-line input area grows/shrinks).
  // The setTimeout(..., 0) ensures Yoga layout is complete before measuring.
  // The measurementScheduled guard prevents multiple simultaneous measurements.
  // The setMeasuredHeight comparison prevents unnecessary state updates.
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
          const newHeight = Math.max(1, Math.floor(dimensions.height) + heightAdjustment);
          setMeasuredHeight(prev => (prev !== newHeight ? newHeight : prev));
        }
      }
      measurementScheduled.current = false;
    }, 0);
    return () => {
      clearTimeout(timeoutId);
      measurementScheduled.current = false;
    };
  });

  const visibleHeight = useMemo(() => {
    if (fixedHeight !== undefined) return fixedHeight;
    return measuredHeight !== null
      ? Math.max(1, measuredHeight)
      : Math.max(1, terminalHeight - reservedLines);
  }, [measuredHeight, terminalHeight, reservedLines, fixedHeight]);

  // PERF-004: Compute visible items using lazy accessor when available
  const visibleItems = useMemo(() => {
    const start = scrollOffset;
    const end = Math.min(totalItemCount, scrollOffset + visibleHeight);
    
    // Use lazy accessor if available, otherwise slice the items array
    if (isLazyMode && getItems) {
      return getItems(start, end);
    }
    return items.slice(start, end);
  }, [items, scrollOffset, visibleHeight, totalItemCount, isLazyMode, getItems]);

  // Clamp selection if items shrink
  useEffect(() => {
    if (totalItemCount > 0 && selectedIndex >= totalItemCount) {
      setSelectedIndex(Math.max(0, totalItemCount - 1));
    }
  }, [totalItemCount, selectedIndex]);

  // Select last item when transitioning from scroll mode to item mode (with scrollToEnd)
  // This ensures the last turn is selected when entering turn selection mode
  // Note: Don't set scrollOffset here - the scroll adjustment effect (below) will
  // properly position the viewport using getVisibleRange which accounts for groupPaddingBefore
  useEffect(() => {
    const wasScrollMode = prevSelectionModeRef.current === 'scroll';
    prevSelectionModeRef.current = selectionMode;
    
    if (wasScrollMode && selectionMode === 'item' && scrollToEnd && totalItemCount > 0) {
      setSelectedIndex(totalItemCount - 1);
    }
  }, [selectionMode, scrollToEnd, totalItemCount]);

  const maxScrollOffset = useMemo(
    () => Math.max(0, totalItemCount - visibleHeight),
    [totalItemCount, visibleHeight]
  );

  // Auto-scroll to end (only in scroll mode to preserve selection in item mode)
  useEffect(() => {
    if (scrollToEnd && totalItemCount > 0 && !userScrolledAway && selectionMode !== 'item') {
      const lastIndex = totalItemCount - 1;
      const newOffset = Math.max(0, lastIndex - visibleHeight + 1);
      setScrollOffset(newOffset);
    }
  }, [scrollToEnd, totalItemCount, visibleHeight, selectionMode, userScrolledAway]);

  // Get visible range for current selection (including group padding)
  // PERF-004: Updated to use getGroupId for lazy mode compatibility
  const getVisibleRange = useCallback((index: number): [number, number] => {
    const hasGrouping = groupBy || groupByIndex;
    if (!hasGrouping || totalItemCount === 0) {
      return [index, index];
    }
    
    const groupId = getGroupId(index);
    if (groupId === null) return [index, index];
    
    // Find first and last item in group
    // PERF-004: Use getGroupId for lazy mode compatibility
    let rangeStart = index;
    let rangeEnd = index;
    
    for (let i = index - 1; i >= 0; i--) {
      if (getGroupId(i) === groupId) {
        rangeStart = i;
      } else {
        break;
      }
    }
    
    for (let i = index + 1; i < totalItemCount; i++) {
      if (getGroupId(i) === groupId) {
        rangeEnd = i;
      } else {
        break;
      }
    }
    
    // Apply padding before (for separator lines)
    rangeStart = Math.max(0, rangeStart - groupPaddingBefore);
    
    return [rangeStart, rangeEnd];
  }, [getGroupId, totalItemCount, groupPaddingBefore, groupBy, groupByIndex]);

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
  // PERF-004: Only call onFocus in non-lazy mode (lazy mode has different data access)
  useEffect(() => {
    if (isLazyMode) return;  // Skip in lazy mode - parent handles this
    if (selectionMode === 'item' && totalItemCount > 0 && selectedIndex >= 0 && selectedIndex < totalItemCount) {
      const item = items[selectedIndex];
      if (item !== undefined) {
        onFocus?.(item, selectedIndex);
      }
    }
  }, [selectedIndex, items, onFocus, selectionMode, totalItemCount, isLazyMode]);

  // Navigate to previous/next group (or item if no groupBy)
  // PERF-004: Updated to use getGroupId for lazy mode compatibility
  const navigateToGroup = useCallback((direction: 'up' | 'down'): void => {
    if (totalItemCount === 0) return;
    
    const hasGrouping = groupBy || groupByIndex;
    if (!hasGrouping) {
      // No grouping - standard single item navigation
      const newIndex = direction === 'up' 
        ? Math.max(0, selectedIndex - 1)
        : Math.min(totalItemCount - 1, selectedIndex + 1);
      setSelectedIndex(newIndex);
      return;
    }
    
    const currentGroupId = getGroupId(selectedIndex);
    
    if (direction === 'up') {
      // Find first line of previous group
      for (let i = selectedIndex - 1; i >= 0; i--) {
        if (getGroupId(i) !== currentGroupId) {
          const prevGroupId = getGroupId(i);
          // Find first line of that group
          for (let j = i; j >= 0; j--) {
            if (getGroupId(j) !== prevGroupId) {
              setSelectedIndex(j + 1);
              return;
            }
          }
          setSelectedIndex(0);
          return;
        }
      }
      // Already at first group - stay at first line of current group
      for (let i = 0; i < totalItemCount; i++) {
        if (getGroupId(i) === currentGroupId) {
          setSelectedIndex(i);
          return;
        }
      }
    } else {
      // Find first line of next group
      for (let i = selectedIndex + 1; i < totalItemCount; i++) {
        if (getGroupId(i) !== currentGroupId) {
          setSelectedIndex(i);
          return;
        }
      }
      // Already at last group - stay at first line of current group
      for (let i = 0; i < totalItemCount; i++) {
        if (getGroupId(i) === currentGroupId) {
          setSelectedIndex(i);
          return;
        }
      }
    }
  }, [totalItemCount, selectedIndex, getGroupId, groupBy, groupByIndex]);

  const navigateTo = useCallback((newIndex: number): void => {
    if (totalItemCount === 0) return;
    let targetIndex = newIndex;
    if (enableWrapAround) {
      if (targetIndex < 0) targetIndex = totalItemCount - 1;
      else if (targetIndex >= totalItemCount) targetIndex = 0;
    } else {
      targetIndex = Math.max(0, Math.min(totalItemCount - 1, targetIndex));
    }
    setSelectedIndex(targetIndex);
  }, [totalItemCount, enableWrapAround]);

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

  // Mouse scroll input handler with BACKGROUND priority
  useInputCompat({
    id: `virtual-list-scroll-${instanceId}`,
    priority: InputPriority.BACKGROUND,
    description: 'Virtual list mouse scroll',
    isActive: isFocused,
    handler: (input, key) => {
      if (totalItemCount === 0) return false;
      if (input.startsWith('[M')) {
        const buttonByte = input.charCodeAt(2);
        if (buttonByte === 96) { handleScroll('up'); return true; }
        if (buttonByte === 97) { handleScroll('down'); return true; }
      }
      if (key.mouse) {
        if (key.mouse.button === 'wheelDown') { handleScroll('down'); return true; }
        if (key.mouse.button === 'wheelUp') { handleScroll('up'); return true; }
      }
      return false;
    },
  });

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
    const hasGrouping = groupBy || groupByIndex;
    if (key.upArrow) {
      if (hasGrouping) navigateToGroup('up');
      else navigateTo(selectedIndex - 1);
    } else if (key.downArrow) {
      if (hasGrouping) navigateToGroup('down');
      else navigateTo(selectedIndex + 1);
    } else if (key.pageUp) {
      navigateTo(Math.max(0, selectedIndex - visibleHeight));
    } else if (key.pageDown) {
      navigateTo(Math.min(totalItemCount - 1, selectedIndex + visibleHeight));
    } else if (key.home) {
      navigateTo(0);
    } else if (key.end) {
      navigateTo(totalItemCount - 1);
    } else if (key.return && onSelect && !isLazyMode) {
      // PERF-004: In lazy mode, parent must handle selection via selectionRef
      const item = items[selectedIndex];
      if (item !== undefined) {
        onSelect(item, selectedIndex);
      }
    }
  };

  // Keyboard navigation with BACKGROUND priority
  useInputCompat({
    id: `virtual-list-nav-${instanceId}`,
    priority: InputPriority.BACKGROUND,
    description: 'Virtual list keyboard navigation',
    isActive: isFocused,
    handler: (input, key) => {
      if (totalItemCount === 0) return false;
      if (input.startsWith('[M') || key.mouse) return false;
      if (key.shift && (key.upArrow || key.downArrow)) return false;

      if (selectionMode === 'scroll') {
        handleScrollNavigation(key);
        return true;
      } else {
        handleItemNavigation(key);
        return true;
      }
    },
  });

  // Check if item is selected (for group selection, all items in group are selected)
  // PERF-004: Updated to use getGroupId for lazy mode compatibility
  const isItemSelected = useCallback((index: number): boolean => {
    if (selectionMode !== 'item') return false;
    const hasGrouping = groupBy || groupByIndex;
    if (!hasGrouping) return index === selectedIndex;
    return getGroupId(index) === getGroupId(selectedIndex);
  }, [selectionMode, groupBy, groupByIndex, getGroupId, selectedIndex]);

  if (totalItemCount === 0) {
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
          itemCount={totalItemCount}
          visibleHeight={visibleHeight}
          scrollOffset={scrollOffset}
        />
      )}
    </Box>
  );
}
