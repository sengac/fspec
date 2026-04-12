/**
 * Flat Model List Utilities
 *
 * Pure helper functions for building and navigating flattened model lists
 * in the Model Selector. Extracted from useModelSelectorState for
 * separation of concerns and file size compliance (< 300 lines).
 */

import type { NapiModelInfo } from '@sengac/codelet-napi';
import type { ModelSelectorItem } from '../types/provider';
import type { ProviderSection } from '../store/modelStore';

/**
 * Build flattened list from sections and expanded state.
 * Sections always appear; models appear only if their section is expanded.
 */
export const buildFlatModelList = (
  sections: ProviderSection[],
  expandedProviders: Set<string>
): ModelSelectorItem[] => {
  const items: ModelSelectorItem[] = [];
  sections.forEach((section, sectionIdx) => {
    const isExpanded = expandedProviders.has(section.providerId);
    items.push({ type: 'section', sectionIdx, section, isExpanded });
    if (isExpanded) {
      section.models.forEach((model, modelIdx) => {
        items.push({ type: 'model', sectionIdx, modelIdx, section, model });
      });
    }
  });
  return items;
};

/**
 * Convert flat index to (sectionIdx, modelIdx).
 * modelIdx is -1 for section headers.
 */
export const flatIndexToSectionModel = (
  flatIndex: number,
  items: ModelSelectorItem[]
): { sectionIdx: number; modelIdx: number } => {
  const item = items[flatIndex];
  if (!item) {
    return { sectionIdx: 0, modelIdx: -1 };
  }
  if (item.type === 'section') {
    return { sectionIdx: item.sectionIdx, modelIdx: -1 };
  }
  return { sectionIdx: item.sectionIdx, modelIdx: item.modelIdx };
};

/**
 * Convert (sectionIdx, modelIdx) to flat index.
 */
export const sectionModelToFlatIndex = (
  sectionIdx: number,
  modelIdx: number,
  items: ModelSelectorItem[]
): number => {
  for (let i = 0; i < items.length; i++) {
    const item = items[i];
    if (
      item.type === 'section' &&
      item.sectionIdx === sectionIdx &&
      modelIdx === -1
    ) {
      return i;
    }
    if (
      item.type === 'model' &&
      item.sectionIdx === sectionIdx &&
      item.modelIdx === modelIdx
    ) {
      return i;
    }
  }
  return 0;
};

/**
 * Filter flat items by a search string (case-insensitive).
 * Matches provider name, provider ID, model ID, or model name.
 * If a section header matches, all its expanded models are included.
 */
export const filterFlatItems = (
  flatItems: ModelSelectorItem[],
  filter: string
): ModelSelectorItem[] => {
  if (!filter) {
    return flatItems;
  }

  const filterLower = filter.toLowerCase();
  const matchingSectionIdxs = new Set<number>();

  // First pass: find matching sections and models
  flatItems.forEach(item => {
    if (item.type === 'section') {
      if (
        item.section.providerName.toLowerCase().includes(filterLower) ||
        item.section.providerId.toLowerCase().includes(filterLower)
      ) {
        matchingSectionIdxs.add(item.sectionIdx);
      }
    } else if (item.type === 'model') {
      if (
        item.model.id.toLowerCase().includes(filterLower) ||
        item.model.name.toLowerCase().includes(filterLower)
      ) {
        matchingSectionIdxs.add(item.sectionIdx);
      }
    }
  });

  // Second pass: build filtered list
  return flatItems.filter(item => {
    if (item.type === 'section') {
      return matchingSectionIdxs.has(item.sectionIdx);
    }
    if (item.type === 'model') {
      const modelMatches =
        item.model.id.toLowerCase().includes(filterLower) ||
        item.model.name.toLowerCase().includes(filterLower);
      const sectionMatches =
        item.section.providerName.toLowerCase().includes(filterLower) ||
        item.section.providerId.toLowerCase().includes(filterLower);
      return modelMatches || sectionMatches;
    }
    return false;
  });
};

/**
 * Check whether a given item in the flat list is a profile section header.
 * Profile sections have a `profileName` set, distinguishing them from cloud providers.
 */
export function isProfileSectionItem(item: ModelSelectorItem): boolean {
  return item.type === 'section' && !!item.section.profileName;
}

/**
 * Check whether a given model item belongs to a custom model.
 * Returns true if the model's ID is in the section's customModelIds set.
 */
export function isCustomModelItem(item: ModelSelectorItem): boolean {
  if (item.type !== 'model') {
    return false;
  }
  return !!item.section.customModelIds?.has(item.model.id);
}
