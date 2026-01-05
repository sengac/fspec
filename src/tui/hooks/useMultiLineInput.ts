/**
 * useMultiLineInput - Hook for managing multi-line text input state with cursor navigation
 *
 * Provides cursor movement, text editing, line management, and viewport scrolling
 * for the MultiLineInput component.
 */

import { useState, useCallback, useMemo, useRef } from 'react';

export interface UseMultiLineInputOptions {
  initialValue?: string;
  maxVisibleLines?: number;
  onHistoryPrev?: () => void;
  onHistoryNext?: () => void;
}

export interface UseMultiLineInputResult {
  // State
  lines: string[];
  cursorRow: number;
  cursorCol: number;
  scrollOffset: number;
  value: string;
  visibleLines: string[];

  // Cursor Movement
  moveCursorLeft: () => void;
  moveCursorRight: () => void;
  moveCursorUp: () => void;
  moveCursorDown: () => void;
  moveCursorToLineStart: () => void;
  moveCursorToLineEnd: () => void;
  moveWordLeft: () => void;
  moveWordRight: () => void;

  // Text Editing
  insertChar: (ch: string) => void;
  insertString: (str: string) => void;
  insertNewline: () => void;
  deleteCharBefore: () => void;
  deleteCharAt: () => void;
  deleteWordBefore: () => void;

  // State Setters
  setCursor: (row: number, col: number) => void;
  setValue: (value: string) => void;

  // History Callbacks
  handleHistoryPrev: () => void;
  handleHistoryNext: () => void;
}

