/**
 * Types for Reverse ACDD strategy planning session
 */

export type SessionPhase =
  | 'analyzing'
  | 'gap-detection'
  | 'strategy-planning'
  | 'executing'
  | 'complete';

export type StrategyType = 'A' | 'B' | 'C' | 'D';

export interface GapAnalysis {
  testsWithoutFeatures: number;
  featuresWithoutTests: number;
  unmappedScenarios: number;
  unmappedImplementation: number;
  files: string[];
}

export interface ReverseSession {
  phase: SessionPhase;
  strategy?: StrategyType;
  strategyName?: string;
  currentStep?: number;
  totalSteps?: number;
  gaps: GapAnalysis;
  completed?: string[];
  timestamp: string;
}

export interface AnalysisResult {
  testFiles: string[];
  featureFiles: string[];
  implementationFiles: string[];
  coverageAnalysis?: {
    unmappedCount: number;
    scenarios: string[];
  };
  summary: string;
}

export interface ReverseCommandResult {
  analysis?: AnalysisResult;
  gaps?: GapAnalysis;
  suggestedStrategy?: StrategyType;
  strategyName?: string;
  systemReminder?: string;
  guidance?: string;
  phase?: SessionPhase;
  strategy?: StrategyType;
  gapsDetected?: string;
  progress?: string;
  gapList?: Array<{ file: string; completed: boolean }>;
  message?: string;
  validationComplete?: boolean;
  gapsFilled?: boolean;
  effortEstimate?: string;
  pagination?: {
    total: number;
    perPage: number;
    page: number;
  };
  summary?: string;
  existingSessionDetected?: boolean;
  exitCode?: number;
  currentPhase?: SessionPhase;
  currentStrategy?: string;
  currentProgress?: string;
  suggestions?: string[];
}

export interface ReverseCommandOptions {
  cwd?: string;
  strategy?: string;
  continue?: boolean;
  status?: boolean;
  reset?: boolean;
  complete?: boolean;
  dryRun?: boolean;
  implementationContext?: string;
}

export interface StrategyTemplate {
  name: string;
  description: string;
  effortEstimate: string;
  steps: string[];
}
