// Question Item Interface
export interface QuestionItem {
  text: string;
  selected: boolean;
  answer?: string;
}

// User Story Interface
export interface UserStory {
  role: string;
  action: string;
  benefit: string;
}

// Work Item Type
export type WorkItemType = 'story' | 'task' | 'bug';

// Work Units Types
export interface WorkUnit {
  id: string;
  title: string;
  type?: WorkItemType; // Optional for backward compatibility, defaults to 'story'
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
  rules?: string[];
  examples?: string[];
  questions?: QuestionItem[];
  assumptions?: string[];
  architectureNotes?: string[];
  attachments?: string[]; // Relative paths to files in spec/attachments/<work-unit-id>/
  userStory?: UserStory;
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
