/**
 * Generic Foundation Schema Types
 *
 * Feature: spec/features/design-generic-foundation-schema.feature
 *
 * This schema is designed to work for ANY project type (web apps, CLI tools,
 * libraries, services, mobile apps) and focuses ONLY on WHY (problem) and
 * WHAT (solution), never HOW (implementation).
 *
 * Key Principles:
 * - REQUIRED: project identity, problem statement, solution overview
 * - OPTIONAL: architecture diagrams, constraints, detailed personas
 * - Supports hierarchical foundations via subFoundations array
 * - Validates using Ajv with ajv-formats
 * - Preserves Mermaid diagram validation
 */

import type { EventStormItem } from './index';

/**
 * Shared base interface for Event Storm structures
 * Reused by both foundation-level and work unit-level Event Storms
 */
export interface EventStormBase {
  sessionDate?: string; // ISO 8601 timestamp
  facilitator?: string; // Who facilitated the session
  participants?: string[]; // Who participated
  items: EventStormItem[]; // All Event Storm artifacts
  nextItemId: number; // Auto-increment counter for stable IDs
}

/**
 * Foundation-level Event Storm (Big Picture Event Storming)
 * Used for strategic domain understanding at the foundation level
 */
export interface FoundationEventStorm extends EventStormBase {
  level: 'big_picture'; // Fixed value for foundation-level Event Storm
}

/**
 * Main Generic Foundation Document
 * Maps to generic-foundation.schema.json
 */
export interface GenericFoundation {
  $schema?: string;
  version: string; // Schema version for migration compatibility

  // REQUIRED: Project Identity
  project: ProjectIdentity;

  // REQUIRED: Problem Space (WHY)
  problemSpace: ProblemSpace;

  // REQUIRED: Solution Space (WHAT)
  solutionSpace: SolutionSpace;

  // OPTIONAL: Hierarchical foundations
  subFoundations?: string[];

  // OPTIONAL: Architecture diagrams
  architectureDiagrams?: MermaidDiagram[];

  // OPTIONAL: Constraints
  constraints?: Constraints;

  // OPTIONAL: Detailed personas
  personas?: Persona[];

  // OPTIONAL: Big Picture Event Storm
  eventStorm?: FoundationEventStorm;
}

/**
 * Project Identity
 * Basic metadata about the project
 */
export interface ProjectIdentity {
  name: string;
  vision: string; // One-sentence elevator pitch
  projectType: ProjectType;
  repository?: string;
  license?: string;
}

/**
 * Project Type
 * Supports any kind of software project
 */
export type ProjectType =
  | 'web-app'
  | 'cli-tool'
  | 'library'
  | 'sdk'
  | 'mobile-app'
  | 'desktop-app'
  | 'service'
  | 'api'
  | 'other';

/**
 * Problem Space (WHY)
 * Captures the problems this project solves
 */
export interface ProblemSpace {
  primaryProblem: Problem;
  additionalProblems?: Problem[];
  currentStatePainPoints?: string[];
}

/**
 * Problem Definition
 * Can have thousands of problems - use subFoundations for scalability
 */
export interface Problem {
  title: string;
  description: string;
  impact: ImpactRating;
  frequency?: FrequencyRating;
  cost?: CostRating;
  affectedStakeholders?: string[];
}

export type ImpactRating = 'high' | 'medium' | 'low';
export type FrequencyRating = 'constant' | 'frequent' | 'occasional' | 'rare';
export type CostRating = 'critical' | 'significant' | 'moderate' | 'minor';

/**
 * Solution Space (WHAT)
 * Broad capabilities only (3-7 high-level abilities)
 * Granular features belong in .feature files
 */
export interface SolutionSpace {
  overview: string;
  capabilities: Capability[]; // 3-7 high-level capabilities
  outOfScope?: string[]; // What this solution does NOT do
  successCriteria?: string[];
}

/**
 * Capability
 * High-level ability of the system (WHAT it does, not HOW)
 * Example: 'User Authentication' is a capability
 * Example: 'Login with OAuth' is a feature in user-authentication.feature
 */
export interface Capability {
  name: string;
  description: string;
  rationale?: string; // Why this capability is important
}

/**
 * Persona
 * Unlimited personas allowed, suggest 3-7 as best practice
 */
export interface Persona {
  name: string;
  description: string;
  goals?: string[];
  painPoints?: string[];
}

/**
 * Mermaid Diagram
 * Validated using mermaid.parse() with jsdom (existing pattern)
 */
export interface MermaidDiagram {
  title: string;
  mermaidCode: string;
  description?: string;
}

/**
 * Constraints
 * Business, technical, and timeline constraints
 */
export interface Constraints {
  business?: string[];
  technical?: string[];
  timeline?: string[];
  budget?: string[];
}
