import type { EventStormBase } from './generic-foundation';

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
  answered?: boolean;
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

// Event Storm Types

// Base Event Storm item interface (extends ItemWithId pattern)
export interface EventStormItemBase extends ItemWithId {
  color: string; // Event Storming color convention
  timestamp?: number; // For timeline visualization
  boundedContext?: string; // Optional bounded context association
  relatedTo?: number[]; // IDs of related items for traceability
}

// Domain Event (orange sticky note)
export interface EventStormEvent extends EventStormItemBase {
  type: 'event';
  color: 'orange';
}

// Command (blue sticky note)
export interface EventStormCommand extends EventStormItemBase {
  type: 'command';
  color: 'blue';
  actor?: string; // Who executes the command
  triggersEvent?: number; // ID of event this command triggers
}

// Aggregate (yellow large sticky note)
export interface EventStormAggregate extends EventStormItemBase {
  type: 'aggregate';
  color: 'yellow';
  responsibilities?: string[]; // What this aggregate is responsible for
  emits?: string[]; // Domain event names this aggregate emits
}

// Policy (purple sticky note)
export interface EventStormPolicy extends EventStormItemBase {
  type: 'policy';
  color: 'purple';
  when?: string; // Trigger condition
  then?: string; // Resulting action
}

// Hotspot/Question (red sticky note)
export interface EventStormHotspot extends EventStormItemBase {
  type: 'hotspot';
  color: 'red';
  concern?: string; // Description of risk, uncertainty, or problem
}

// External System (pink sticky note)
export interface EventStormExternalSystem extends EventStormItemBase {
  type: 'external_system';
  color: 'pink';
  integrationType?: string; // API, library, service, etc.
}

// Bounded Context (blue tape / pivotal event marker)
export interface EventStormBoundedContext
  extends Omit<EventStormItemBase, 'color'> {
  type: 'bounded_context';
  color: null; // Conceptual boundary, not a sticky note
  description?: string; // Scope and responsibilities
  itemIds?: number[]; // IDs of items within this bounded context
}

// Discriminated union of all Event Storm item types
export type EventStormItem =
  | EventStormEvent
  | EventStormCommand
  | EventStormAggregate
  | EventStormPolicy
  | EventStormHotspot
  | EventStormExternalSystem
  | EventStormBoundedContext;

// Suggested tags generated from Event Storm analysis
export interface SuggestedTags {
  componentTags: string[]; // Derived from bounded contexts and aggregates
  featureGroupTags: string[]; // Derived from domain events and processes
  technicalTags: string[]; // Derived from external systems and integrations
  reasoning: string; // Explanation of how tags were derived
}

// Event Storm section for work units
export interface EventStorm extends EventStormBase {
  level: 'process_modeling' | 'software_design'; // Type of Event Storming
  suggestedTags?: SuggestedTags; // Tags suggested from Event Storm analysis
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
  eventStorm?: EventStorm; // Event Storming discovery artifacts
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
  version?: string;
  migrationHistory?: Array<{
    version: string;
    applied: string;
    backupPath: string;
  }>;
  meta?: {
    version: string;
    lastUpdated: string;
  };
  prefixCounters?: Record<string, number>;
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
