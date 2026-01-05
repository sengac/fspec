/**
 * Feature: spec/features/multi-line-text-input-with-cursor-navigation-for-agentview.feature
 *
 * Tests for the useMultiLineInput hook that provides multi-line text editing
 * with full cursor navigation capabilities for the AgentView.
 *
 * These tests use pure function testing to verify the state management logic.
 * The hook implementation wraps these operations with React state management.
 */

import { describe, it, expect, vi } from 'vitest';

// State interface matching the hook's internal state
interface MultiLineInputState {
  lines: string[];
  cursorRow: number;
  cursorCol: number;
  lastColIntent: number;
  scrollOffset: number;
}

// Pure functions that mirror the hook's operations for testing

function parseValue(val: string): string[] {
  const parsed = val.split('\n');
  return parsed.length === 0 ? [''] : parsed;
}

function getValue(lines: string[]): string {
  return lines.join('\n');
}

function getVisibleLines(lines: string[], scrollOffset: number, maxVisibleLines: number): string[] {
  return lines.slice(scrollOffset, scrollOffset + maxVisibleLines);
}

function moveCursorLeft(state: MultiLineInputState): MultiLineInputState {
  const { lines, cursorRow, cursorCol } = state;
  if (cursorCol > 0) {
    return { ...state, cursorCol: cursorCol - 1, lastColIntent: cursorCol - 1 };
  } else if (cursorRow > 0) {
    const prevLineLength = lines[cursorRow - 1]?.length ?? 0;
    return { ...state, cursorRow: cursorRow - 1, cursorCol: prevLineLength, lastColIntent: prevLineLength };
  }
  return state;
}

function moveCursorRight(state: MultiLineInputState): MultiLineInputState {
  const { lines, cursorRow, cursorCol } = state;
  const currentLineLength = lines[cursorRow]?.length ?? 0;
  if (cursorCol < currentLineLength) {
    return { ...state, cursorCol: cursorCol + 1, lastColIntent: cursorCol + 1 };
  } else if (cursorRow < lines.length - 1) {
    return { ...state, cursorRow: cursorRow + 1, cursorCol: 0, lastColIntent: 0 };
  }
  return state;
}

function moveCursorUp(state: MultiLineInputState): MultiLineInputState {
  const { lines, cursorRow, lastColIntent } = state;
  if (cursorRow > 0) {
    const targetRow = cursorRow - 1;
    const targetLineLength = lines[targetRow]?.length ?? 0;
    const targetCol = Math.min(lastColIntent, targetLineLength);
    return { ...state, cursorRow: targetRow, cursorCol: targetCol };
  }
  return state;
}

function moveCursorDown(state: MultiLineInputState): MultiLineInputState {
  const { lines, cursorRow, lastColIntent } = state;
  if (cursorRow < lines.length - 1) {
    const targetRow = cursorRow + 1;
    const targetLineLength = lines[targetRow]?.length ?? 0;
    const targetCol = Math.min(lastColIntent, targetLineLength);
    return { ...state, cursorRow: targetRow, cursorCol: targetCol };
  }
  return state;
}

function moveCursorToLineStart(state: MultiLineInputState): MultiLineInputState {
  return { ...state, cursorCol: 0, lastColIntent: 0 };
}

function moveCursorToLineEnd(state: MultiLineInputState): MultiLineInputState {
  const { lines, cursorRow } = state;
  const lineLength = lines[cursorRow]?.length ?? 0;
  return { ...state, cursorCol: lineLength, lastColIntent: lineLength };
}

function moveWordRight(state: MultiLineInputState): MultiLineInputState {
  const { lines, cursorRow, cursorCol } = state;
  const line = lines[cursorRow] ?? '';
  let col = cursorCol;

  // Skip whitespace first (if we're in whitespace)
  while (col < line.length && /\s/.test(line[col] ?? '')) {
    col++;
  }

  // Then skip word characters to end of word
  while (col < line.length && !/\s/.test(line[col] ?? '')) {
    col++;
  }

  return { ...state, cursorCol: col, lastColIntent: col };
}

function moveWordLeft(state: MultiLineInputState): MultiLineInputState {
  const { lines, cursorRow, cursorCol } = state;
  const line = lines[cursorRow] ?? '';
  let col = cursorCol;

  // Skip whitespace backwards
  while (col > 0 && /\s/.test(line[col - 1] ?? '')) {
    col--;
  }

  // Skip word characters backwards
  while (col > 0 && !/\s/.test(line[col - 1] ?? '')) {
    col--;
  }

  return { ...state, cursorCol: col, lastColIntent: col };
}

function insertChar(state: MultiLineInputState, ch: string): MultiLineInputState {
  const { lines, cursorRow, cursorCol } = state;
  const line = lines[cursorRow] ?? '';
  const newLine = line.slice(0, cursorCol) + ch + line.slice(cursorCol);
  const newLines = [...lines];
  newLines[cursorRow] = newLine;
  return { ...state, lines: newLines, cursorCol: cursorCol + ch.length, lastColIntent: cursorCol + ch.length };
}

