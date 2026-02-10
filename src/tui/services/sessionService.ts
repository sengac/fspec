/**
 * sessionService.ts - Session Creation and Management Service
 *
 * VIEWNV-001: Provides clean, reusable functions for session operations.
 * These are pure async functions (no React hooks) that handle Rust/persistence interactions.
 *
 * This follows SOLID principles:
 * - Single Responsibility: Only handles session creation/management
 * - Open/Closed: Easy to extend with new operations
 * - DRY: Reusable across CreateSessionDialog, /resume, navigation, etc.
 *
 * CONFIG-004: Integrated with credentials system to pass API keys to Rust NAPI.
 * Credentials are loaded from ~/.fspec/credentials/credentials.json when creating sessions.
 */

import {
  sessionManagerCreateWithId,
  sessionManagerList,
  sessionRestoreMessages,
  sessionRestoreTokenState,
  sessionRestoreAnchorPoints,
  sessionAttach,
  persistenceCreateSessionWithProvider,
  persistenceLoadSession,
  persistenceGetSessionMessageEnvelopes,
} from '@sengac/codelet-napi';
import type { StreamChunk } from '@sengac/codelet-napi';
import { logger } from '../../utils/logger';
import { getProviderConfig } from '../../utils/credentials';

/**
 * Extract provider ID from model path.
 * e.g., "anthropic/claude-sonnet-4" -> "anthropic"
 *
 * CONFIG-004: Used to look up credentials for the correct provider.
 */
function extractProviderId(modelPath: string): string {
  return modelPath.split('/')[0];
}

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
 * Create a new session in both persistence and Rust background.
 * This is the canonical way to create a session that's immediately usable.
 *
 * CONFIG-004: Now reads credentials from ~/.fspec/credentials/credentials.json
 * and passes them to Rust NAPI for provider authentication.
 *
 * @returns The created session info
 * @throws If session creation fails
 */
export async function createSession(
  options: CreateSessionOptions
): Promise<CreateSessionResult> {
  const { modelPath, project, name } = options;
  const sessionName = name || `New Session ${new Date().toLocaleString()}`;

  logger.debug(
    `[SessionService] Creating new session: ${sessionName}, provider: ${modelPath}`
  );

  // CONFIG-004: Get credentials for this provider from credentials file
  const providerId = extractProviderId(modelPath);
  const providerConfig = await getProviderConfig(providerId);
  const apiKey = providerConfig.apiKey;

  if (apiKey) {
    logger.debug(
      `[SessionService] Found credentials for ${providerId} (source: ${providerConfig.source})`
    );
  } else {
    logger.debug(
      `[SessionService] No credentials found for ${providerId}, Rust will use env vars`
    );
  }

  // Create persisted session first (gives us the ID)
  const persistedSession = persistenceCreateSessionWithProvider(
    sessionName,
    project,
    modelPath
  );

  // Create Rust background session with the same ID
  // CONFIG-004: Pass API key from credentials file to Rust
  await sessionManagerCreateWithId(
    persistedSession.id,
    modelPath,
    project,
    sessionName,
    apiKey // undefined if not found - Rust falls back to env vars
  );

  logger.debug(`[SessionService] Created session ${persistedSession.id}`);

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
    logger.debug(
      `[SessionService] Session ${sessionId} already exists in background`
    );

    // Attach for streaming if callback provided
    if (onStreamChunk) {
      sessionAttach(sessionId, (_err: Error | null, chunk: StreamChunk) => {
        if (chunk) {
          onStreamChunk(chunk);
        }
      });
    }

    return {
      sessionId,
      name: bgSession.name || 'Session',
      provider: bgSession.model || fallbackModelPath,
      wasBackgroundSession: true,
    };
  }

  logger.debug(`[SessionService] Restoring persisted session ${sessionId}`);

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
    // Use provided session data if available
    sessionManifest = {
      provider: sessionData.provider || fallbackModelPath,
      name: sessionData.name || 'Restored Session',
      tokenUsage: sessionData.tokenUsage,
    };
  } else {
    // Fall back to loading from persistence
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

  // CONFIG-004: Get credentials for this provider from credentials file
  const providerId = extractProviderId(modelPath);
  const providerConfig = await getProviderConfig(providerId);
  const apiKey = providerConfig.apiKey;

  // Create background session
  try {
    // CONFIG-004: Pass API key from credentials file to Rust
    await sessionManagerCreateWithId(
      sessionId,
      modelPath,
      fallbackProject,
      sessionName,
      apiKey // undefined if not found - Rust falls back to env vars
    );
  } catch (err) {
    // Session may already exist - this is actually OK, but log for debugging
    logger.debug(
      `[SessionService] Session ${sessionId} may already exist in background:`,
      err
    );
  }

  // Restore messages from persistence
  const envelopes: string[] = persistenceGetSessionMessageEnvelopes(sessionId);
  await sessionRestoreMessages(sessionId, envelopes);

  // TUI-056: Restore anchor points from persistence
  // This ensures /anchors shows correct history after session resume
  try {
    const restoredAnchors = sessionRestoreAnchorPoints(sessionId);
    if (restoredAnchors > 0) {
      logger.debug(
        `[SessionService] Restored ${restoredAnchors} anchor points for session ${sessionId}`
      );
    }
  } catch (err) {
    // Don't fail session restore if anchor restore fails - it's not critical
    logger.warn(
      `[SessionService] Failed to restore anchor points for ${sessionId}:`,
      err
    );
  }

  // Restore token state if available
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

  // Attach for streaming if callback provided
  if (onStreamChunk) {
    sessionAttach(sessionId, (_err: Error | null, chunk: StreamChunk) => {
      if (chunk) {
        onStreamChunk(chunk);
      }
    });
  }

  logger.debug(`[SessionService] Restored session ${sessionId}`);

  return {
    sessionId,
    name: sessionName,
    provider: modelPath,
    tokenUsage: sessionManifest?.tokenUsage,
    wasBackgroundSession: false,
  };
}
