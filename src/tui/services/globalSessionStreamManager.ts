/**
 * Global Session Stream Manager
 *
 * BRIDGE-012: Refactored to use a single global callback for all sessions.
 * Rust emits ALL chunks from ALL sessions through one callback with (session_id, chunk).
 * TypeScript owns all routing logic via session_id.
 *
 * AgentView registers with this manager via registerHandler().
 *
 * FspecCommandRequest handling is done globally - when any session emits a
 * FspecCommandRequest, this manager handles it, preventing deadlocks when
 * users navigate away from sessions.
 */

import { logger } from '../../utils/logger';
import type {
  StreamChunk,
  GlobalChunkCallbackArgs,
} from '@sengac/codelet-napi';
import { useSessionStore } from '../store/sessionStore';
import { useFooterStore } from '../store/footerStore';

export type SessionChunkHandler = (chunk: StreamChunk) => void;
export type GlobalChunkHandler = (
  sessionId: string,
  chunk: StreamChunk
) => void;

interface NapiModule {
  // BRIDGE-012: Global callback - registered ONCE at startup
  sessionSetGlobalChunkCallback: (
    callback: (err: Error | null, args: GlobalChunkCallbackArgs) => void
  ) => void;
  sessionSendFspecResult: (
    sessionId: string,
    result: {
      success: boolean;
      data: string;
      error?: string;
      systemReminder?: string;
      toolCallId: string;
    }
  ) => void;
}

let napiModule: NapiModule | null = null;
let globalCallbackRegistered = false;

async function getNapiModule(): Promise<NapiModule> {
  if (!napiModule) {
    const napi = await import('@sengac/codelet-napi');
    napiModule = {
      sessionSetGlobalChunkCallback: napi.sessionSetGlobalChunkCallback,
      sessionSendFspecResult: napi.sessionSendFspecResult,
    };
  }
  return napiModule;
}

/**
 * GlobalSessionStreamManager
 *
 * BRIDGE-012: Singleton that manages all session stream subscriptions via global callback.
 * Rust emits all chunks through ONE global callback, TypeScript routes by session_id.
 */
/**
 * Pending isolation state for sessions that haven't been activated yet.
 * GIT-029: When IsolationStateChange arrives before activateSession is called,
 * we store the state here and apply it when the session is activated.
 */
export interface PendingIsolationState {
  isIsolated: boolean;
  worktreePath: string | null;
}

export class GlobalSessionStreamManager {
  private static instance: GlobalSessionStreamManager | null = null;
  private sessionHandlers: Map<string, Set<SessionChunkHandler>> = new Map();
  private globalHandlers: Set<GlobalChunkHandler> = new Set();
  private subscribedSessions: Set<string> = new Set();
  private fspecCallback:
    | ((
        command: string,
        argsJson: string,
        projectRoot: string
      ) => Promise<string>)
    | null = null;

  /**
   * GIT-029: Pending isolation state for sessions not yet activated.
   * Key: sessionId, Value: isolation state to apply when session activates
   */
  private pendingIsolationState: Map<string, PendingIsolationState> = new Map();

  private constructor() {}

  /**
   * Get the singleton instance
   */
  public static getInstance(): GlobalSessionStreamManager {
    if (!GlobalSessionStreamManager.instance) {
      GlobalSessionStreamManager.instance = new GlobalSessionStreamManager();
    }
    return GlobalSessionStreamManager.instance;
  }

  /**
   * Reset the singleton (for testing)
   */
  public static resetInstance(): void {
    if (GlobalSessionStreamManager.instance) {
      const sessions = Array.from(
        GlobalSessionStreamManager.instance.subscribedSessions
      );
      for (const sessionId of sessions) {
        GlobalSessionStreamManager.instance.unsubscribeFromSession(sessionId);
      }
      // GIT-029: Clear pending isolation state
      GlobalSessionStreamManager.instance.pendingIsolationState.clear();
    }
    GlobalSessionStreamManager.instance = null;
    // BRIDGE-012: Reset global callback state for testing
    globalCallbackRegistered = false;
  }

  /**
   * BRIDGE-012: Register the global chunk callback with Rust NAPI.
   * Called ONCE at app startup. All chunks from all sessions come through this callback.
   */
  public async registerGlobalCallback(): Promise<void> {
    if (globalCallbackRegistered) {
      return;
    }

    try {
      const napi = await getNapiModule();
      napi.sessionSetGlobalChunkCallback(
        (err: Error | null, args: GlobalChunkCallbackArgs) => {
          if (err || !args || !args.sessionId || !args.chunk) {
            return;
          }
          this.handleChunk(args.sessionId, args.chunk);
        }
      );
      globalCallbackRegistered = true;
    } catch (error) {
      logger.error(
        '[GlobalSessionStreamManager] Failed to register global callback:',
        error
      );
    }
  }

  /**
   * Subscribe to a session's stream.
   * Chunks are delivered via the global callback registered at initialization.
   * Called when a session is created to track it for handler routing.
   */
  public subscribeToSession(sessionId: string): void {
    if (this.subscribedSessions.has(sessionId)) {
      return;
    }

    this.subscribedSessions.add(sessionId);
    this.sessionHandlers.set(sessionId, new Set());
  }