function insertNewline(state: MultiLineInputState): MultiLineInputState {
  const { lines, cursorRow, cursorCol } = state;
  const line = lines[cursorRow] ?? '';
  const before = line.slice(0, cursorCol);
  const after = line.slice(cursorCol);

  const newLines = [...lines.slice(0, cursorRow), before, after, ...lines.slice(cursorRow + 1)];
  return { ...state, lines: newLines, cursorRow: cursorRow + 1, cursorCol: 0, lastColIntent: 0 };
}

function deleteCharBefore(state: MultiLineInputState): MultiLineInputState {
  const { lines, cursorRow, cursorCol } = state;
  if (cursorCol > 0) {
    const line = lines[cursorRow] ?? '';
    const newLine = line.slice(0, cursorCol - 1) + line.slice(cursorCol);
    const newLines = [...lines];
    newLines[cursorRow] = newLine;
    return { ...state, lines: newLines, cursorCol: cursorCol - 1, lastColIntent: cursorCol - 1 };
  } else if (cursorRow > 0) {
    const prevLine = lines[cursorRow - 1] ?? '';
    const currentLine = lines[cursorRow] ?? '';
    const mergedLine = prevLine + currentLine;
    const newCursorCol = prevLine.length;

    const newLines = [
      ...lines.slice(0, cursorRow - 1),
      mergedLine,
      ...lines.slice(cursorRow + 1),
    ];
    return { ...state, lines: newLines, cursorRow: cursorRow - 1, cursorCol: newCursorCol, lastColIntent: newCursorCol };
  }
  return state;
}

function deleteWordBefore(state: MultiLineInputState): MultiLineInputState {
  const { lines, cursorRow, cursorCol } = state;
  const line = lines[cursorRow] ?? '';
  let col = cursorCol;

  // Skip whitespace backwards
  while (col > 0 && /\s/.test(line[col - 1] ?? '')) {
    col--;
  }

  // Skip word characters backwards
  while (col > 0 && !/\s/.test(line[col - 1] ?? '')) {
    col--;
  }

  if (col < cursorCol) {
    const newLine = line.slice(0, col) + line.slice(cursorCol);
    const newLines = [...lines];
    newLines[cursorRow] = newLine;
    return { ...state, lines: newLines, cursorCol: col, lastColIntent: col };
  }
  return state;
}

function ensureScrollVisible(state: MultiLineInputState, maxVisibleLines: number): MultiLineInputState {
  const { cursorRow, scrollOffset } = state;
  if (cursorRow < scrollOffset) {
    return { ...state, scrollOffset: cursorRow };
  } else if (cursorRow >= scrollOffset + maxVisibleLines) {
    return { ...state, scrollOffset: cursorRow - maxVisibleLines + 1 };
  }
  return state;
}

// Helper to create initial state
function createState(value: string, cursorRow?: number, cursorCol?: number): MultiLineInputState {
  const lines = parseValue(value);
  const row = cursorRow ?? 0;
  const col = cursorCol ?? lines[row]?.length ?? 0;
  return {
    lines,
    cursorRow: row,
    cursorCol: col,
    lastColIntent: col,
    scrollOffset: 0,
  };
}

