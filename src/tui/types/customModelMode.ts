/**
 * Custom model form mode types for the Model Selector
 *
 * MODEL-004: Defines mode state for add/edit/delete custom model operations.
 * Follows the same HookMode pattern used by ProviderSettings (settingsMode.ts).
 */

import type { CustomModelDefinition } from '../../utils/provider-config';

/**
 * Custom model form mode — the internal state for custom model CRUD in the Model Selector.
 */
export type CustomModelMode =
  | { type: 'browse' }
  | {
      type: 'add-custom-model';
      /** Provider ID (always 'openai' for profiles) */
      providerId: string;
      /** Profile name for this custom model */
      profileName: string;
    }
  | {
      type: 'edit-custom-model';
      /** Provider ID (always 'openai' for profiles) */
      providerId: string;
      /** Profile name for this custom model */
      profileName: string;
      /** Original model ID being edited */
      originalModelId: string;
    }
  | {
      type: 'delete-custom-model-confirm';
      /** Provider ID (always 'openai' for profiles) */
      providerId: string;
      /** Profile name for this custom model */
      profileName: string;
      /** Model ID to delete */
      modelId: string;
      /** Display name of the model (for confirmation prompt) */
      displayName: string;
    };

/**
 * Custom model form state — field values and cursor position.
 */
export interface CustomModelFormState {
  /** Current field values (partial — empty fields are omitted) */
  values: Partial<CustomModelDefinition>;
  /** Currently focused field index (0-based, indexes CUSTOM_MODEL_FORM_FIELDS) */
  fieldIndex: number;
}
