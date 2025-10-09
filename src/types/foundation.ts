/**
 * TypeScript types for foundation.json structure
 * These types match the foundation.schema.json schema
 */

export interface Foundation {
  $schema?: string;
  project: ProjectMetadata;
  whatWeAreBuilding: WhatSection;
  whyWeAreBuildingIt: WhySection;
  architectureDiagrams: DiagramSection[];
  coreCommands: CommandSection;
  featureInventory: InventorySection;
  notes: NotesSection;
}

export interface ProjectMetadata {
  name: string;
  description: string;
  repository: string;
  license: string;
  importantNote: string;
}

export interface WhatSection {
  projectOverview: string;
  technicalRequirements: {
    coreTechnologies: TechnologyItem[];
    architecture: ArchitectureDescription;
    developmentAndOperations: DevOpsDescription;
    keyLibraries: LibraryCategory[];
  };
  nonFunctionalRequirements: RequirementCategory[];
}

export interface TechnologyItem {
  category: string;
  name: string;
  description?: string;
}

export interface ArchitectureDescription {
  pattern: string;
  fileStructure: string;
  deploymentTarget: string;
  integrationModel: string[];
}

export interface DevOpsDescription {
  developmentTools: string;
  testingStrategy: string;
  logging: string;
  validation: string;
  formatting: string;
}

export interface LibraryCategory {
  category: string;
  libraries: Library[];
}

export interface Library {
  name: string;
  description: string;
}

export interface RequirementCategory {
  category: string;
  requirements: string[];
}

export interface WhySection {
  problemDefinition: {
    primary: ProblemDescription;
    secondary: string[];
  };
  painPoints: {
    currentState: string;
    specific: PainPoint[];
  };
  stakeholderImpact: StakeholderImpact[];
  theoreticalSolutions: Solution[];
  developmentMethodology: Methodology;
  successCriteria: Criterion[];
  constraintsAndAssumptions: {
    constraints: ConstraintCategory[];
    assumptions: AssumptionCategory[];
  };
}

export interface ProblemDescription {
  title: string;
  description: string;
  points: string[];
}

export interface PainPoint {
  title: string;
  impact: string;
  frequency: string;
  cost: string;
}

export interface StakeholderImpact {
  stakeholder: string;
  description: string;
}

export interface Solution {
  title: string;
  selected: boolean;
  description: string;
  pros: string[];
  cons: string[];
  feasibility: string;
}

export interface Methodology {
  name: string;
  description: string;
  steps: string[];
  ensures: string[];
}

export interface Criterion {
  title: string;
  criteria: string[];
}

export interface ConstraintCategory {
  category: string;
  items: string[];
}

export interface AssumptionCategory {
  category: string;
  items: string[];
}

export interface DiagramSection {
  title: string;
  mermaidCode: string;
  description?: string;
}

export interface CommandSection {
  categories: CommandCategory[];
}

export interface CommandCategory {
  title: string;
  commands: Command[];
}

export interface Command {
  command: string;
  description: string;
  status: '✅' | '🚧' | '📋';
}

export interface InventorySection {
  phases: PhaseInventory[];
  tagUsageSummary: {
    phaseDistribution: Distribution[];
    componentDistribution: Distribution[];
    featureGroupDistribution: Distribution[];
    priorityDistribution: Distribution[];
    testingCoverage: Distribution[];
  };
}

export interface PhaseInventory {
  phase: string;
  title: string;
  description: string;
  features: FeatureEntry[];
}

export interface FeatureEntry {
  featureFile: string;
  command: string;
  description: string;
}

export interface Distribution {
  tag: string;
  count: number;
  percentage: string;
}

export interface NotesSection {
  developmentStatus: PhaseStatus[];
  integrationNotes?: {
    title: string;
    content: string[];
  };
  projectName?: {
    name: string;
    description: string;
    repository: string;
  };
  futureEnhancements?: Enhancement[];
}

export interface PhaseStatus {
  phase: string;
  title: string;
  status: 'COMPLETE' | 'IN_PROGRESS' | 'PLANNED';
  items: string[];
  testCoverage?: string;
  progress?: string;
}

export interface Enhancement {
  title: string;
  description: string;
  details?: string[];
}
