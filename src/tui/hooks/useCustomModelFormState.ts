/**
 * useCustomModelFormState - Hook for custom model form state in Model Selector
 *
 * MODEL-004: Manages the custom model add/edit/delete state and CRUD callbacks.
 * Extracted from useModelSelectorState to keep files under 300 lines.
 *
 * Feature: spec/features/custom-model-registration-and-facade-override-in-model-selector.feature
 */

import { useState, useCallback } from 'react';
import type { CustomModelDefinition } from '../../utils/provider-config';
import type {
  CustomModelMode,
  CustomModelFormState,
} from '../types/customModelMode';
import { useModelStore } from '../store/modelStore';
import {
  saveCustomModel,
  deleteCustomModel,
} from '../services/customModelCrudService';
import { initializeModels } from '../services/modelInitializationService';

/**
 * Return type for the custom model form state hook
 */
export interface UseCustomModelFormStateReturn {
  /** Current mode (browse / add / edit / delete-confirm) */
  customModelMode: CustomModelMode;
  /** Form field values and cursor position */
  customModelForm: CustomModelFormState;
  /** Set the mode (browse, add, edit, delete-confirm) */
  setCustomModelMode: (mode: CustomModelMode) => void;
  /** Update form field values */
  setCustomModelFormValues: (
    values:
      | Partial<CustomModelDefinition>
      | ((
          prev: Partial<CustomModelDefinition>
        ) => Partial<CustomModelDefinition>)
  ) => void;
  /** Update the focused field index */
  setCustomModelFormFieldIndex: (
    index: number | ((prev: number) => number)
  ) => void;
  /** Save the current form (add or edit) */
  saveCustomModelForm: () => Promise<void>;
  /** Confirm deletion of the custom model */
  deleteCustomModelConfirmed: () => Promise<void>;
}

/**
 * Hook for managing custom model form state and CRUD operations.
 */
export function useCustomModelFormState(): UseCustomModelFormStateReturn {
  const [customModelMode, setCustomModelMode] = useState<CustomModelMode>({
    type: 'browse',
  });
  const [customModelForm, setCustomModelForm] = useState<CustomModelFormState>({
    values: {},
    fieldIndex: 0,
  });

  const store = useModelStore.getState();

  const setCustomModelFormValues = useCallback(
    (
      values:
        | Partial<CustomModelDefinition>
        | ((
            prev: Partial<CustomModelDefinition>
          ) => Partial<CustomModelDefinition>)
    ) => {
      setCustomModelForm(prev => ({
        ...prev,
        values: typeof values === 'function' ? values(prev.values) : values,
      }));
    },
    []
  );

  const setCustomModelFormFieldIndex = useCallback(
    (index: number | ((prev: number) => number)) => {
      setCustomModelForm(prev => ({
        ...prev,
        fieldIndex:
          typeof index === 'function' ? index(prev.fieldIndex) : index,
      }));
    },
    []
  );

  /**
   * Save the current custom model form (add or edit).
   * Persists to fspec-config.json, then refreshes models.
   */
  const saveCustomModelForm = useCallback(async () => {
    const mode = customModelMode;
    const { values } = customModelForm;

    if (mode.type !== 'add-custom-model' && mode.type !== 'edit-custom-model') {
      return;
    }

    // Require at minimum a model ID
    if (!values.id?.trim()) {
      return;
    }

    const definition: CustomModelDefinition = {
      id: values.id.trim(),
      ...(values.displayName?.trim() && {
        displayName: values.displayName.trim(),
      }),
      ...(values.facade && { facade: values.facade }),
      ...(values.contextWindow && { contextWindow: values.contextWindow }),
      ...(values.maxOutputTokens && {
        maxOutputTokens: values.maxOutputTokens,
      }),
      ...(values.compactionThreshold && {
        compactionThreshold: values.compactionThreshold,
      }),
      ...(values.reasoning !== undefined && { reasoning: values.reasoning }),
      ...(values.hasVision !== undefined && { hasVision: values.hasVision }),
    };

    const originalModelId =
      mode.type === 'edit-custom-model' ? mode.originalModelId : undefined;

    await saveCustomModel(
      mode.providerId,
      mode.profileName,
      definition,
      originalModelId
    );

    // Return to browse mode and refresh
    setCustomModelMode({ type: 'browse' });
    setCustomModelForm({ values: {}, fieldIndex: 0 });

    // Re-init models to pick up the new custom model
    store.setModelsInitialized(false);
    await initializeModels();
  }, [customModelMode, customModelForm, store]);

  /**
   * Confirm deletion of a custom model.
   * Removes from fspec-config.json, then refreshes models.
   */
  const deleteCustomModelConfirmed = useCallback(async () => {
    const mode = customModelMode;
    if (mode.type !== 'delete-custom-model-confirm') {
      return;
    }

    await deleteCustomModel(mode.providerId, mode.profileName, mode.modelId);

    // Return to browse mode and refresh
    setCustomModelMode({ type: 'browse' });

    store.setModelsInitialized(false);
    await initializeModels();
  }, [customModelMode, store]);

  return {
    customModelMode,
    customModelForm,
    setCustomModelMode,
    setCustomModelFormValues,
    setCustomModelFormFieldIndex,
    saveCustomModelForm,
    deleteCustomModelConfirmed,
  };
}
