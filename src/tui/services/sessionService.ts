/**
 * sessionService.ts - Session Creation and Management Service
 *
 * Provides clean, reusable functions for session operations.
 * These are pure async functions (no React hooks) that handle Rust/persistence interactions.
 */

import {
  sessionManagerCreateWithId,
  sessionManagerList,
  sessionRestoreMessages,
  sessionRestoreTokenState,
  sessionRestoreAnchorPoints,
  persistenceCreateSessionWithProvider,
  persistenceLoadSession,
  persistenceGetSessionMessageEnvelopes,
} from '@sengac/codelet-napi';
import type { StreamChunk } from '@sengac/codelet-napi';
import { logger } from '../../utils/logger';
import { GlobalSessionStreamManager } from './globalSessionStreamManager';

/**
 * Result of creating a new session
 */
export interface CreateSessionResult {
  sessionId: string;
  name: string;
  provider: string;
}

/**
 * Options for creating a new session
 */
export interface CreateSessionOptions {
  /** Model path (e.g., "anthropic/claude-sonnet-4-20250514") */
  modelPath: string;
  /** Project/working directory */
  project: string;
  /** Optional session name (defaults to timestamp-based name) */
  name?: string;
}

/**
 * Extract provider ID from model path.
 * Model paths are in "provider/model-id" format (e.g., "anthropic/claude-sonnet-4").
 *
 * @param modelPath - Full model path
 * @returns Provider ID (e.g., "anthropic")
 */
export function extractProviderId(modelPath: string): string {
  const parts = modelPath.split('/');
  return parts[0] || '';
}

/**
 * Create a new session in both persistence and Rust background.
 * This is the canonical way to create a session that's immediately usable.
 * Credentials are resolved internally by Rust.
 *
 * @returns The created session info
 * @throws If session creation fails
 */
export async function createSession(
  options: CreateSessionOptions
): Promise<CreateSessionResult> {
  const { modelPath, project, name } = options;
  const sessionName = name || `New Session ${new Date().toLocaleString()}`;

  // Create persisted session first (gives us the ID)
  const persistedSession = persistenceCreateSessionWithProvider(
    sessionName,
    project,
    modelPath
  );

  // Create Rust background session with the same ID
  await sessionManagerCreateWithId(
    persistedSession.id,
    modelPath,
    project,
    sessionName
  );

  const manager = GlobalSessionStreamManager.getInstance();
  manager.subscribeToSession(persistedSession.id);

  return {
    sessionId: persistedSession.id,
    name: sessionName,
    provider: modelPath,
  };
}

/**
 * Result of restoring a session from persistence
 */
export interface RestoreSessionResult {
  sessionId: string;
  name: string;
  provider: string;
  tokenUsage?: {
    currentContextTokens: number;
    cumulativeBilledOutput: number;
    cacheReadTokens?: number;
    cacheCreationTokens?: number;
    cumulativeBilledInput?: number;
  };
  wasBackgroundSession: boolean;
  unregister?: () => void;
}

/**
 * Options for restoring a session
 */
export interface RestoreSessionOptions {
  /** Session ID to restore */
  sessionId: string;
  /** Fallback model path if not in manifest */
  fallbackModelPath: string;
  /** Fallback project if not in manifest */
  fallbackProject: string;
  /** Callback for stream chunks (for attaching) */
  onStreamChunk?: (chunk: StreamChunk) => void;
  /** Optional session data (if already available, avoids persistence lookup) */
  sessionData?: {
    name?: string;
    provider?: string;
    tokenUsage?: {
      currentContextTokens: number;
      cumulativeBilledOutput: number;
      cacheReadTokens?: number;
      cacheCreationTokens?: number;
      cumulativeBilledInput?: number;
    };
  };
}

/**
 * Restore a session from persistence to Rust background.
 * If the session already exists in Rust (background session), just returns its info.
 * Credentials are resolved internally by Rust.
 *
 * This handles:
 * 1. Loading session manifest from persistence
 * 2. Creating Rust background session if needed
 * 3. Restoring messages from persistence
 * 4. Restoring token state
 * 5. Attaching for live streaming
 *
 * @returns The restored session info
 * @throws If session restoration fails
 */
export async function restoreSession(
  options: RestoreSessionOptions
): Promise<RestoreSessionResult> {
  const {
    sessionId,
    fallbackModelPath,
    fallbackProject,
    onStreamChunk,
    sessionData,
  } = options;

  // Check if this is already a background session
  const backgroundSessions = sessionManagerList();
  const bgSession = backgroundSessions.find(bg => bg.id === sessionId);

  if (bgSession) {
    const manager = GlobalSessionStreamManager.getInstance();
    manager.subscribeToSession(sessionId);

    let unregister: (() => void) | undefined;
    if (onStreamChunk) {
      unregister = manager.registerHandler(sessionId, onStreamChunk);
    }

    return {
      sessionId,
      name: bgSession.name || 'Session',
      provider: bgSession.model || fallbackModelPath,
      wasBackgroundSession: true,
      unregister,
    };
  }

  // Load session manifest from persistence (if not provided)
  let sessionManifest: {
    provider: string;
    name: string;
    tokenUsage?: {
      currentContextTokens: number;
      cumulativeBilledOutput: number;
      cacheReadTokens?: number;
      cacheCreationTokens?: number;
      cumulativeBilledInput?: number;
    };
  } | null = null;

  if (sessionData) {
    sessionManifest = {
      provider: sessionData.provider || fallbackModelPath,
      name: sessionData.name || 'Restored Session',
      tokenUsage: sessionData.tokenUsage,
    };
  } else {
    try {
      sessionManifest = persistenceLoadSession(sessionId);
    } catch (err) {
      logger.error(
        `[SessionService] Could not load session manifest for ${sessionId}:`,
        err
      );
    }
  }

  const modelPath = sessionManifest?.provider || fallbackModelPath;
  const sessionName = sessionManifest?.name || 'Restored Session';

  try {
    await sessionManagerCreateWithId(
      sessionId,
      modelPath,
      fallbackProject,
      sessionName
    );
  } catch {
    // Session may already exist
  }

  const envelopes: string[] = persistenceGetSessionMessageEnvelopes(sessionId);
  await sessionRestoreMessages(sessionId, envelopes);

  try {
    sessionRestoreAnchorPoints(sessionId);
  } catch (err) {
    logger.error(
      `[SessionService] Failed to restore anchor points for ${sessionId}:`,
      err
    );
  }

  if (sessionManifest?.tokenUsage) {
    await sessionRestoreTokenState(
      sessionId,
      sessionManifest.tokenUsage.currentContextTokens,
      sessionManifest.tokenUsage.cumulativeBilledOutput,
      sessionManifest.tokenUsage.cacheReadTokens ?? 0,
      sessionManifest.tokenUsage.cacheCreationTokens ?? 0,
      sessionManifest.tokenUsage.cumulativeBilledInput ?? 0,
      sessionManifest.tokenUsage.cumulativeBilledOutput
    );
  }

  const manager = GlobalSessionStreamManager.getInstance();
  manager.subscribeToSession(sessionId);

  let unregister: (() => void) | undefined;
  if (onStreamChunk) {
    unregister = manager.registerHandler(sessionId, onStreamChunk);
  }

  return {
    sessionId,
    name: sessionName,
    provider: modelPath,
    tokenUsage: sessionManifest?.tokenUsage,
    wasBackgroundSession: false,
    unregister,
  };
}