export function useMultiLineInput(
  options: UseMultiLineInputOptions = {}
): UseMultiLineInputResult {
  const {
    initialValue = '',
    maxVisibleLines = 5,
    onHistoryPrev,
    onHistoryNext,
  } = options;

  // Parse initial value into lines
  const parseValue = useCallback((val: string): string[] => {
    const parsed = val.split('\n');
    return parsed.length === 0 ? [''] : parsed;
  }, []);

  const [lines, setLines] = useState<string[]>(() => parseValue(initialValue));
  const [cursorRow, setCursorRow] = useState(0);
  const [cursorCol, setCursorCol] = useState(() => {
    const initialLines = parseValue(initialValue);
    return initialLines[0]?.length ?? 0;
  });
  const [lastColIntent, setLastColIntent] = useState(0);
  const [scrollOffset, setScrollOffset] = useState(0);

  // Computed value
  const value = useMemo(() => lines.join('\n'), [lines]);

  // Visible lines based on scroll offset
  const visibleLines = useMemo(() => {
    return lines.slice(scrollOffset, scrollOffset + maxVisibleLines);
  }, [lines, scrollOffset, maxVisibleLines]);

  // Ensure cursor is visible and adjust scroll
  const ensureCursorVisible = useCallback(
    (row: number) => {
      if (row < scrollOffset) {
        setScrollOffset(row);
      } else if (row >= scrollOffset + maxVisibleLines) {
        setScrollOffset(row - maxVisibleLines + 1);
      }
    },
    [scrollOffset, maxVisibleLines]
  );

  // Set cursor position with bounds checking
  const setCursor = useCallback(
    (row: number, col: number) => {
      const clampedRow = Math.max(0, Math.min(row, lines.length - 1));
      const lineLength = lines[clampedRow]?.length ?? 0;
      const clampedCol = Math.max(0, Math.min(col, lineLength));
      setCursorRow(clampedRow);
      setCursorCol(clampedCol);
      setLastColIntent(clampedCol);
      ensureCursorVisible(clampedRow);
    },
    [lines, ensureCursorVisible]
  );

  // Set value and reset cursor to end
  const setValue = useCallback(
    (newValue: string) => {
      const newLines = parseValue(newValue);
      setLines(newLines);
      const lastRow = newLines.length - 1;
      const lastCol = newLines[lastRow]?.length ?? 0;
      setCursorRow(lastRow);
      setCursorCol(lastCol);
      setLastColIntent(lastCol);
      setScrollOffset(Math.max(0, newLines.length - maxVisibleLines));
    },
    [parseValue, maxVisibleLines]
  );

  // Move cursor left
  const moveCursorLeft = useCallback(() => {
    if (cursorCol > 0) {
      setCursorCol(cursorCol - 1);
      setLastColIntent(cursorCol - 1);
    } else if (cursorRow > 0) {
      // Wrap to end of previous line
      const prevLineLength = lines[cursorRow - 1]?.length ?? 0;
      setCursorRow(cursorRow - 1);
      setCursorCol(prevLineLength);
      setLastColIntent(prevLineLength);
      ensureCursorVisible(cursorRow - 1);
    }
  }, [cursorCol, cursorRow, lines, ensureCursorVisible]);

  // Move cursor right
  const moveCursorRight = useCallback(() => {
    const currentLineLength = lines[cursorRow]?.length ?? 0;
    if (cursorCol < currentLineLength) {
      setCursorCol(cursorCol + 1);
      setLastColIntent(cursorCol + 1);
    } else if (cursorRow < lines.length - 1) {
      // Wrap to start of next line
      setCursorRow(cursorRow + 1);
      setCursorCol(0);
      setLastColIntent(0);
      ensureCursorVisible(cursorRow + 1);
    }
  }, [cursorCol, cursorRow, lines, ensureCursorVisible]);

  // Move cursor up
  const moveCursorUp = useCallback(() => {
    if (cursorRow > 0) {
      const targetRow = cursorRow - 1;
      const targetLineLength = lines[targetRow]?.length ?? 0;
      const targetCol = Math.min(lastColIntent, targetLineLength);
      setCursorRow(targetRow);
      setCursorCol(targetCol);
      ensureCursorVisible(targetRow);
    }
  }, [cursorRow, lines, lastColIntent, ensureCursorVisible]);

  // Move cursor down
  const moveCursorDown = useCallback(() => {
    if (cursorRow < lines.length - 1) {
      const targetRow = cursorRow + 1;
      const targetLineLength = lines[targetRow]?.length ?? 0;
      const targetCol = Math.min(lastColIntent, targetLineLength);
      setCursorRow(targetRow);
      setCursorCol(targetCol);
      ensureCursorVisible(targetRow);
    }
  }, [cursorRow, lines, lastColIntent, ensureCursorVisible]);

  // Move cursor to line start
  const moveCursorToLineStart = useCallback(() => {
    setCursorCol(0);
    setLastColIntent(0);
  }, []);

  // Move cursor to line end
  const moveCursorToLineEnd = useCallback(() => {
    const lineLength = lines[cursorRow]?.length ?? 0;
    setCursorCol(lineLength);
    setLastColIntent(lineLength);
  }, [lines, cursorRow]);

  // Move word left
  const moveWordLeft = useCallback(() => {
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

    setCursorCol(col);
    setLastColIntent(col);
  }, [lines, cursorRow, cursorCol]);

  // Move word right
  const moveWordRight = useCallback(() => {
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

    setCursorCol(col);
    setLastColIntent(col);
  }, [lines, cursorRow, cursorCol]);

  // Insert a single character - uses refs to avoid stale closures
  const linesRef = useRef(lines);
  linesRef.current = lines;
  const cursorRowRef = useRef(cursorRow);
  cursorRowRef.current = cursorRow;
  const cursorColRef = useRef(cursorCol);
  cursorColRef.current = cursorCol;

  const insertChar = useCallback((ch: string) => {
    const row = cursorRowRef.current;
    const col = cursorColRef.current;
    const currentLines = linesRef.current;
    const line = currentLines[row] ?? '';
    const newLine = line.slice(0, col) + ch + line.slice(col);
    const newLines = [...currentLines];
    newLines[row] = newLine;
    setLines(newLines);
    setCursorCol(col + ch.length);
    setLastColIntent(col + ch.length);
  }, []);

  // Insert a string (may contain newlines) - uses refs to avoid stale closures
  const insertString = useCallback(
    (str: string) => {
      const row = cursorRowRef.current;
      const col = cursorColRef.current;
      const currentLines = linesRef.current;
      const line = currentLines[row] ?? '';
      const before = line.slice(0, col);
      const after = line.slice(col);

      const insertedLines = str.split('\n');
      if (insertedLines.length === 1) {
        // Single line insert
        const newLine = before + str + after;
        const newLines = [...currentLines];
        newLines[row] = newLine;
        setLines(newLines);
        setCursorCol(col + str.length);
        setLastColIntent(col + str.length);
      } else {
        // Multi-line insert
        const firstLine = before + insertedLines[0];
        const lastLine = insertedLines[insertedLines.length - 1] + after;
        const middleLines = insertedLines.slice(1, -1);

        const newLines = [
          ...currentLines.slice(0, row),
          firstLine,
          ...middleLines,
          lastLine,
          ...currentLines.slice(row + 1),
        ];
        setLines(newLines);

        const newRow = row + insertedLines.length - 1;
        const newCol = insertedLines[insertedLines.length - 1].length;
        setCursorRow(newRow);
        setCursorCol(newCol);
        setLastColIntent(newCol);
        ensureCursorVisible(newRow);
      }
    },
    [ensureCursorVisible]
  );

  // Insert a newline (split current line)
  const insertNewline = useCallback(() => {
    const line = lines[cursorRow] ?? '';
    const before = line.slice(0, cursorCol);
    const after = line.slice(cursorCol);

    const newLines = [
      ...lines.slice(0, cursorRow),
      before,
      after,
      ...lines.slice(cursorRow + 1),
    ];
    setLines(newLines);
    setCursorRow(cursorRow + 1);
    setCursorCol(0);
    setLastColIntent(0);
    ensureCursorVisible(cursorRow + 1);
  }, [lines, cursorRow, cursorCol, ensureCursorVisible]);

  // Delete character before cursor
  const deleteCharBefore = useCallback(() => {
    if (cursorCol > 0) {
      // Delete character in current line
      const line = lines[cursorRow] ?? '';
      const newLine = line.slice(0, cursorCol - 1) + line.slice(cursorCol);
      const newLines = [...lines];
      newLines[cursorRow] = newLine;
      setLines(newLines);
      setCursorCol(cursorCol - 1);
      setLastColIntent(cursorCol - 1);
    } else if (cursorRow > 0) {
      // Merge with previous line
      const prevLine = lines[cursorRow - 1] ?? '';
      const currentLine = lines[cursorRow] ?? '';
      const mergedLine = prevLine + currentLine;
      const newCursorCol = prevLine.length;

      const newLines = [
        ...lines.slice(0, cursorRow - 1),
        mergedLine,
        ...lines.slice(cursorRow + 1),
      ];
      setLines(newLines);
      setCursorRow(cursorRow - 1);
      setCursorCol(newCursorCol);
      setLastColIntent(newCursorCol);
      ensureCursorVisible(cursorRow - 1);
    }
  }, [lines, cursorRow, cursorCol, ensureCursorVisible]);

  // Delete character at cursor
  const deleteCharAt = useCallback(() => {
    const line = lines[cursorRow] ?? '';
    if (cursorCol < line.length) {
      // Delete character at cursor
      const newLine = line.slice(0, cursorCol) + line.slice(cursorCol + 1);
      const newLines = [...lines];
      newLines[cursorRow] = newLine;
      setLines(newLines);
    } else if (cursorRow < lines.length - 1) {
      // Merge with next line
      const nextLine = lines[cursorRow + 1] ?? '';
      const mergedLine = line + nextLine;

      const newLines = [
        ...lines.slice(0, cursorRow),
        mergedLine,
        ...lines.slice(cursorRow + 2),
      ];
      setLines(newLines);
    }
  }, [lines, cursorRow, cursorCol]);

  // Delete word before cursor
  const deleteWordBefore = useCallback(() => {
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
      setLines(newLines);
      setCursorCol(col);
      setLastColIntent(col);
    }
  }, [lines, cursorRow, cursorCol]);

  // History navigation callbacks
  const handleHistoryPrev = useCallback(() => {
    if (onHistoryPrev) {
      onHistoryPrev();
    }
  }, [onHistoryPrev]);

  const handleHistoryNext = useCallback(() => {
    if (onHistoryNext) {
      onHistoryNext();
    }
  }, [onHistoryNext]);

  return {
    // State
    lines,
    cursorRow,
    cursorCol,
    scrollOffset,
    value,
    visibleLines,

    // Cursor Movement
    moveCursorLeft,
    moveCursorRight,
    moveCursorUp,
    moveCursorDown,
    moveCursorToLineStart,
    moveCursorToLineEnd,
    moveWordLeft,
    moveWordRight,

    // Text Editing
    insertChar,
    insertString,
    insertNewline,
    deleteCharBefore,
    deleteCharAt,
    deleteWordBefore,

    // State Setters
    setCursor,
    setValue,

    // History Callbacks
    handleHistoryPrev,
    handleHistoryNext,
  };
}
