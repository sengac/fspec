/**
 * TUI-075: Default Thinking Level Hook
 *
 * Manages default thinking level for sessions:
 * - Loads default from config on mount
 * - Applies default to every session when it becomes active
 * - Provides setter for updating the default
 *
 * Single Responsibility: Encapsulates all default thinking level behavior.
 */

import { useState, useEffect, useRef, useCallback } from 'react';
import type { JsThinkingLevel } from '@sengac/codelet-napi';
import { getRustStateSource } from './rustStateSource';
import {
  loadDefaultThinkingLevel,
  saveDefaultThinkingLevel,
} from '../config/defaultThinkingLevelConfig';
import { logger } from '../../utils/logger';

interface UseDefaultThinkingLevelOptions {
  sessionId: string | null;
  refreshRustState: () => void;
}

interface UseDefaultThinkingLevelResult {
  /** Current default thinking level (null if not yet loaded) */
  defaultLevel: JsThinkingLevel | null;
  /** Update the persisted default and apply to current session */
  setDefault: (level: JsThinkingLevel) => Promise<void>;
}

/**
 * Hook that manages default thinking level for sessions.
 *
 * Automatically applies the default level to each session when it becomes active,
 * whether created new or resumed via /resume.
 */
export function useDefaultThinkingLevel({
  sessionId,
  refreshRustState,
}: UseDefaultThinkingLevelOptions): UseDefaultThinkingLevelResult {
  const [defaultLevel, setDefaultLevel] = useState<JsThinkingLevel | null>(
    null
  );
  const appliedToSessionRef = useRef<string | null>(null);

  // Load default from config on mount
  useEffect(() => {
    const load = async () => {
      const level = await loadDefaultThinkingLevel();
      setDefaultLevel(level);
    };
    void load();
  }, []);

  // Apply default to session when session changes or default loads
  useEffect(() => {
    if (
      defaultLevel !== null &&
      sessionId &&
      appliedToSessionRef.current !== sessionId
    ) {
      appliedToSessionRef.current = sessionId;
      getRustStateSource().setBaseThinkingLevel(sessionId, defaultLevel);
      refreshRustState();
      logger.debug(
        `TUI-075: Applied default thinking level ${defaultLevel} to session ${sessionId}`
      );
    }
  }, [defaultLevel, sessionId, refreshRustState]);

  // Update default and apply to current session
  const setDefault = useCallback(
    async (level: JsThinkingLevel) => {
      await saveDefaultThinkingLevel(level);
      setDefaultLevel(level);
      // Apply immediately to current session
      if (sessionId) {
        getRustStateSource().setBaseThinkingLevel(sessionId, level);
        refreshRustState();
      }
    },
    [sessionId, refreshRustState]
  );

  return { defaultLevel, setDefault };
}
