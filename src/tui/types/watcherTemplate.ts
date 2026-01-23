/**
 * Watcher Template Types
 *
 * Type definitions for watcher templates and instances.
 * Part of WATCH-023: Watcher Templates and Improved Creation UX
 *
 * @see spec/features/watcher-templates.feature
 */

/**
 * A saved watcher configuration that can spawn multiple instances.
 * Templates are stored at user level (~/.fspec/watcher-templates.json).
 */
export interface WatcherTemplate {
  /** Unique identifier (UUID) */
  id: string;
  /** Display name (e.g., "Security Reviewer") */
  name: string;
  /** URL-friendly slug derived from name (e.g., "security-reviewer") */
  slug: string;
  /** Model identifier (e.g., "anthropic/claude-sonnet-4-20250514") */
  modelId: string;
  /** Authority level: peer (suggestions) or supervisor (directives) */
  authority: 'peer' | 'supervisor';
  /** Watching brief - instructions for what to watch for */
  brief: string;
  /** Whether to auto-inject messages into parent session */
  autoInject: boolean;
  /** ISO timestamp of creation */
  createdAt: string;
  /** ISO timestamp of last update */
  updatedAt: string;
}

/**
 * A running watcher instance spawned from a template.
 * Instances are ephemeral and tied to session lifecycle.
 */
export interface WatcherInstance {
  /** Session ID of the watcher */
  sessionId: string;
  /** ID of the template this instance was spawned from */
  templateId: string;
  /** Current status of the watcher */
  status: 'running' | 'idle';
}

/**
 * Union type for flat list navigation in WatcherTemplateList.
 * Follows the same pattern as ModelListItem in AgentView.tsx.
 */
export type WatcherListItem =
  | {
      type: 'template';
      template: WatcherTemplate;
      isExpanded: boolean;
      instanceCount: number;
    }
  | {
      type: 'instance';
      template: WatcherTemplate;
      instance: WatcherInstance;
    }
  | {
      type: 'create-new';
    };
