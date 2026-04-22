/**
 * Model Selection Service
 *
 * PROV-008: Orchestrates model selection across session, store, and persistence layers.
 *
 * This service consolidates the duplicated model selection logic from AgentView.tsx:
 * 1. Configures environment for profile-based models
 * 2. Updates Rust session via NAPI (if session exists)
 * 3. Updates local state (if no session)
 * 4. Persists selection to user config
 *
 * Supports both:
 * - Cloud providers: Uses sessionSetModel() for registry-based validation
 * - Profile-based models: Uses sessionSetModelProfile() to bypass registry
 */

import { sessionSetModel, sessionSetModelProfile } from '@sengac/codelet-napi';
import { loadConfig, writeConfig } from '../../utils/config';
import { buildModelString } from '../utils/model-selection';
import { mapProviderIdToInternal } from '../utils/provider-mapping';
import { configureProfileEnvironment } from './profileEnvironmentService';
import { isCustomProviderSection } from './customProviderSectionBuilder';
import { logger } from '../../utils/logger';
import type { ModelSelection } from '../types/provider';

/**
 * Options for selectModel service
 */
export interface SelectModelOptions {
  /** Current session ID, or null if no session exists */
  sessionId: string | null;

  /** The model selection to apply */
  selection: ModelSelection;

  /** Callback to refresh Rust state after session update */
  onRefreshRustState?: (sessionId: string) => void;

  /** Callback to set current model in local state (when no session) */
  onSetCurrentModel?: (selection: ModelSelection) => void;

  /** Callback to set current provider in local state (when no session) */
  onSetCurrentProvider?: (provider: string) => void;
}

/**
 * Result of model selection attempt
 */
export interface SelectModelResult {
  /** Whether the model switch succeeded */
  success: boolean;

  /** Error message if failed */
  error?: string;
}

/**
 * Select a model, updating session and persisting to config.
 *
 * This is the single entry point for model selection, replacing the
 * duplicated model selection callbacks.
 *
 * Flow:
 * 1. If profile model, configure environment variables
 * 2. If session exists, update Rust session
 * 3. If no session, update local state for later sync
 * 4. Persist selection to user config (only on success)
 *
 * @param options - Selection options including session, model, and callbacks
 * @returns Result indicating success/failure
 */
export async function selectModel(
  options: SelectModelOptions
): Promise<SelectModelResult> {
  const {
    sessionId,
    selection,
    onRefreshRustState,
    onSetCurrentModel,
    onSetCurrentProvider,
  } = options;

  // 1. Configure environment for profile-based models
  if (selection.profileConfig) {
    configureProfileEnvironment(selection.profileConfig);
  }

  // Track whether session update succeeded
  let sessionUpdateSucceeded = true;
  let errorMessage: string | undefined;

  // 2. Update Rust session if exists
  if (sessionId) {
    try {
      if (selection.profileConfig) {
        // Profile-based model: use sessionSetModelProfile (bypasses registry)
        // MODEL-005: Pass context window and max output tokens
        // MODEL-004: Pass facade override for custom model dispatch
        // CTX-008: Pass compaction threshold if configured
        // BUG-137: Pass profile name so selected_model_string() can emit
        // the profile-qualified composite "provider:profile/model". Without
        // this, AgentManager.spawn re-creates subordinates as cloud models
        // and fails registry validation.
        await sessionSetModelProfile(
          sessionId,
          selection.providerId,
          selection.modelId,
          selection.contextWindow,
          selection.maxOutput,
          selection.facade ?? null,
          selection.compactionThreshold?.type ?? null,
          selection.compactionThreshold?.value ?? null,
          selection.profileName ?? null
        );
      } else if (
        selection.providerId === 'codex' ||
        isCustomProviderSection(selection.providerId)
      ) {
        // PROV-018: Codex models bypass registry (not in models.dev under 'codex')
        // PROV-067: Custom providers bypass registry (not in models.dev at all)
        // MODEL-005: Pass context window and max output tokens
        // CTX-008: Pass compaction threshold if configured
        // BUG-137: No profile_name for codex / custom providers — the
        // plain "provider/model" composite is what subordinates expect.
        await sessionSetModelProfile(
          sessionId,
          selection.providerId,
          selection.modelId,
          selection.contextWindow,
          selection.maxOutput,
          selection.facade ?? null,
          selection.compactionThreshold?.type ?? null,
          selection.compactionThreshold?.value ?? null,
          null
        );
      } else {
        // Cloud provider: use sessionSetModel (uses registry validation)
        // MODEL-005: Pass context window and max output tokens
        // CTX-008: Pass compaction threshold if configured
        await sessionSetModel(
          sessionId,
          selection.providerId,
          selection.modelId,
          selection.contextWindow,
          selection.maxOutput,
          selection.compactionThreshold?.type ?? null,
          selection.compactionThreshold?.value ?? null
        );
      }
      onRefreshRustState?.(sessionId);
    } catch (err) {
      sessionUpdateSucceeded = false;
      errorMessage = err instanceof Error ? err.message : String(err);
      logger.error('Failed to update session model', {
        error: err,
        sessionId,
        providerId: selection.providerId,
        modelId: selection.modelId,
      });
    }
  }

  // 3. Always update Zustand store on success (keeps store in sync for new sessions)
  if (sessionUpdateSucceeded) {
    onSetCurrentModel?.(selection);
    onSetCurrentProvider?.(mapProviderIdToInternal(selection.providerId));
  }

  // 4. Persist to config ONLY if session update succeeded (or no session exists)
  if (sessionUpdateSucceeded) {
    try {
      const modelString = buildModelString(
        {
          providerId: selection.providerId,
          profileName: selection.profileName,
        },
        selection.modelId
      );
      const existingConfig = await loadConfig();
      await writeConfig('user', {
        ...existingConfig,
        tui: {
          ...existingConfig?.tui,
          lastUsedModel: modelString,
        },
      });
    } catch (err) {
      logger.error('Failed to persist model selection', { error: err });
      // Don't fail the whole operation if persistence fails
    }
  }

  return {
    success: sessionUpdateSucceeded,
    error: errorMessage,
  };
}