  /**
   * Unsubscribe from a session's stream.
   * Removes tracking for the session.
   * Called when a session is destroyed.
   */
  public unsubscribeFromSession(sessionId: string): void {
    if (!this.subscribedSessions.has(sessionId)) {
      return;
    }

    this.subscribedSessions.delete(sessionId);
    this.sessionHandlers.delete(sessionId);
    // GIT-029: Clean up any pending isolation state for this session
    this.pendingIsolationState.delete(sessionId);
    // BRIDGE-012: No longer need to call detachFromSession - global callback handles cleanup
  }

  /**
   * Register a handler for a specific session.
   * UI components call this to receive events for the session they're displaying.
   * FspecCommandRequest events are not forwarded to session handlers.
   *
   * @returns Cleanup function to unregister the handler
   */
  public registerHandler(
    sessionId: string,
    handler: SessionChunkHandler
  ): () => void {
    const handlers = this.sessionHandlers.get(sessionId);
    if (handlers) {
      handlers.add(handler);
    } else {
      const newHandlers = new Set<SessionChunkHandler>();
      newHandlers.add(handler);
      this.sessionHandlers.set(sessionId, newHandlers);
    }

    return () => {
      const currentHandlers = this.sessionHandlers.get(sessionId);
      if (currentHandlers) {
        currentHandlers.delete(handler);
      }
    };
  }

  /**
   * Subscribe to a session and register a handler in one call.
   *
   * @param sessionId - Session to subscribe to
   * @param handler - Callback for UI events (not FspecCommandRequest)
   * @returns Cleanup function to unregister the handler
   */
  public attachWithHandler(
    sessionId: string,
    handler: SessionChunkHandler
  ): () => void {
    this.subscribeToSession(sessionId);
    return this.registerHandler(sessionId, handler);
  }

  /**
   * Register a global handler that receives events from all sessions.
   * @returns Cleanup function to unregister the handler
   */
  public registerGlobalHandler(handler: GlobalChunkHandler): () => void {
    this.globalHandlers.add(handler);
    return () => {
      this.globalHandlers.delete(handler);
    };
  }

  /**
   * Get list of currently subscribed sessions.
   */
  public getSubscribedSessions(): string[] {
    return Array.from(this.subscribedSessions);
  }

  /**
   * GIT-029: Get and clear pending isolation state for a session.
   * Called when a session is activated to apply any IsolationStateChange
   * that arrived before the session was activated.
   *
   * @param sessionId - Session to get pending state for
   * @returns Pending isolation state if any, or null
   */
  public getPendingIsolationState(
    sessionId: string
  ): PendingIsolationState | null {
    const state = this.pendingIsolationState.get(sessionId);
    if (state) {
      this.pendingIsolationState.delete(sessionId);
      logger.debug(
        `[GlobalSessionStreamManager] Retrieved pending isolation state for ${sessionId}: isIsolated=${state.isIsolated}`
      );
      return state;
    }
    return null;
  }

  /**
   * Simulate a chunk being received from a session (for testing).
   */
  public simulateChunk(sessionId: string, chunk: StreamChunk): void {
    this.handleChunk(sessionId, chunk);
  }

  private handleChunk(sessionId: string, chunk: StreamChunk): void {
    for (const handler of this.globalHandlers) {
      try {
        handler(sessionId, chunk);
      } catch (error) {
        logger.error(
          `[GlobalSessionStreamManager] Global handler error:`,
          error
        );
      }
    }

    // GIT-029: Handle IsolationStateChange globally to sync session store
    if (chunk.type === 'IsolationStateChange') {
      const { isIsolated, worktreePath } = chunk;
      const currentSessionId = useSessionStore.getState().currentSessionId;
      // Only update if this is the current active session
      if (currentSessionId === sessionId) {
        useSessionStore
          .getState()
          .setIsolationState(isIsolated, worktreePath ?? null);
        logger.debug(
          `[GlobalSessionStreamManager] IsolationStateChange applied immediately: isIsolated=${isIsolated}, worktreePath=${worktreePath}`
        );
      } else {
        // Session not yet active - store for later application when activated
        this.pendingIsolationState.set(sessionId, {
          isIsolated,
          worktreePath: worktreePath ?? null,
        });
        logger.debug(
          `[GlobalSessionStreamManager] IsolationStateChange stored as pending for ${sessionId}: isIsolated=${isIsolated}`
        );
      }
      // Don't return - allow the chunk to be forwarded to session handlers as well
    }

    // TUI-091: Handle FooterStateUpdate globally to sync footer store
    if (chunk.type === 'FooterStateUpdate') {
      useFooterStore
        .getState()
        .updateFooterState(
          sessionId,
          chunk.cwd,
          chunk.displayPath,
          chunk.isGitRepo,
          chunk.branch ?? null
        );
      // Don't forward to session handlers — footer state is store-only
      return;
    }

    if (chunk.type === 'FspecCommandRequest' && chunk.fspecRequest) {
      void this.handleFspecCommandRequest(sessionId, chunk);
      return;
    }

    const handlers = this.sessionHandlers.get(sessionId);
    if (handlers) {
      for (const handler of handlers) {
        try {
          handler(chunk);
        } catch (error) {
          logger.error(
            `[GlobalSessionStreamManager] Session handler error:`,
            error
          );
        }
      }
    }
  }

