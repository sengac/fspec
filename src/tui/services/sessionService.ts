/**
 * sessionService.ts - Session Creation and Management Service
 *
 * Provides clean, reusable functions for session operations.
 * These are pure async functions (no React hooks) that handle Rust/persistence interactions.
 */

import {
  sessionManagerCreateWithId,
  sessionManagerCreateIsolated,
  sessionManagerList,
  sessionManagerDestroy,
  sessionRestoreMessages,
  sessionRestoreTokenState,
  sessionSetModel,
  sessionSetModelProfile,
  persistenceCreateSessionWithProvider,
  persistenceLoadSession,
  persistenceGetSessionMessageEnvelopes,
  listSessions,
  inspectSession,
  mergeSession,
  discardSession,
  pruneOrphaned,
} from '@sengac/codelet-napi';
import { isCustomProviderSection } from './customProviderSectionBuilder';
import type {
  SessionInfoJs,
  SessionResultJs,
  MergeResultJs,
  DiscardResultJs,
  PruneResultJs,
} from '@sengac/codelet-napi';
import type { StreamChunk } from '@sengac/codelet-napi';
import { logger } from '../../utils/logger';
import { GlobalSessionStreamManager } from './globalSessionStreamManager';
import { useFspecStore } from '../store/fspecStore';
import { useSessionStore } from '../store/sessionStore';
import { setWorkUnitContext } from './workUnitContextService';

/**
 * Result of creating a new session
 */
export interface CreateSessionResult {
  sessionId: string;
  name: string;
  provider: string;
}

/**
 * Result of creating an isolated session (GIT-029)
 */
