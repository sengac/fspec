/**
 * Work Unit Context Types
 *
 * SOLID: Interface Segregation - only the data needed for context
 *
 * TUI-059: Work Unit Context in Environment Information
 */

/**
 * Work unit context information stored in session
 */
export interface WorkUnitContext {
  /** Work unit ID (e.g., "AUTH-001") */
  id: string;
  /** Work unit title (e.g., "User Authentication") */
  title: string;
  /** Current status (e.g., "specifying", "testing") */
  status: string;
  /** Optional work unit type */
  type?: 'story' | 'bug' | 'task';
}

/**
 * Represents a change in work unit context
 * Used to generate system reminders when LLM switches work units
 */
export interface WorkUnitContextChange {
  /** Previous work unit context (null if none was set) */
  previous: WorkUnitContext | null;
  /** New work unit context */
  current: WorkUnitContext;
  /** Session ID where the change occurred */
  sessionId: string;
}
