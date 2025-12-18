{
  "matches": [
    {
      "type": "arrow_function",
      "line": 60,
      "column": 30,
      "text": "(terminalWidth: number): { baseWidth: number; remainder: number } => {\n  const borders = 2;\n  const separators = STATES.length - 1;\n  const availableWidth = terminalWidth - borders - separators;\n  const baseWidth = Math.floor(availableWidth / STATES.length);\n  const remainder = availableWidth % STATES.length;\n  return { baseWidth: Math.max(8, baseWidth), remainder: baseWidth >= 8 ? remainder : 0 };\n}"
    },
    {
      "type": "arrow_function",
      "line": 69,
      "column": 23,
      "text": "(columnIndex: number, baseWidth: number, remainder: number): number => {\n  return columnIndex < remainder ? baseWidth + 1 : baseWidth;\n}"
    },
    {
      "type": "arrow_function",
      "line": 74,
      "column": 23,
      "text": "(text: string): number => {\n  let width = 0;\n  for (const char of text) {\n    const code = char.codePointAt(0) || 0;\n    // Emoji ranges that render as 2 columns:\n    // U+2300-U+27BF (Miscellaneous Technical, Dingbats) - includes ⏩ (U+23E9)\n    // U+1F000+ (Emoticons, symbols, etc.)\n    // Arrows like ↓ (U+2193) render as 1 column\n    const isWide = (code >= 0x2300 && code <= 0x27BF) || code >= 0x1F000;\n    width += isWide ? 2 : 1;\n  }\n  return width;\n}"
    },
    {
      "type": "arrow_function",
      "line": 88,
      "column": 19,
      "text": "(text: string, width: number): string => {\n  const visualWidth = getVisualWidth(text);\n\n  if (visualWidth > width) {\n    // Truncate while being careful about emoji boundaries\n    let result = '';\n    let currentVisualWidth = 0;\n    for (const char of text) {\n      const code = char.codePointAt(0) || 0;\n      const isWide = (code >= 0x2300 && code <= 0x27BF) || code >= 0x1F000;\n      const charWidth = isWide ? 2 : 1;\n      if (currentVisualWidth + charWidth > width) break;\n      result += char;\n      currentVisualWidth += charWidth;\n    }\n    // Pad to exact width\n    return result + ' '.repeat(width - currentVisualWidth);\n  } else if (visualWidth < width) {\n    // Pad with spaces to reach visual width\n    return text + ' '.repeat(width - visualWidth);\n  }\n  return text;\n}"
    },
    {
      "type": "arrow_function",
      "line": 112,
      "column": 23,
      "text": "(\n  baseWidth: number,\n  remainder: number,\n  left: string,\n  mid: string,\n  right: string,\n  separatorType: 'plain' | 'top' | 'cross' | 'bottom' = 'cross'\n): string => {\n  const separatorChar = {\n    plain: '─',\n    top: '┬',\n    cross: '┼',\n    bottom: '┴',\n  }[separatorType];\n  return left + STATES.map((_, idx) => '─'.repeat(getColumnWidth(idx, baseWidth, remainder))).join(separatorChar) + right;\n}"
    },
    {
      "type": "arrow_function",
      "line": 126,
      "column": 27,
      "text": "(_, idx) => '─'.repeat(getColumnWidth(idx, baseWidth, remainder))"
    },
    {
      "type": "arrow_function",
      "line": 129,
      "column": 32,
      "text": "(terminalHeight: number): number => {\n  // Fixed rows breakdown:\n  // 1 (top border) + 4 (header) + 1 (header separator) +\n  // 5 (details) + 1 (details separator with ┬) + 1 (column headers) +\n  // 1 (column separator with ┼) + 1 (footer separator with ┴) +\n  // 1 (footer) + 1 (bottom border) + 1 (bottom padding) = 18\n  const fixedRows = 18;\n  return Math.max(5, terminalHeight - fixedRows);\n}"
    },
    {
      "type": "arrow_function",
      "line": 162,
      "column": 41,
      "text": "state => state.checkpointCounts"
    },
    {
      "type": "arrow_function",
      "line": 163,
      "column": 45,
      "text": "state => state.loadCheckpointCounts"
    },
    {
      "type": "arrow_function",
      "line": 166,
      "column": 12,
      "text": "() => {\n    void loadCheckpointCounts();\n  }"
    },
    {
      "type": "arrow_function",
      "line": 171,
      "column": 4,
      "text": "() => calculateColumnWidths(terminalWidth)"
    },
    {
      "type": "arrow_function",
      "line": 176,
      "column": 34,
      "text": "() => calculateViewportHeight(terminalHeight)"
    },
    {
      "type": "arrow_function",
      "line": 178,
      "column": 35,
      "text": "(sum, _, idx) => sum + getColumnWidth(idx, colWidth, colRemainder)"
    },
    {
      "type": "arrow_function",
      "line": 192,
      "column": 35,
      "text": "() => {\n    return STATES.map(status => {\n      const units = workUnits.filter(wu => wu.status === status);\n      const totalPoints = units.reduce((sum, wu) => sum + (wu.estimate || 0), 0);\n      return { status, units, count: units.length, totalPoints };\n    });\n  }"
    },
    {
      "type": "arrow_function",
      "line": 193,
      "column": 22,
      "text": "status => {\n      const units = workUnits.filter(wu => wu.status === status);\n      const totalPoints = units.reduce((sum, wu) => sum + (wu.estimate || 0), 0);\n      return { status, units, count: units.length, totalPoints };\n    }"
    },
    {
      "type": "arrow_function",
      "line": 194,
      "column": 37,
      "text": "wu => wu.status === status"
    },
    {
      "type": "arrow_function",
      "line": 195,
      "column": 39,
      "text": "(sum, wu) => sum + (wu.estimate || 0)"
    },
    {
      "type": "arrow_function",
      "line": 201,
      "column": 38,
      "text": "() => {\n    if (workUnits.length === 0) return null;\n\n    return workUnits.reduce((latest, current) => {\n      const latestStateTimestamp = latest.stateHistory && latest.stateHistory.length > 0\n        ? new Date(latest.stateHistory[latest.stateHistory.length - 1].timestamp).getTime()\n        : 0;\n      const currentStateTimestamp = current.stateHistory && current.stateHistory.length > 0\n        ? new Date(current.stateHistory[current.stateHistory.length - 1].timestamp).getTime()\n        : 0;\n\n      return currentStateTimestamp > latestStateTimestamp ? current : latest;\n    });\n  }"
    },
    {
      "type": "arrow_function",
      "line": 204,
      "column": 28,
      "text": "(latest, current) => {\n      const latestStateTimestamp = latest.stateHistory && latest.stateHistory.length > 0\n        ? new Date(latest.stateHistory[latest.stateHistory.length - 1].timestamp).getTime()\n        : 0;\n      const currentStateTimestamp = current.stateHistory && current.stateHistory.length > 0\n        ? new Date(current.stateHistory[current.stateHistory.length - 1].timestamp).getTime()\n        : 0;\n\n      return currentStateTimestamp > latestStateTimestamp ? current : latest;\n    }"
    },
    {
      "type": "arrow_function",
      "line": 217,
      "column": 12,
      "text": "() => {\n    const currentColumn = STATES[focusedColumnIndex];\n    const columnUnits = groupedWorkUnits[focusedColumnIndex].units;\n\n    if (columnUnits.length === 0) return;\n\n    setScrollOffsets(prev => {\n      const scrollOffset = prev[currentColumn];\n      const firstVisible = scrollOffset + (scrollOffset > 0 ? 1 : 0); // Account for up arrow\n      const lastVisible = scrollOffset + VIEWPORT_HEIGHT - 1 - (scrollOffset + VIEWPORT_HEIGHT < columnUnits.length ? 1 : 0); // Account for down arrow\n\n      // Check if selected item is visible\n      if (selectedWorkUnitIndex >= firstVisible && selectedWorkUnitIndex <= lastVisible) {\n        // Already visible, no scroll needed\n        return prev;\n      }\n\n      // Need to scroll - calculate new offset\n      let newOffset;\n\n      if (selectedWorkUnitIndex < firstVisible) {\n        // Scroll up: position selected item near top\n        newOffset = Math.max(0, selectedWorkUnitIndex - 1);\n        if (selectedWorkUnitIndex === 0) {\n          newOffset = 0;\n        }\n      } else {\n        // Scroll down: position selected item near bottom\n        const estimatedEffectiveHeight = VIEWPORT_HEIGHT - 2; // Assume both arrows\n        newOffset = selectedWorkUnitIndex - estimatedEffectiveHeight + 1;\n\n        const testUpArrow = newOffset > 0;\n        const testDownArrow = newOffset + VIEWPORT_HEIGHT < columnUnits.length;\n        const testArrows = (testUpArrow ? 1 : 0) + (testDownArrow ? 1 : 0);\n        const testEffectiveHeight = VIEWPORT_HEIGHT - testArrows;\n\n        newOffset = selectedWorkUnitIndex - testEffectiveHeight + (testUpArrow ? 0 : 1);\n      }\n\n      // Clamp to valid range\n      const maxOffset = Math.max(0, columnUnits.length - VIEWPORT_HEIGHT);\n      newOffset = Math.max(0, Math.min(newOffset, maxOffset));\n\n      return {\n        ...prev,\n        [currentColumn]: newOffset,\n      };\n    });\n  }"
    },
    {
      "type": "arrow_function",
      "line": 223,
      "column": 21,
      "text": "prev => {\n      const scrollOffset = prev[currentColumn];\n      const firstVisible = scrollOffset + (scrollOffset > 0 ? 1 : 0); // Account for up arrow\n      const lastVisible = scrollOffset + VIEWPORT_HEIGHT - 1 - (scrollOffset + VIEWPORT_HEIGHT < columnUnits.length ? 1 : 0); // Account for down arrow\n\n      // Check if selected item is visible\n      if (selectedWorkUnitIndex >= firstVisible && selectedWorkUnitIndex <= lastVisible) {\n        // Already visible, no scroll needed\n        return prev;\n      }\n\n      // Need to scroll - calculate new offset\n      let newOffset;\n\n      if (selectedWorkUnitIndex < firstVisible) {\n        // Scroll up: position selected item near top\n        newOffset = Math.max(0, selectedWorkUnitIndex - 1);\n        if (selectedWorkUnitIndex === 0) {\n          newOffset = 0;\n        }\n      } else {\n        // Scroll down: position selected item near bottom\n        const estimatedEffectiveHeight = VIEWPORT_HEIGHT - 2; // Assume both arrows\n        newOffset = selectedWorkUnitIndex - estimatedEffectiveHeight + 1;\n\n        const testUpArrow = newOffset > 0;\n        const testDownArrow = newOffset + VIEWPORT_HEIGHT < columnUnits.length;\n        const testArrows = (testUpArrow ? 1 : 0) + (testDownArrow ? 1 : 0);\n        const testEffectiveHeight = VIEWPORT_HEIGHT - testArrows;\n\n        newOffset = selectedWorkUnitIndex - testEffectiveHeight + (testUpArrow ? 0 : 1);\n      }\n\n      // Clamp to valid range\n      const maxOffset = Math.max(0, columnUnits.length - VIEWPORT_HEIGHT);\n      newOffset = Math.max(0, Math.min(newOffset, maxOffset));\n\n      return {\n        ...prev,\n        [currentColumn]: newOffset,\n      };\n    }"
    },
    {
      "type": "arrow_function",
      "line": 268,
      "column": 29,
      "text": "(direction: 'up' | 'down'): void => {\n    if (direction === 'down') {\n      // Scroll down: move selector down\n      onWorkUnitChange?.(1);\n    } else if (direction === 'up') {\n      // Scroll up: move selector up\n      onWorkUnitChange?.(-1);\n    }\n  }"
    },
    {
      "type": "arrow_function",
      "line": 282,
      "column": 12,
      "text": "() => {\n    if (!internal_eventEmitter || !onWorkUnitChange) return;\n\n    const handleRawInput = (data: string) => {\n      // Parse Home/End keys that useInput filters out\n      // Home: ESC[H, ESC[1~, ESC[7~, ESCOH\n      // End: ESC[F, ESC[4~, ESC[8~, ESCOF\n      // ESC is \\u001B (charCode 27)\n      const isHome = data === '\\u001B[H' || data === '\\u001B[1~' || data === '\\u001B[7~' || data === '\\u001BOH';\n      const isEnd = data === '\\u001B[F' || data === '\\u001B[4~' || data === '\\u001B[8~' || data === '\\u001BOF';\n\n      if (isHome) {\n        const columnUnits = groupedWorkUnits[focusedColumnIndex].units;\n        if (columnUnits.length > 0) {\n          onWorkUnitChange(-selectedWorkUnitIndex);\n        }\n      } else if (isEnd) {\n        const columnUnits = groupedWorkUnits[focusedColumnIndex].units;\n        if (columnUnits.length > 0) {\n          onWorkUnitChange(columnUnits.length - 1 - selectedWorkUnitIndex);\n        }\n      }\n    };\n\n    internal_eventEmitter.on('input', handleRawInput);\n    return () => {\n      internal_eventEmitter.removeListener('input', handleRawInput);\n    };\n  }"
    },
    {
      "type": "arrow_function",
      "line": 285,
      "column": 27,
      "text": "(data: string) => {\n      // Parse Home/End keys that useInput filters out\n      // Home: ESC[H, ESC[1~, ESC[7~, ESCOH\n      // End: ESC[F, ESC[4~, ESC[8~, ESCOF\n      // ESC is \\u001B (charCode 27)\n      const isHome = data === '\\u001B[H' || data === '\\u001B[1~' || data === '\\u001B[7~' || data === '\\u001BOH';\n      const isEnd = data === '\\u001B[F' || data === '\\u001B[4~' || data === '\\u001B[8~' || data === '\\u001BOF';\n\n      if (isHome) {\n        const columnUnits = groupedWorkUnits[focusedColumnIndex].units;\n        if (columnUnits.length > 0) {\n          onWorkUnitChange(-selectedWorkUnitIndex);\n        }\n      } else if (isEnd) {\n        const columnUnits = groupedWorkUnits[focusedColumnIndex].units;\n        if (columnUnits.length > 0) {\n          onWorkUnitChange(columnUnits.length - 1 - selectedWorkUnitIndex);\n        }\n      }\n    }"
    },
    {
      "type": "arrow_function",
      "line": 307,
      "column": 11,
      "text": "() => {\n      internal_eventEmitter.removeListener('input', handleRawInput);\n    }"
    },
    {
      "type": "arrow_function",
      "line": 313,
      "column": 11,
      "text": "(input, key) => {\n    // Mouse scroll handling (TUI-010)\n    // Parse raw escape sequences for terminals that don't parse mouse events\n    if (input && input.startsWith('[M')) {\n      const buttonByte = input.charCodeAt(2);\n      if (buttonByte === 96) {  // Scroll up (ASCII '`')\n        handleColumnScroll('up');\n        return;\n      } else if (buttonByte === 97) {  // Scroll down (ASCII 'a')\n        handleColumnScroll('down');\n        return;\n      }\n    }\n\n    // Handle Ink-parsed mouse events (primary method)\n    if (key.mouse) {\n      if (key.mouse.button === 'wheelDown') {\n        handleColumnScroll('down');\n        return;\n      } else if (key.mouse.button === 'wheelUp') {\n        handleColumnScroll('up');\n        return;\n      }\n    }\n\n    // Page Up/Down: Move selector by viewport height\n    if (key.pageDown) {\n      onWorkUnitChange?.(VIEWPORT_HEIGHT);\n      return;\n    }\n\n    if (key.pageUp) {\n      onWorkUnitChange?.(-VIEWPORT_HEIGHT);\n      return;\n    }\n\n    // Home/End are handled via raw stdin event emitter above\n    // because useInput filters them out before we can see them\n\n    // Arrow keys handled by parent\n    if (key.leftArrow || key.rightArrow) {\n      onColumnChange?.(key.rightArrow ? 1 : -1);\n    }\n    if (key.upArrow || key.downArrow) {\n      onWorkUnitChange?.(key.downArrow ? 1 : -1);\n    }\n    if (key.return) {\n      onEnter?.();\n    }\n\n    // BOARD-010: Bracket keys for priority reordering\n    if (input === '[') {\n      onMoveUp?.();\n    }\n    if (input === ']') {\n      onMoveDown?.();\n    }\n\n    // TUI-019: Open attachment dialog (handled at BoardView level)\n    // The 'a' key handler is now in BoardView to show AttachmentDialog\n  }"
    },
    {
      "type": "arrow_function",
      "line": 453,
      "column": 26,
      "text": "(state, idx) => {\n          const header = state.toUpperCase();\n          const currentColWidth = getColumnWidth(idx, colWidth, colRemainder);\n          const paddedHeader = fitToWidth(header, currentColWidth);\n          // Highlight focused column in cyan, others in gray\n          return idx === focusedColumnIndex ? chalk.cyan(paddedHeader) : chalk.gray(paddedHeader);\n        }"
    },
    {
      "type": "arrow_function",
      "line": 466,
      "column": 51,
      "text": "(_, rowIndex) => {\n        const cells = STATES.map((state, colIndex) => {\n          const column = groupedWorkUnits[colIndex];\n          const scrollOffset = scrollOffsets[state];\n          const itemIndex = scrollOffset + rowIndex;\n          const currentColWidth = getColumnWidth(colIndex, colWidth, colRemainder);\n\n          // Show scroll indicators ONLY if there are items to scroll to\n          if (rowIndex === 0 && scrollOffset > 0 && column.units.length > 0) {\n            return fitToWidth('↑', currentColWidth); // Up arrow at top when scrolled down\n          }\n          if (rowIndex === VIEWPORT_HEIGHT - 1 && scrollOffset + VIEWPORT_HEIGHT < column.units.length) {\n            return fitToWidth('↓', currentColWidth); // Down arrow at bottom when more items below\n          }\n\n          if (itemIndex >= column.units.length) {\n            return fitToWidth('', currentColWidth);\n          }\n\n          const wu = column.units[itemIndex];\n          const estimate = wu.estimate || 0;\n          const storyPointsText = estimate > 0 ? ` [${estimate}]` : '';\n\n          // Check if this work unit is the last changed (TUI-017)\n          const isLastChanged = lastChangedWorkUnit?.id === wu.id;\n          const text = isLastChanged\n            ? `⏩ ${wu.id}${storyPointsText} ⏩`\n            : `${wu.id}${storyPointsText}`;\n\n          const paddedText = fitToWidth(text, currentColWidth);\n\n          // Check if this work unit is selected\n          const isSelected = colIndex === focusedColumnIndex && itemIndex === selectedWorkUnitIndex;\n\n          // Apply color-coding without shimmer animation (TUI-017)\n          if (isSelected) {\n            return chalk.bgGreen.black(paddedText);\n          } else {\n            if (wu.type === 'bug') {\n              return chalk.red(paddedText);\n            } else if (wu.type === 'task') {\n              return chalk.blue(paddedText);\n            } else {\n              return chalk.white(paddedText);\n            }\n          }\n        });\n\n        return (\n          <Text key={rowIndex}>{'│' + cells.join('│') + '│'}</Text>\n        );\n      }"
    },
    {
      "type": "arrow_function",
      "line": 467,
      "column": 33,
      "text": "(state, colIndex) => {\n          const column = groupedWorkUnits[colIndex];\n          const scrollOffset = scrollOffsets[state];\n          const itemIndex = scrollOffset + rowIndex;\n          const currentColWidth = getColumnWidth(colIndex, colWidth, colRemainder);\n\n          // Show scroll indicators ONLY if there are items to scroll to\n          if (rowIndex === 0 && scrollOffset > 0 && column.units.length > 0) {\n            return fitToWidth('↑', currentColWidth); // Up arrow at top when scrolled down\n          }\n          if (rowIndex === VIEWPORT_HEIGHT - 1 && scrollOffset + VIEWPORT_HEIGHT < column.units.length) {\n            return fitToWidth('↓', currentColWidth); // Down arrow at bottom when more items below\n          }\n\n          if (itemIndex >= column.units.length) {\n            return fitToWidth('', currentColWidth);\n          }\n\n          const wu = column.units[itemIndex];\n          const estimate = wu.estimate || 0;\n          const storyPointsText = estimate > 0 ? ` [${estimate}]` : '';\n\n          // Check if this work unit is the last changed (TUI-017)\n          const isLastChanged = lastChangedWorkUnit?.id === wu.id;\n          const text = isLastChanged\n            ? `⏩ ${wu.id}${storyPointsText} ⏩`\n            : `${wu.id}${storyPointsText}`;\n\n          const paddedText = fitToWidth(text, currentColWidth);\n\n          // Check if this work unit is selected\n          const isSelected = colIndex === focusedColumnIndex && itemIndex === selectedWorkUnitIndex;\n\n          // Apply color-coding without shimmer animation (TUI-017)\n          if (isSelected) {\n            return chalk.bgGreen.black(paddedText);\n          } else {\n            if (wu.type === 'bug') {\n              return chalk.red(paddedText);\n            } else if (wu.type === 'task') {\n              return chalk.blue(paddedText);\n            } else {\n              return chalk.white(paddedText);\n            }\n          }\n        }"
    }
  ]
}
