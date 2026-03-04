/**
 * ModelSelectorScreen - Orchestrator for model selection
 *
 * TUI-073: Extracts model selector from AgentView.tsx.
 * Composes useModelSelectorState (state) + useInput (keyboard) + ModelSelectorView (UI).
 *
 * Feature: spec/features/model-selector-screen.feature
 */

import React, { useEffect, useRef } from 'react';
import { useInput } from 'ink';
import { ModelSelectorView } from './ModelSelectorView';
import { useModelSelectorState } from '../hooks/useModelSelectorState';
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
  // Destructure hook return for stable references
  // React useState setters are stable (same reference across renders)
  // useCallback functions are stable when their deps don't change
  const {
    // Data
    providerSections,
    filteredFlatItems,
    isRefreshing,
    modelsInitialized,
    // Selection state
    selectedSectionIdx,
    selectedModelIdx,
    expandedProviders,
    // Scroll/filter state
    scrollOffset,
    visibleHeight,
    filter,
    isFilterMode,
    // Actions (stable setters from useState)
    setSelectedSectionIdx,
    setSelectedModelIdx,
    setVisibleHeight,
    setFilter,
    setIsFilterMode,
    setIsVisible,
    // Operations (stable from useCallback)
    toggleSectionExpansion,
    refreshModels,
    selectModel,
    navigateUp,
    navigateDown,
    getCurrentFlatIndex,
  } = useModelSelectorState();

  const hasAutoExpanded = useRef(false);

  // Set visible height based on terminal height (account for header/footer)
  useEffect(() => {
    setVisibleHeight(height - 6);
  }, [height, setVisibleHeight]);

  // Mark as visible when mounted, reset on unmount
  useEffect(() => {
    setIsVisible(true);
    return () => {
      setIsVisible(false);
    };
  }, [setIsVisible]);

  // Auto-expand section containing currentModelId when screen opens
  useEffect(() => {
    // Only auto-expand once per mount, and only after models are loaded
    if (hasAutoExpanded.current || !modelsInitialized || !currentModelId) {
      return;
    }

    // Find the section containing the current model
    for (
      let sectionIdx = 0;
      sectionIdx < providerSections.length;
      sectionIdx++
    ) {
      const section = providerSections[sectionIdx];
      const modelIdx = section.models.findIndex(m => m.id === currentModelId);

      if (modelIdx !== -1) {
        // Found the model - expand its section and navigate to it
        if (!expandedProviders.has(section.providerId)) {
          toggleSectionExpansion(section.providerId);
        }
        setSelectedSectionIdx(sectionIdx);
        setSelectedModelIdx(modelIdx);
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
      // ===========================================
      // FILTER MODE
      // ===========================================
      if (isFilterMode) {
        // Escape in filter mode: clear filter and exit mode
        if (key.escape) {
          setIsFilterMode(false);
          setFilter('');
          return;
        }
        // Enter in filter mode: exit mode, keep filter
        if (key.return) {
          setIsFilterMode(false);
          return;
        }
        // Backspace: remove last character
        if (key.backspace || key.delete) {
          setFilter(filter.slice(0, -1));
          return;
        }
        // Accept printable characters (ASCII 32-126)
        const clean = input
          .split('')
          .filter(ch => {
            const code = ch.charCodeAt(0);
            return code >= 32 && code <= 126;
          })
          .join('');
        if (clean) {
          setFilter(filter + clean);
        }
        return;
      }

      // ===========================================
      // NORMAL MODE
      // ===========================================

      // Escape: clear filter if active, otherwise close
      if (key.escape) {
        if (filter) {
          setFilter('');
          return;
        }
        onClose();
        return;
      }

      // Tab: switch to provider settings
      if (key.tab) {
        onSwitchToSettings();
        return;
      }

      // Slash: enter filter mode
      if (input === '/') {
        setIsFilterMode(true);
        return;
      }

      // Refresh models with 'r' or 'R'
      if (input === 'r' || input === 'R') {
        void refreshModels();
        return;
      }

      // Navigation: Up arrow
      if (key.upArrow) {
        navigateUp();
        return;
      }

      // Navigation: Down arrow
      if (key.downArrow) {
        navigateDown();
        return;
      }

      // Left arrow: collapse section and move to section header
      if (key.leftArrow) {
        const currentSection = providerSections[selectedSectionIdx];
        if (
          currentSection &&
          expandedProviders.has(currentSection.providerId)
        ) {
          toggleSectionExpansion(currentSection.providerId);
          setSelectedModelIdx(-1); // Move to section header
        }
        return;
      }

      // Right arrow: expand section
      if (key.rightArrow) {
        const currentSection = providerSections[selectedSectionIdx];
        if (
          currentSection &&
          !expandedProviders.has(currentSection.providerId)
        ) {
          toggleSectionExpansion(currentSection.providerId);
        }
        return;
      }

      // Enter: select model or toggle section expansion
      if (key.return) {
        const flatIdx = getCurrentFlatIndex();
        const item = filteredFlatItems[flatIdx];

        if (!item) {
          return;
        }

        if (item.type === 'section') {
          // Toggle section expansion
          toggleSectionExpansion(item.section.providerId);
        } else if (item.type === 'model') {
          // Select model and close
          const selection = selectModel(item.section, item.model);
          onSelectModel(selection);
          onClose();
        }
        return;
      }
    },
    { isActive: true }
  );

  // Render the presentation component
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
    />
  );
}