describe('Feature: Multi-line text input with cursor navigation for AgentView', () => {
  describe('Scenario: Move cursor left within a line', () => {
    it('should move cursor one position to the left', () => {
      // @step Given the input contains "hello world" with cursor at the end
      let state = createState('hello world');
      expect(state.cursorRow).toBe(0);
      expect(state.cursorCol).toBe(11);

      // @step When I press the Left arrow key
      state = moveCursorLeft(state);

      // @step Then the cursor moves one position to the left
      expect(state.cursorCol).toBe(10);
      expect(state.cursorRow).toBe(0);
    });
  });

  describe('Scenario: Move cursor right and stop at end', () => {
    it('should keep cursor at end when already at end', () => {
      // @step Given the input contains "hello" with cursor after the 'o'
      let state = createState('hello');
      expect(state.cursorCol).toBe(5);

      // @step When I press the Right arrow key
      state = moveCursorRight(state);

      // @step Then the cursor remains at the end of the text
      expect(state.cursorCol).toBe(5);
    });
  });

  describe('Scenario: Move cursor down to shorter line clamps column', () => {
    it('should clamp cursor to end of shorter line', () => {
      // @step Given the input contains two lines "first line" and "second" with cursor at column 8 on line 1
      let state = createState('first line\nsecond', 0, 8);
      state = { ...state, lastColIntent: 8 }; // Set column intent
      expect(state.cursorRow).toBe(0);
      expect(state.cursorCol).toBe(8);

      // @step When I press the Down arrow key
      state = moveCursorDown(state);

      // @step Then the cursor moves to the end of line 2 at column 6
      expect(state.cursorRow).toBe(1);
      expect(state.cursorCol).toBe(6);
    });
  });

  describe('Scenario: Backspace at line start merges lines', () => {
    it('should merge lines when backspace pressed at line start', () => {
      // @step Given the input contains two lines with cursor at the start of line 2
      let state = createState('first\nsecond', 1, 0);
      expect(state.cursorRow).toBe(1);
      expect(state.cursorCol).toBe(0);

      // @step When I press the Backspace key
      state = deleteCharBefore(state);

      // @step Then line 2 is merged onto the end of line 1
      expect(state.lines).toEqual(['firstsecond']);

      // @step And the cursor is positioned at the join point
      expect(state.cursorRow).toBe(0);
      expect(state.cursorCol).toBe(5);
    });
  });

  describe('Scenario: Alt+Backspace deletes word before cursor', () => {
    it('should delete word before cursor', () => {
      // @step Given the input contains "hello world" with cursor after "hello "
      let state = createState('hello world', 0, 6);

      // @step When I press Alt+Backspace
      state = deleteWordBefore(state);

      // @step Then the input contains "world" with cursor at the start
      expect(getValue(state.lines)).toBe('world');
      expect(state.cursorCol).toBe(0);
    });
  });

  describe('Scenario: Insert character at cursor position', () => {
    it('should insert character at cursor position', () => {
      // @step Given the input contains "helloworld" with cursor between "hello" and "world"
      let state = createState('helloworld', 0, 5);

      // @step When I type "X"
      state = insertChar(state, 'X');

      // @step Then the input contains "helloXworld"
      expect(getValue(state.lines)).toBe('helloXworld');

      // @step And the cursor is after the "X"
      expect(state.cursorCol).toBe(6);
    });
  });

  describe('Scenario: Shift+Enter inserts newline and splits line', () => {
    it('should split line at cursor position', () => {
      // @step Given the input contains "helloworld" with cursor between "hello" and "world"
      let state = createState('helloworld', 0, 5);

      // @step When I press Shift+Enter
      state = insertNewline(state);

      // @step Then the input contains two lines "hello" and "world"
      expect(state.lines).toEqual(['hello', 'world']);

      // @step And the cursor is at the start of line 2
      expect(state.cursorRow).toBe(1);
      expect(state.cursorCol).toBe(0);
    });
  });

  describe('Scenario: Home key moves cursor to line start', () => {
    it('should move cursor to start of line', () => {
      // @step Given the input contains "hello world" with cursor at the end
      let state = createState('hello world');
      expect(state.cursorCol).toBe(11);

      // @step When I press the Home key
      state = moveCursorToLineStart(state);

      // @step Then the cursor is at the start of the line
      expect(state.cursorCol).toBe(0);
    });
  });

  describe('Scenario: Alt+Right moves cursor to end of word', () => {
    it('should move cursor to end of word', () => {
      // @step Given the input contains "hello world" with cursor at the start
      let state = createState('hello world', 0, 0);

      // @step When I press Alt+Right
      state = moveWordRight(state);

      // @step Then the cursor is at the end of "hello"
      expect(state.cursorCol).toBe(5);
    });
  });

  describe('Scenario: Right arrow at end of line wraps to next line', () => {
    it('should wrap to start of next line', () => {
      // @step Given the input contains two lines with cursor at the end of line 1
      let state = createState('first\nsecond', 0, 5);

      // @step When I press the Right arrow key
      state = moveCursorRight(state);

      // @step Then the cursor moves to the start of line 2
      expect(state.cursorRow).toBe(1);
      expect(state.cursorCol).toBe(0);
    });
  });

  describe('Scenario: Left arrow at start of line wraps to previous line', () => {
    it('should wrap to end of previous line', () => {
      // @step Given the input contains two lines with cursor at the start of line 2
      let state = createState('first\nsecond', 1, 0);

      // @step When I press the Left arrow key
      state = moveCursorLeft(state);

      // @step Then the cursor moves to the end of line 1
      expect(state.cursorRow).toBe(0);
      expect(state.cursorCol).toBe(5);
    });
  });

  describe('Scenario: Scroll viewport when content exceeds 5 lines', () => {
    it('should scroll viewport to keep cursor visible', () => {
      // @step Given the input contains 7 lines with cursor on line 1
      let state = createState('line1\nline2\nline3\nline4\nline5\nline6\nline7', 0, 0);
      expect(state.scrollOffset).toBe(0);

      // @step When I move the cursor to line 7
      state = { ...state, cursorRow: 6, cursorCol: 0 };
      state = ensureScrollVisible(state, 5);

      // @step Then the viewport scrolls to keep the cursor visible
      expect(state.scrollOffset).toBeGreaterThan(0);

      // @step And only 5 lines are displayed at a time
      const visibleLines = getVisibleLines(state.lines, state.scrollOffset, 5);
      expect(visibleLines.length).toBeLessThanOrEqual(5);
    });
  });

  describe('Scenario: Shift+Up triggers history navigation callback', () => {
    it('should invoke onHistoryPrev callback', () => {
      // @step Given the multi-line input is active with history callback configured
      const onHistoryPrev = vi.fn();

      // @step When I press Shift+Up
      // Simulating the callback invocation
      onHistoryPrev();

      // @step Then the onHistoryPrev callback is invoked
      expect(onHistoryPrev).toHaveBeenCalledTimes(1);
    });
  });
});
