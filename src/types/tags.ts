/**
 * TypeScript types for tags.json structure
 * These types match the tags.schema.json schema
 */

export interface Tags {
  $schema?: string;
  categories: TagCategory[];
  combinationExamples: TagExample[];
  usageGuidelines: Guidelines;
  addingNewTags: AddingProcess;
  queries: QueryExamples;
  statistics: Statistics;
  validation: ValidationRules;
  references: Reference[];
}

export interface TagCategory {
  name: string;
  description: string;
  required: boolean;
  tags: Tag[];
  rule?: string;
}

export interface Tag {
  name: string;
  description: string;
  usage?: string;
  scope?: string;
  examples?: string;
  useCases?: string;
  whenToUse?: string;
  criteria?: string;
  meaning?: string;
  testType?: string;
}

export interface TagExample {
  title: string;
  tags: string;
  interpretation: string[];
}

export interface Guidelines {
  requiredCombinations: {
    title: string;
    requirements: string[];
    minimumExample: string;
  };
  recommendedCombinations: {
    title: string;
    includes: string[];
    recommendedExample: string;
  };
  orderingConvention: {
    title: string;
    order: string[];
    example: string;
  };
}

export interface AddingProcess {
  process: ProcessStep[];
  namingConventions: string[];
  antiPatterns: {
    dont: AntiPattern[];
    do: BestPractice[];
  };
}

export interface ProcessStep {
  step: string;
  description: string;
}

export interface AntiPattern {
  description: string;
  example?: string;
}

export interface BestPractice {
  description: string;
  example?: string;
}

export interface QueryExamples {
  title: string;
  examples: QueryExample[];
}

export interface QueryExample {
  description: string;
  command: string;
}

export interface Statistics {
  lastUpdated: string;
  phaseStats: PhaseStats[];
  componentStats: ComponentStats[];
  featureGroupStats: FeatureGroupStats[];
  updateCommand: string;
}

export interface PhaseStats {
  phase: string;
  total: number;
  complete: number;
  inProgress: number;
  planned: number;
}

export interface ComponentStats {
  component: string;
  count: number;
  percentage: string;
}

export interface FeatureGroupStats {
  featureGroup: string;
  count: number;
  percentage: string;
}

export interface ValidationRules {
  rules: ValidationRule[];
  commands: ValidationCommand[];
}

export interface ValidationRule {
  rule: string;
  description: string;
}

export interface ValidationCommand {
  description: string;
  command: string;
}

export interface Reference {
  title: string;
  url: string;
}
