/**
 * fspec Browser Agent - Ref State Management
 *
 * Centralized state management for scan results, enabling the
 * scan→interact→verify workflow. Stores ref→selector mappings,
 * accessibility tree text (for diffing), and timestamps per tab.
 *
 * State is in-memory in the service worker — no chrome.storage
 * needed because refs are ephemeral (invalidated on navigation)
 * and sub-millisecond access is required for click/fill resolution.
 *
 * Implemented by: LOCATE-003
 */

/** A single interactive element's identifying information */
export interface RefEntry {
  /** CSS selector to find the element */
  selector: string;
  /** ARIA/semantic role (e.g., 'button', 'textbox', 'link') */
  role: string;
  /** Accessible name (text content, aria-label, etc.) */
  name: string;
  /** Frame ID — 0 for main frame, positive for iframe elements */
  frameId: number;
}

/** Complete scan state for a single tab */
export interface TabScanState {
  /** Map of ref keys (e.g. 'e1', 'e2') to their RefEntry */
  refs: Map<string, RefEntry>;
  /** Full accessibility tree text output (used for diff comparison) */
  treeText: string;
  /** Timestamp (Date.now()) when the scan was performed */
  timestamp: number;
}

/** In-memory state store, keyed by tabId */
const tabStates = new Map<number, TabScanState>();

/**
 * Store a new scan result for a tab.
 * Replaces any existing state for this tabId.
 */
export function setTabScanState(tabId: number, state: TabScanState): void {
  tabStates.set(tabId, state);
}

/**
 * Retrieve the current scan state for a tab.
 * Returns undefined if no scan exists or state was invalidated.
 */
export function getTabScanState(tabId: number): TabScanState | undefined {
  return tabStates.get(tabId);
}

/**
 * Clear scan state for a tab.
 * Called on navigation (changeInfo.url) and tab close (onRemoved)
 * to invalidate stale refs and free memory.
 */
export function clearTabScanState(tabId: number): void {
  tabStates.delete(tabId);
}

/**
 * Reset all tab state. For testing only — allows clean isolation
 * between tests without knowing which tab IDs were used.
 */
export function _resetForTesting(): void {
  tabStates.clear();
}

/**
 * Resolve a ref key to its RefEntry for a given tab.
 * Convenience wrapper for getTabScanState + refs.get.
 *
 * @param tabId - The tab ID to look up
 * @param ref - The ref key (e.g. 'e1', 'e5')
 * @returns The RefEntry if found, undefined otherwise
 */
export function resolveRef(tabId: number, ref: string): RefEntry | undefined {
  const state = tabStates.get(tabId);
  if (!state) {
    return undefined;
  }
  return state.refs.get(ref);
}
