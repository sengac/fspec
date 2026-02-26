/**
 * ActionPrompt - Generic deferred user confirmation mechanism.
 *
 * Used by InputTransition to show a message and wait for Enter/Esc
 * before executing a callback. This is NOT merge-specific — any
 * feature that needs deferred-action confirmation can reuse it.
 *
 * GIT-037
 */

/**
 * Action prompt for deferred user confirmation.
 * Generic mechanism — not merge-specific.
 */
export interface ActionPrompt {
  message: string;
  onConfirm: () => void | Promise<void>;
}
