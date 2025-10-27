/**
 * InputModeContext - Modal Input Mode Management
 *
 * Provides vim-style modal input modes (normal, insert, command) for TUI components.
 * Adapted from cage's modal input pattern for fspec's TUI infrastructure.
 *
 * Coverage: ITF-001 - Scenario: Modal input modes switch between normal and insert
 */

import React, { createContext, useContext, useState } from 'react';
import { useInput } from 'ink';

type InputMode = 'normal' | 'insert' | 'command';

interface InputModeContextValue {
  mode: InputMode;
  setMode: (mode: InputMode) => void;
  enterInsertMode: () => void;
  enterNormalMode: () => void;
}

const InputModeContext = createContext<InputModeContextValue | undefined>(
  undefined
);

interface InputModeProviderProps {
  children: React.ReactNode;
}

export const InputModeProvider: React.FC<InputModeProviderProps> = ({
  children,
}) => {
  const [mode, setMode] = useState<InputMode>('normal');

  const enterInsertMode = (): void => {
    setMode('insert');
  };

  const enterNormalMode = (): void => {
    setMode('normal');
  };

  // Global key handler for mode switching
  useInput((input, key) => {
    if (mode === 'normal' && input === 'i') {
      enterInsertMode();
    } else if (mode === 'insert' && key.escape) {
      enterNormalMode();
    }
  });

  const contextValue: InputModeContextValue = {
    mode,
    setMode,
    enterInsertMode,
    enterNormalMode,
  };

  return (
    <InputModeContext.Provider value={contextValue}>
      {children}
    </InputModeContext.Provider>
  );
};

export const useInputMode = (): InputModeContextValue => {
  const context = useContext(InputModeContext);
  if (!context) {
    throw new Error('useInputMode must be used within InputModeProvider');
  }
  return context;
};

/**
 * useSafeInput Hook
 *
 * Wrapper around Ink's useInput that respects input modes.
 * Only triggers callback when in the specified mode.
 */
export const useSafeInput = (
  callback: (input: string, key: { escape?: boolean }) => void,
  options: { isActive?: boolean; mode?: InputMode } = {}
): void => {
  const { mode: currentMode } = useInputMode();
  const { isActive = true, mode = 'normal' } = options;

  useInput((input, key) => {
    if (isActive && currentMode === mode) {
      callback(input, key);
    }
  });
};
