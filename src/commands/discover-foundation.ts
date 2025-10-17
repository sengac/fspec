/**
 * discover-foundation Command
 *
 * Orchestrates code analysis + questionnaire to generate foundation.json
 */

import { discoveryGuidance } from '../guidance/automated-discovery-code-analysis';
import {
  runQuestionnaire,
  type QuestionnaireOptions,
} from './interactive-questionnaire';
import { validateGenericFoundationObject } from '../validators/generic-foundation-validator';
import type { GenericFoundation } from '../types/generic-foundation';
import { wrapInSystemReminder } from '../utils/system-reminder';

export interface DiscoverFoundationOptions {
  outputPath?: string;
}

export interface DiscoveryResult {
  personas: string[];
  capabilities: string[];
  projectType: string;
}

/**
 * Simulate code analysis (uses FOUND-002 guidance)
 * In real implementation, this would analyze actual codebase
 */
export function analyzeCodebase(): DiscoveryResult {
  // This is a simplified version that demonstrates the concept
  // Real implementation would use the guidance from FOUND-002
  return {
    personas: ['End User', 'Admin', 'API Consumer'],
    capabilities: ['User Authentication', 'Data Management', 'API Access'],
    projectType: 'web-app',
  };
}

/**
 * Emit system-reminder after code analysis
 */
export function emitDiscoveryReminder(result: DiscoveryResult): string {
  const message = `Detected ${result.personas.length} user personas from routes: ${result.personas.join(', ')}.

Review in questionnaire. Focus on WHY/WHAT, not HOW.
See CLAUDE.md for boundary guidance.

Code analysis also detected:
- Project Type: ${result.projectType}
- Key Capabilities: ${result.capabilities.join(', ')}`;

  return wrapInSystemReminder(message);
}

/**
 * Run questionnaire with prefilled data from discovery
 */
export function runQuestionnaireWithDiscovery(
  discoveryResult: DiscoveryResult
): Record<string, string> {
  const options: QuestionnaireOptions = {
    mode: 'from-discovery',
    discoveryData: {
      projectType: discoveryResult.projectType,
      personas: discoveryResult.personas,
      capabilities: discoveryResult.capabilities,
    },
  };

  const questionnaire = runQuestionnaire(options);

  // Simulate questionnaire completion
  // In real implementation, this would be interactive UI
  const answers: Record<string, string> = {
    'core-purpose': 'Example project purpose',
    'primary-users': discoveryResult.personas.join(', '),
    'problem-solved': 'Example problem statement',
  };

  return answers;
}

/**
 * Generate foundation.json from questionnaire answers
 */
export function generateFoundationJson(
  discoveryResult: DiscoveryResult,
  questionnaireAnswers: Record<string, string>
): GenericFoundation {
  const foundation: GenericFoundation = {
    version: '2.0.0',
    project: {
      name: 'Discovered Project',
      vision: questionnaireAnswers['core-purpose'] || 'Project vision',
      projectType: discoveryResult.projectType as any,
    },
    problemSpace: {
      primaryProblem: {
        title: 'Primary Problem',
        description: questionnaireAnswers['problem-solved'] || 'Problem description',
        impact: 'high',
      },
    },
    solutionSpace: {
      overview: 'Solution overview',
      capabilities: discoveryResult.capabilities.map((cap) => ({
        name: cap,
        description: `${cap} capability`,
      })),
    },
    personas: discoveryResult.personas.map((persona) => ({
      name: persona,
      description: `${persona} description`,
      goals: ['Example goal'],
    })),
  };

  return foundation;
}

/**
 * Main discover-foundation command
 */
export async function discoverFoundation(
  options: DiscoverFoundationOptions = {}
): Promise<{
  systemReminder: string;
  foundation: GenericFoundation;
  valid: boolean;
}> {
  // Step 1: Analyze codebase
  const discoveryResult = analyzeCodebase();

  // Step 2: Emit system-reminder with detected personas/capabilities
  const systemReminder = emitDiscoveryReminder(discoveryResult);

  // Step 3: Run questionnaire with prefilled data
  const questionnaireAnswers = runQuestionnaireWithDiscovery(discoveryResult);

  // Step 4: Generate foundation.json
  const foundation = generateFoundationJson(
    discoveryResult,
    questionnaireAnswers
  );

  // Step 5: Validate foundation.json
  const validation = validateGenericFoundationObject(foundation);

  return {
    systemReminder,
    foundation,
    valid: validation.valid,
  };
}
