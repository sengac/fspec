/**
 * ModelSelectorScreen - Orchestrator for model selection
 *
 * TUI-073: Extracts model selector from AgentView.tsx.
 * Composes useModelSelectorState (state) + useInput (keyboard) + ModelSelectorView (UI).
 * MODEL-004: Adds 'a', 'e', 'd' keybinds for custom model CRUD.
 *
 * Feature: spec/features/model-selector-screen.feature
 */

import React, { useEffect, useRef, useMemo } from 'react';
import { useInput } from 'ink';
import { ModelSelectorView } from './ModelSelectorView';
import { CustomModelFormView } from './CustomModelFormView';
import { DeleteCustomModelConfirmView } from './DeleteCustomModelConfirmView';
import { useModelSelectorState } from '../hooks/useModelSelectorState';
import { prefillCustomModelValues } from '../constants/customModelForm';
import {
  handleDeleteConfirmInput,
  handleCustomModelFormInput,
} from '../inputHandlers/customModelFormHandler';
import type { ModelSelection } from '../types/provider';

export interface ModelSelectorScreenProps {
  /** Terminal width for layout */
  width: number;
  /** Terminal height for layout */
  height: number;
  /** Currently selected model ID (for highlighting and auto-expand) */
  currentModelId?: string;
  /** Called when a model is selected */
  onSelectModel: (model: ModelSelection) => void;
  /** Called when screen should close */
  onClose: () => void;
  /** Called to switch to provider settings */
  onSwitchToSettings: () => void;
}

