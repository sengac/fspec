/**
 * Provider types for TUI components
 *
 * PROV-007: Shared types for provider configuration and profile management.
 * Used by ModelSelectorView and related hooks.
 */

import type { ProfileConfig } from '../../utils/provider-config';
import type { NapiModelInfo } from '@sengac/codelet-napi';

// Re-export NapiModelInfo as ProviderModel for component compatibility
export type ProviderModel = NapiModelInfo;

/**
 * Provider section for model selector
 * Represents either a cloud provider or a local profile
 */
export interface ProviderSection {
  providerId: string;
  providerName: string;
  internalName: string;
  models: ProviderModel[];
  hasCredentials: boolean;
  /** Profile name if this is a profile section */
  profileName?: string;
  /** Profile config if this is a profile section */
  profileConfig?: ProfileConfig;
  /** Whether the local server is unreachable */
  isUnreachable?: boolean;
}

// ============================================================================
// Model Selection Types (TUI-034, TUI-076)
// ============================================================================

/**
 * Selected model with full configuration
 *
 * TUI-034: Created for hierarchical model selector
 * PROV-007: Extended with profileName/profileConfig for local servers
 *
 * Used to track the currently active model in a session.
 * Persisted to session manifest for resume functionality.
 */
export interface ModelSelection {
  /** Provider ID from models.dev (e.g., "anthropic", "openai", "google") */
  providerId: string;

  /** Model ID without provider prefix (e.g., "claude-sonnet-4") */
  modelId: string;

  /** Full API model ID for API calls (e.g., "claude-sonnet-4-20250514") */
  apiModelId: string;

  /** Human-readable display name (e.g., "Claude Sonnet 4") */
  displayName: string;

  /** Whether model supports extended thinking/reasoning */
  reasoning: boolean;

  /** Whether model supports vision/image input */
  hasVision: boolean;

  /** Context window size in tokens */
  contextWindow: number;

  /** Maximum output tokens */
  maxOutput: number;

  /** Profile name if model is from a local profile (PROV-007) */
  profileName?: string;

  /** Profile config for local servers (PROV-007) */
  profileConfig?: ProfileConfig;
}

/**
 * Flattened item for VirtualList-based model selector
 *
 * TUI-034: Created for efficient scrolling through hierarchical list
 * TUI-076: Consolidated from AgentView.tsx
 *
 * The model selector shows a tree structure:
 * - Provider/Profile sections (collapsible)
 * - Models within each section
 *
 * This discriminated union allows VirtualList to render both
 * section headers and model items in a flat list.
 */
export type ModelSelectorItem =
  | {
      type: 'section';
      sectionIdx: number;
      section: ProviderSection;
      isExpanded: boolean;
    }
  | {
      type: 'model';
      sectionIdx: number;
      modelIdx: number;
      section: ProviderSection;
      model: NapiModelInfo;
    };

/**
 * Profile form field
 */
export interface ProfileFormField {
  key: keyof ProfileConfig;
  label: string;
  type: 'text' | 'number' | 'password';
  required: boolean;
  placeholder?: string;
}

/**
 * Profile form fields configuration
 */
export const PROFILE_FORM_FIELDS: ProfileFormField[] = [
  {
    key: 'baseUrl',
    label: 'Base URL',
    type: 'text',
    required: true,
    placeholder: 'http://localhost:8888',
  },
  {
    key: 'apiKey',
    label: 'API Key',
    type: 'password',
    required: true,
    placeholder: 'Enter API key',
  },
  {
    key: 'contextWindow',
    label: 'Context Window',
    type: 'number',
    required: false,
    placeholder: '128000',
  },
  {
    key: 'maxOutputTokens',
    label: 'Max Output Tokens',
    type: 'number',
    required: false,
    placeholder: '16384',
  },
];
