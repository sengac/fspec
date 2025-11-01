// Base interface for items with stable IDs and soft-delete support
export interface ItemWithId {
  id: number; // Auto-incrementing, never reused
  text: string; // The actual content
  deleted: boolean; // Soft-delete flag
  createdAt: string; // ISO 8601 timestamp
  deletedAt?: string; // ISO 8601 timestamp (only when deleted=true)
}

// Rule Item Interface
export type RuleItem = ItemWithId;

// Example Item Interface
export type ExampleItem = ItemWithId;

// Architecture Note Item Interface
export type ArchitectureNoteItem = ItemWithId;

// Question Item Interface
export interface QuestionItem extends ItemWithId {
  selected: boolean;
  answer?: string;
}

// User Story Interface
export interface UserStory {
  role: string;
  action: string;
  benefit: string;
}

// Virtual Hook Interface
export interface VirtualHook {
  name: string;
  event: string;
  command: string;
  blocking: boolean;
  gitContext?: boolean;
}

// Work Unit Type
export type WorkUnitType = 'story' | 'task' | 'bug';

// Work Units Types
export interface WorkUnit {
  id: string;
  title: string;
  type?: WorkUnitType; // Optional for backward compatibility, defaults to 'story'
  status:
    | 'backlog'
    | 'specifying'
    | 'testing'
    | 'implementing'
    | 'validating'
    | 'done'
    | 'blocked';
  description?: string;
  estimate?: number;
  epic?: string;
  parent?: string;
  children?: string[];
  blocks?: string[];
  blockedBy?: string[];
  dependsOn?: string[];
  relatesTo?: string[];
  blockedReason?: string;
  rules?: RuleItem[];
  examples?: ExampleItem[];
  questions?: QuestionItem[];
  assumptions?: string[];
  architectureNotes?: ArchitectureNoteItem[];
  attachments?: string[]; // Relative paths to files in spec/attachments/<work-unit-id>/
  // ID counters for auto-increment (backward compatible with v0.6.0)
  nextRuleId?: number;
  nextExampleId?: number;
  nextQuestionId?: number;
  nextNoteId?: number;
  userStory?: UserStory;
  virtualHooks?: VirtualHook[]; // Work unit-scoped hooks for dynamic validation
  stateHistory?: Array<{
    state: string;
    timestamp: string;
    reason?: string;
  }>;
  createdAt: string;
  updatedAt: string;
  [key: string]: unknown;
}

export interface WorkUnitsData {
  meta?: {
    version: string;
    lastUpdated: string;
  };
  workUnits: Record<string, WorkUnit>;
  states: Record<string, string[]>;
}

// Epics Types
export interface Epic {
  id: string;
  title: string;
  description?: string;
  workUnits?: string[];
  createdAt: string;
  updatedAt: string;
}

export interface EpicsData {
  epics: Record<string, Epic>;
}

// Prefixes Types
export interface Prefix {
  id: string;
  description: string;
  createdAt: string;
  updatedAt: string;
}

export interface PrefixesData {
  prefixes: Record<string, Prefix>;
}