  /**
   * Handle FspecCommandRequest from any session.
   * FspecCommandRequest is handled globally so detached sessions don't deadlock.
   */
  private async handleFspecCommandRequest(
    sessionId: string,
    chunk: StreamChunk
  ): Promise<void> {
    const request = chunk.fspecRequest;
    if (!request) {
      return;
    }

    const { command, argsJson, projectRoot, toolCallId } = request;

    try {
      if (!this.fspecCallback) {
        const { fspecCallback } = await import('../../utils/fspec-callback');
        this.fspecCallback = fspecCallback;
      }

      const resultJson = await this.fspecCallback(
        command,
        argsJson,
        projectRoot
      );

      const parsed = JSON.parse(resultJson) as {
        success?: boolean;
        data?: string;
        error?: string;
        systemReminders?: string[];
      };

      let systemReminder: string | undefined = undefined;
      if (parsed.systemReminders && parsed.systemReminders.length > 0) {
        systemReminder = `<system-reminder>\n${parsed.systemReminders.join('\n\n')}\n</system-reminder>`;
      }

      const napi = await getNapiModule();
      napi.sessionSendFspecResult(sessionId, {
        success: parsed.success ?? true,
        data: parsed.data ?? resultJson,
        error: parsed.error ?? undefined,
        systemReminder,
        toolCallId,
      });
    } catch (error) {
      logger.error(
        `[GlobalSessionStreamManager] FspecCommandRequest failed: session=${sessionId}`,
        error
      );

      try {
        const napi = await getNapiModule();
        const errorMessage =
          error instanceof Error ? error.message : String(error);
        napi.sessionSendFspecResult(sessionId, {
          success: false,
          data: '',
          error: errorMessage,
          systemReminder: undefined,
          toolCallId,
        });
      } catch (sendError) {
        logger.error(
          `[GlobalSessionStreamManager] Failed to send error result:`,
          sendError
        );
      }
    }
  }
}

let isInitialized = false;

/**
 * Initialize the GlobalSessionStreamManager. Call once at app startup.
 * BRIDGE-012: Registers the global chunk callback with Rust NAPI.
 */
export function initGlobalSessionStreamManager(): void {
  if (isInitialized) {
    return;
  }
  const manager = GlobalSessionStreamManager.getInstance();

  // BRIDGE-012: Register global callback ONCE at startup
  void manager.registerGlobalCallback();

  isInitialized = true;
}

/**
 * Stop the GlobalSessionStreamManager. Unsubscribes from all sessions.
 * BRIDGE-012: Resets global callback state for testing.
 */
export function stopGlobalSessionStreamManager(): void {
  GlobalSessionStreamManager.resetInstance();
  isInitialized = false;
  globalCallbackRegistered = false;
}

/**
 * Clear the napiModule cache (for testing).
 * This forces a fresh import on next access.
 * BRIDGE-012: Also resets global callback state.
 */
export function clearNapiModuleCache(): void {
  napiModule = null;
  globalCallbackRegistered = false;
}

/**
 * Inject a chunk into the manager for testing purposes.
 * This bypasses the NAPI layer and directly invokes handlers.
 *
 * @param sessionId - Session to inject chunk for
 * @param chunk - The chunk to inject
 */
export function injectTestChunk(sessionId: string, chunk: StreamChunk): void {
  const manager = GlobalSessionStreamManager.getInstance();
  manager.simulateChunk(sessionId, chunk);
}

/**
 * Register a handler for a session without subscribing (for testing).
 * Use this when you want to receive chunks via injectTestChunk.
 *
 * @param sessionId - Session to register handler for
 * @param handler - Chunk handler callback
 * @returns Cleanup function
 */
export function registerTestHandler(
  sessionId: string,
  handler: SessionChunkHandler
): () => void {
  const manager = GlobalSessionStreamManager.getInstance();

  if (!manager['sessionHandlers'].has(sessionId)) {
    manager['sessionHandlers'].set(sessionId, new Set());
  }
  return manager.registerHandler(sessionId, handler);
}

/**
 * GIT-029: Apply pending isolation state for a session.
 * Called when a session is activated to apply any IsolationStateChange
 * that arrived before the session was activated.
 *
 * @param sessionId - Session to check for pending isolation state
 */
export function applyPendingIsolationState(sessionId: string): void {
  const manager = GlobalSessionStreamManager.getInstance();
  const pendingState = manager.getPendingIsolationState(sessionId);
  if (pendingState) {
    useSessionStore
      .getState()
      .setIsolationState(pendingState.isIsolated, pendingState.worktreePath);
  }
}
