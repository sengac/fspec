/**
 * Supervisor Template Types
 *
 * Type definitions for supervisor templates and instances.
 * Part of WATCH-023: Supervisor Templates and Improved Creation UX
 *
 * @see spec/features/supervisor-templates.feature
 */

/**
 * A saved supervisor configuration that can spawn multiple instances.
 * Templates are stored at user level (~/.fspec/supervisor-templates.json).
 */
export interface SupervisorTemplate {
  /** Unique identifier (UUID) */
  id: string;
  /** Display name (e.g., "Security Reviewer") */
  name: string;
  /** URL-friendly slug derived from name (e.g., "security-reviewer") */
  slug: string;
  /** Model identifier (e.g., "anthropic/claude-sonnet-4-20250514") */
  modelId: string;
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
 * A running supervisor instance spawned from a template.
 * Instances are ephemeral and tied to session lifecycle.
 */
export interface SupervisorInstance {
  /** Session ID of the supervisor */
  sessionId: string;
  /** ID of the template this instance was spawned from */
  templateId: string;
  /** Current status of the supervisor */
  status: 'running' | 'idle';
}

/**
 * Union type for flat list navigation in SupervisorTemplateList.
 * Follows the same pattern as ModelListItem in AgentView.tsx.
 */
export type SupervisorListItem =
  | {
      type: 'template';
      template: SupervisorTemplate;
      isExpanded: boolean;
      instanceCount: number;
    }
  | {
      type: 'instance';
      template: SupervisorTemplate;
      instance: SupervisorInstance;
    }
  | {
      type: 'create-new';
    };