export function ModelSelectorScreen({
  width,
  height,
  currentModelId,
  onSelectModel,
  onClose,
  onSwitchToSettings,
}: ModelSelectorScreenProps): React.ReactElement {
  const state = useModelSelectorState();

  const {
    providerSections,
    filteredFlatItems,
    isRefreshing,
    modelsInitialized,
    selectedSectionIdx,
    selectedModelIdx,
    expandedProviders,
    scrollOffset,
    visibleHeight,
    filter,
    isFilterMode,
    customModelMode,
    customModelForm,
    setSelectedSectionIdx,
    setSelectedModelIdx,
    setVisibleHeight,
    setFilter,
    setIsFilterMode,
    setIsVisible,
    toggleSectionExpansion,
    refreshModels,
    selectModel,
    navigateUp,
    navigateDown,
    getCurrentFlatIndex,
    setCustomModelMode,
    setCustomModelFormValues,
    setCustomModelFormFieldIndex,
  } = state;

  const hasAutoExpanded = useRef(false);

  useEffect(() => {
    setVisibleHeight(height - 6);
  }, [height, setVisibleHeight]);

  useEffect(() => {
    setIsVisible(true);
    return () => {
      setIsVisible(false);
    };
  }, [setIsVisible]);

  // Auto-expand section containing currentModelId when screen opens
  useEffect(() => {
    if (hasAutoExpanded.current || !modelsInitialized || !currentModelId) {
      return;
    }
    for (let si = 0; si < providerSections.length; si++) {
      const section = providerSections[si];
      const mi = section.models.findIndex(m => m.id === currentModelId);
      if (mi !== -1) {
        if (!expandedProviders.has(section.providerId)) {
          toggleSectionExpansion(section.providerId);
        }
        setSelectedSectionIdx(si);
        setSelectedModelIdx(mi);
        hasAutoExpanded.current = true;
        break;
      }
    }
  }, [
    currentModelId,
    modelsInitialized,
    providerSections,
    expandedProviders,
    toggleSectionExpansion,
    setSelectedSectionIdx,
    setSelectedModelIdx,
  ]);

  // Keyboard handling
  useInput(
    (input, key) => {
      // Custom model delete confirmation
      if (handleDeleteConfirmInput(input, key, state)) {
        return;
      }
      // Custom model add/edit form
      if (handleCustomModelFormInput(input, key, state)) {
        return;
      }

      // Filter mode
      if (isFilterMode) {
        if (key.escape) { setIsFilterMode(false); setFilter(''); return; }
        if (key.return) { setIsFilterMode(false); return; }
        if (key.backspace || key.delete) { setFilter(filter.slice(0, -1)); return; }
        const clean = input.split('').filter(ch => { const c = ch.charCodeAt(0); return c >= 32 && c <= 126; }).join('');
        if (clean) { setFilter(filter + clean); }
        return;
      }

      // Normal mode
      if (key.escape) { if (filter) { setFilter(''); return; } onClose(); return; }
      if (key.tab) { onSwitchToSettings(); return; }
      if (input === '/') { setIsFilterMode(true); return; }
      if (input === 'r' || input === 'R') { void refreshModels(); return; }

      // MODEL-004: 'a' — add custom model (only on profile section headers)
      if (input === 'a') {
        const sec = providerSections[selectedSectionIdx];
        if (sec?.profileName) {
          setCustomModelMode({ type: 'add-custom-model', providerId: sec.providerId, profileName: sec.profileName });
          setCustomModelFormValues({});
          setCustomModelFormFieldIndex(0);
        }
        return;
      }

      // MODEL-004: 'e' — edit custom model
      if (input === 'e') {
        const sec = providerSections[selectedSectionIdx];
        if (sec?.profileName && selectedModelIdx >= 0 && sec.customModelIds) {
          const model = sec.models[selectedModelIdx];
          if (model && sec.customModelIds.has(model.id)) {
            const customDef = sec.profileConfig?.customModels?.find(c => c.id === model.id);
            if (customDef) {
              setCustomModelMode({ type: 'edit-custom-model', providerId: sec.providerId, profileName: sec.profileName, originalModelId: customDef.id });
              setCustomModelFormValues(prefillCustomModelValues(customDef));
              setCustomModelFormFieldIndex(0);
            }
          }
        }
        return;
      }

      // MODEL-004: 'd' — delete custom model
      if (input === 'd') {
        const sec = providerSections[selectedSectionIdx];
        if (sec?.profileName && selectedModelIdx >= 0 && sec.customModelIds) {
          const model = sec.models[selectedModelIdx];
          if (model && sec.customModelIds.has(model.id)) {
            setCustomModelMode({ type: 'delete-custom-model-confirm', providerId: sec.providerId, profileName: sec.profileName, modelId: model.id, displayName: model.name });
          }
        }
        return;
      }

      if (key.upArrow) { navigateUp(); return; }
      if (key.downArrow) { navigateDown(); return; }

      if (key.leftArrow) {
        const sec = providerSections[selectedSectionIdx];
        if (sec && expandedProviders.has(sec.providerId)) { toggleSectionExpansion(sec.providerId); setSelectedModelIdx(-1); }
        return;
      }
      if (key.rightArrow) {
        const sec = providerSections[selectedSectionIdx];
        if (sec && !expandedProviders.has(sec.providerId)) { toggleSectionExpansion(sec.providerId); }
        return;
      }

      if (key.return) {
        const flatIdx = getCurrentFlatIndex();
        const item = filteredFlatItems[flatIdx];
        if (!item) { return; }
        if (item.type === 'section') { toggleSectionExpansion(item.section.providerId); }
        else if (item.type === 'model') { const sel = selectModel(item.section, item.model); onSelectModel(sel); onClose(); }
        return;
      }
    },
    { isActive: true }
  );

  // Build custom model IDs map for [C] badge
  const customModelIdsBySection = useMemo(() => {
    const map = new Map<number, Set<string>>();
    for (let i = 0; i < providerSections.length; i++) {
      const section = providerSections[i];
      if (section.customModelIds && section.customModelIds.size > 0) {
        map.set(i, section.customModelIds);
      }
    }
    return map.size > 0 ? map : undefined;
  }, [providerSections]);

  // Render custom model form overlay
  if (customModelMode.type === 'add-custom-model' || customModelMode.type === 'edit-custom-model') {
    return (
      <CustomModelFormView
        title={customModelMode.type === 'add-custom-model' ? 'Add Custom Model' : 'Edit Custom Model'}
        profileName={customModelMode.profileName}
        values={customModelForm.values}
        fieldIndex={customModelForm.fieldIndex}
        width={width}
      />
    );
  }

  // Render delete confirmation overlay
  if (customModelMode.type === 'delete-custom-model-confirm') {
    return (
      <DeleteCustomModelConfirmView
        modelId={customModelMode.modelId}
        displayName={customModelMode.displayName}
        profileName={customModelMode.profileName}
      />
    );
  }

  return (
    <ModelSelectorView
      width={width}
      height={height}
      flatItems={filteredFlatItems}
      selectedSectionIdx={selectedSectionIdx}
      selectedModelIdx={selectedModelIdx}
      expandedProviders={expandedProviders}
      scrollOffset={scrollOffset}
      visibleHeight={visibleHeight}
      filter={filter}
      isFilterMode={isFilterMode}
      currentModelId={currentModelId}
      isRefreshing={isRefreshing}
      customModelIdsBySection={customModelIdsBySection}
    />
  );
}