export interface CreateIsolatedSessionResult extends CreateSessionResult {
  /** Path to the git worktree created for this session */
  worktreePath: string;
  /** Base commit SHA the worktree was created from */
  baseCommit: string;
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
  /** If true, create an isolated session with a git worktree (GIT-029) */
  isolated?: boolean;
  /** MODEL-005: Optional ModelSelection to propagate context window and max output tokens */
  modelSelection?: {
    /** Provider ID (e.g., "anthropic", "openai") */
    providerId: string;
    /** Model ID (e.g., "claude-sonnet-4") */
    modelId: string;
    /** Context window size in tokens */
    contextWindow: number;
    /** Maximum output tokens */
    maxOutput: number;
    /** Profile config if model is from a local profile (PROV-007) */
    profileConfig?: { baseUrl: string };
    /** MODEL-004: Facade override for custom model dispatch */
    facade?: string;
    /**
     * BUG-137: Profile name (e.g., "fireworks") for profile-qualified
     * selections. Passed through to `sessionSetModelProfile` so the
     * subordinate AgentManager.spawn round-trip captures the full
     * "provider:profile/model" composite.
     */
    profileName?: string;
  };
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
 * MODEL-005: After creating the Rust session, propagates per-model context window
 * and max output tokens from ModelSelection to the ProviderManager via NAPI.
 * This ensures compaction uses the correct per-model limits from the start.
 *
 * @returns The created session info
 * @throws If session creation fails
 */
export async function createSession(
  options: CreateSessionOptions
): Promise<CreateSessionResult> {
  const { modelPath, project, name, modelSelection } = options;
  const sessionName = name || `New Session ${new Date().toLocaleString()}`;

  logger.warn('[sessionService] createSession ENTER', {
    modelPath,
    project,
    name: sessionName,
    hasModelSelection: !!modelSelection,
    providerId: modelSelection?.providerId,
    modelId: modelSelection?.modelId,
    hasProfileConfig: !!modelSelection?.profileConfig,
    isCustom: modelSelection
      ? isCustomProviderSection(modelSelection.providerId)
      : undefined,
  });

  // Create persisted session first (gives us the ID)
  const persistedSession = persistenceCreateSessionWithProvider(
    sessionName,
    project,
    modelPath
  );

  logger.warn('[sessionService] persistenceCreateSessionWithProvider done', {
    sessionId: persistedSession.id,
    modelPath,
  });

  // GIT-029: Subscribe BEFORE creating Rust session to catch IsolationStateChange chunk
  // The chunk is emitted during session creation, so we must be subscribed first
  const manager = GlobalSessionStreamManager.getInstance();
  manager.subscribeToSession(persistedSession.id);

  // Create Rust background session with the same ID
  logger.warn('[sessionService] calling sessionManagerCreateWithId', {
    sessionId: persistedSession.id,
    modelPath,
  });
  await sessionManagerCreateWithId(
    persistedSession.id,
    modelPath,
    project,
    sessionName
  );
  logger.warn('[sessionService] sessionManagerCreateWithId OK', {
    sessionId: persistedSession.id,
  });

  // MODEL-005: Propagate per-model context window and max output tokens to ProviderManager.
  // sessionManagerCreateWithId doesn't accept these parameters, so we push them
  // via sessionSetModel/sessionSetModelProfile after session creation.
  if (modelSelection) {
    try {
      if (
        modelSelection.profileConfig ||
        modelSelection.providerId === 'codex' ||
        isCustomProviderSection(modelSelection.providerId)
      ) {
        // PROV-096: Custom (Rhai-scripted / facade-based) providers are not
        // in the models.dev registry, so they MUST go through
        // sessionSetModelProfile which calls `set_model_direct` and
        // bypasses registry validation. sessionSetModel would call
        // `select_model()` and fail with "Unknown provider:
        // '<custom-slug>'" for any non-builtin provider.
        logger.warn(
          '[sessionService] routing custom/profile provider via sessionSetModelProfile',
          {
            sessionId: persistedSession.id,
            providerId: modelSelection.providerId,
            modelId: modelSelection.modelId,
            facade: modelSelection.facade ?? null,
          }
        );
        await sessionSetModelProfile(
          persistedSession.id,
          modelSelection.providerId,
          modelSelection.modelId,
          modelSelection.contextWindow,
          modelSelection.maxOutput,
          modelSelection.facade ?? null,
          null,
          null,
          // BUG-137: Pass profile name (only present on profile-based
          // selections) so AgentManager.spawn later captures the full
          // "provider:profile/model" composite.
          modelSelection.profileConfig
            ? (modelSelection.profileName ?? null)
            : null
        );
      } else {
        logger.warn(
          '[sessionService] routing builtin provider via sessionSetModel',
          {
            sessionId: persistedSession.id,
            providerId: modelSelection.providerId,
            modelId: modelSelection.modelId,
          }
        );
        await sessionSetModel(
          persistedSession.id,
          modelSelection.providerId,
          modelSelection.modelId,
          modelSelection.contextWindow,
          modelSelection.maxOutput
        );
      }
    } catch (err) {
      // MODEL-005: Log but don't fail — session is usable with provider-constant fallback
      logger.error('MODEL-005: Failed to propagate model limits to session', {
        error: err,
      });
    }
  }

  logger.warn('[sessionService] createSession EXIT ok', {
    sessionId: persistedSession.id,
    provider: modelPath,
  });

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
  /** Callback for stream chunks (for attaching). CMPCT-033: receives routed sessionId. */
  onStreamChunk?: (routedSessionId: string, chunk: StreamChunk) => void;
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

  // GIT-029: Subscribe BEFORE creating Rust session to catch IsolationStateChange chunk
  // The chunk is emitted during session creation, so we must be subscribed first
  const manager = GlobalSessionStreamManager.getInstance();
  manager.subscribeToSession(sessionId);

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

// ========================================
// GIT-029: Isolated Session Management
// ========================================

/**
 * Create an isolated session with a git worktree (GIT-029)
 *
 * Creates a session that operates in an isolated git worktree,
 * allowing the AI agent to make file changes without affecting the main project.
 * The worktree is created at `.fspec/worktrees/<session-id>/`.
 *
 * MODEL-005: After creating the Rust session, propagates per-model context window
 * and max output tokens from ModelSelection to the ProviderManager via NAPI.
 *
 * @returns The created session info with worktree path
 * @throws If session creation fails
 */
export async function createIsolatedSession(
  options: CreateSessionOptions
): Promise<CreateIsolatedSessionResult> {
  const { modelPath, project, name, modelSelection } = options;
  const sessionName = name || `Isolated Session ${new Date().toLocaleString()}`;

  // Create persisted session first (gives us the ID)
  const persistedSession = persistenceCreateSessionWithProvider(
    sessionName,
    project,
    modelPath
  );

  // GIT-029: Subscribe BEFORE creating Rust session to catch IsolationStateChange chunk
  // The chunk is emitted during session creation, so we must be subscribed first
  const manager = GlobalSessionStreamManager.getInstance();
  manager.subscribeToSession(persistedSession.id);

  // Create isolated Rust background session with git worktree
  const isolatedResult = await sessionManagerCreateIsolated(
    persistedSession.id,
    modelPath,
    project,
    sessionName
  );

  // MODEL-005: Propagate per-model context window and max output tokens to ProviderManager.
  // sessionManagerCreateIsolated doesn't accept these parameters, so we push them
  // via sessionSetModel/sessionSetModelProfile after session creation.
  if (modelSelection) {
    try {
      if (
        modelSelection.profileConfig ||
        modelSelection.providerId === 'codex' ||
        isCustomProviderSection(modelSelection.providerId)
      ) {
        // PROV-096: see note in createSession() above — custom providers
        // must bypass the models.dev registry via sessionSetModelProfile.
        await sessionSetModelProfile(
          persistedSession.id,
          modelSelection.providerId,
          modelSelection.modelId,
          modelSelection.contextWindow,
          modelSelection.maxOutput,
          modelSelection.facade ?? null,
          null,
          null,
          // BUG-137: Pass profile name (only present on profile-based
          // selections) so AgentManager.spawn later captures the full
          // "provider:profile/model" composite.
          modelSelection.profileConfig
            ? (modelSelection.profileName ?? null)
            : null
        );
      } else {
        await sessionSetModel(
          persistedSession.id,
          modelSelection.providerId,
          modelSelection.modelId,
          modelSelection.contextWindow,
          modelSelection.maxOutput
        );
      }
    } catch (err) {
      // MODEL-005: Log but don't fail — session is usable with provider-constant fallback
      logger.error(
        'MODEL-005: Failed to propagate model limits to isolated session',
        { error: err }
      );
    }
  }

  return {
    sessionId: persistedSession.id,
    name: sessionName,
    provider: modelPath,
    worktreePath: isolatedResult.worktreePath,
    baseCommit: isolatedResult.baseCommit,
  };
}

/**
 * List all session worktrees with status information (GIT-029)
 *
 * Returns information about all session worktrees, optionally filtered by status.
 *
 * @param repoPath - Path to the git repository
 * @param activeSessions - Array of currently active session IDs
 * @param filter - Optional filter: "all", "active", "pending_merge", "clean", "orphaned"
 * @returns Array of session info objects with status
 */
export function listSessionWorktrees(
  repoPath: string,
  activeSessions: string[],
  filter?: string
): SessionInfoJs[] {
  return listSessions(repoPath, activeSessions, filter);
}

/**
 * Inspect a session's changes without side effects (GIT-029)
 *
 * Returns diff information for a session worktree without modifying
 * the worktree or any session state.
 *
 * @param repoPath - Path to the git repository
 * @param sessionId - Session identifier
 * @returns Session result with unified diff and file lists
 */
export function inspectSessionChanges(
  repoPath: string,
  sessionId: string
): SessionResultJs {
  return inspectSession(repoPath, sessionId);
}

/**
 * Merge session changes to main worktree (GIT-029)
 *
 * Applies all changes from session to main worktree and removes
 * the session worktree on success.
 *
 * @param repoPath - Path to the git repository
 * @param sessionId - Session identifier
 * @returns Merge result with file lists
 * @throws Error with "Conflict" if main has conflicting changes
 */
export function mergeSessionChanges(
  repoPath: string,
  sessionId: string
): MergeResultJs {
  return mergeSession(repoPath, sessionId);
}

/**
 * Discard session changes without applying (GIT-029)
 *
 * Removes the session worktree without applying any changes
 * to the main worktree.
 *
 * @param repoPath - Path to the git repository
 * @param sessionId - Session identifier
 * @returns Discard result with count of files discarded
 */
export function discardSessionChanges(
  repoPath: string,
  sessionId: string
): DiscardResultJs {
  return discardSession(repoPath, sessionId);
}

/**
 * Prune orphaned session worktrees (GIT-029)
 *
 * Removes all worktrees that have no valid session record.
 * Active sessions are never pruned.
 *
 * @param repoPath - Path to the git repository
 * @param activeSessions - Array of currently active session IDs
 * @returns Prune result with count and list of pruned session IDs
 */
export function pruneOrphanedSessions(
  repoPath: string,
  activeSessions: string[]
): PruneResultJs {
  return pruneOrphaned(repoPath, activeSessions);
}

// ========================================
// TUI-068: Session Lifecycle Facade
// ========================================

/**
 * Destroy a session and clean up all associated state (TUI-068)
 *
 * Orchestrates the complete cleanup of a session:
 * 1. Destroys the Rust background session
 * 2. Detaches from any associated work unit in fspecStore
 * 3. Clears the current work unit in sessionStore
 * 4. Unsubscribes from the stream manager
 *
 * @param sessionId - Session identifier to destroy
 */
export async function destroySession(sessionId: string): Promise<void> {
  // Get the work unit this session is attached to (if any)
  const fspecState = useFspecStore.getState();
  const workUnitId = fspecState.getWorkUnitBySession(sessionId);

  // 1. Destroy the Rust background session
  try {
    sessionManagerDestroy(sessionId);
  } catch (err) {
    logger.error(
      `[SessionService] Failed to destroy Rust session ${sessionId}:`,
      err
    );
  }

  // 2. Detach from work unit in fspecStore
  if (workUnitId) {
    fspecState.detachSession(workUnitId);
  }

  // 3. Clear current work unit in sessionStore
  const sessionState = useSessionStore.getState();
  sessionState.setCurrentWorkUnit(null, null);

  // 4. Unsubscribe from stream manager
  const manager = GlobalSessionStreamManager.getInstance();
  manager.unsubscribeFromSession(sessionId);
}

/**
 * Attach a session to a work unit (TUI-068)
 *
 * Orchestrates the attachment of a session to a work unit:
 * 1. Updates fspecStore.sessionAttachments
 * 2. Updates sessionStore.currentWorkUnitId
 * 3. Sets the work unit context in Rust via workUnitContextService
 *
 * TUI-069: Added error handling with rollback on failure
 *
 * @param sessionId - Session identifier
 * @param workUnitId - Work unit identifier to attach to
 * @param status - Work unit status (e.g., "specifying", "implementing")
 * @param title - Optional work unit title (defaults to workUnitId if not provided)
 */
export function attachToWorkUnit(
  sessionId: string,
  workUnitId: string,
  status: string,
  title?: string
): void {
  const fspecState = useFspecStore.getState();
  const sessionState = useSessionStore.getState();

  // Track previous state for rollback
  const previousWorkUnitId = fspecState.getWorkUnitBySession(sessionId);
  const previousSessionStoreWorkUnit = sessionState.currentWorkUnitId;
  const previousSessionStoreStatus = sessionState.currentWorkUnitStatus;

  try {
    // 1. Update fspecStore.sessionAttachments
    fspecState.attachSession(workUnitId, sessionId);

    // 2. Update sessionStore.currentWorkUnitId
    sessionState.setCurrentWorkUnit(workUnitId, status);

    // 3. Set work unit context in Rust
    setWorkUnitContext(sessionId, {
      id: workUnitId,
      title: title ?? workUnitId,
      status,
    });
  } catch (err) {
    // TUI-069: Rollback on failure
    logger.error(
      `[SessionService] Failed to attach session ${sessionId} to ${workUnitId}, rolling back:`,
      err
    );

    // Rollback fspecStore
    if (previousWorkUnitId) {
      fspecState.attachSession(previousWorkUnitId, sessionId);
    } else {
      fspecState.detachSession(workUnitId);
    }

    // Rollback sessionStore
    sessionState.setCurrentWorkUnit(
      previousSessionStoreWorkUnit,
      previousSessionStoreStatus
    );

    throw err;
  }
}

/**
 * Get the work unit attached to a session (TUI-069)
 *
 * Provides read access to session-work unit attachments through the facade,
 * avoiding direct store access in components.
 *
 * @param sessionId - Session identifier
 * @returns Work unit ID if attached, undefined otherwise
 */
export function getAttachedWorkUnit(sessionId: string): string | undefined {
  return useFspecStore.getState().getWorkUnitBySession(sessionId);
}

/**
 * Detach a session from its current work unit (TUI-068)
 *
 * Orchestrates the detachment of a session from a work unit:
 * 1. Removes from fspecStore.sessionAttachments
 * 2. Clears sessionStore.currentWorkUnitId
 * 3. Clears the work unit context in Rust via workUnitContextService
 *
 * TUI-069: Added error handling with rollback on failure
 *
 * @param sessionId - Session identifier to detach
 */
export function detachFromWorkUnit(sessionId: string): void {
  const fspecState = useFspecStore.getState();
  const sessionState = useSessionStore.getState();

  // Get the work unit this session is attached to (for potential rollback)
  const workUnitId = fspecState.getWorkUnitBySession(sessionId);
  const previousSessionStoreWorkUnit = sessionState.currentWorkUnitId;
  const previousSessionStoreStatus = sessionState.currentWorkUnitStatus;

  try {
    // 1. Remove from fspecStore.sessionAttachments
    if (workUnitId) {
      fspecState.detachSession(workUnitId);
    }

    // 2. Clear sessionStore.currentWorkUnitId
    sessionState.setCurrentWorkUnit(null, null);

    // 3. Clear work unit context in Rust
    setWorkUnitContext(sessionId, null);
  } catch (err) {
    // TUI-069: Rollback on failure
    logger.error(
      `[SessionService] Failed to detach session ${sessionId}, rolling back:`,
      err
    );

    // Rollback fspecStore
    if (workUnitId) {
      fspecState.attachSession(workUnitId, sessionId);
    }

    // Rollback sessionStore
    sessionState.setCurrentWorkUnit(
      previousSessionStoreWorkUnit,
      previousSessionStoreStatus
    );

    throw err;
  }
}
