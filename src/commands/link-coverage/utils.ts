import { readFile } from 'fs/promises';
import { join } from 'path';
import { WorkUnitType } from '../../types/work-units';

export function wrapSystemReminder(content: string): string {
  return `<system-reminder>\n${content}\n</system-reminder>`;
}

export async function detectWorkUnitType(
  featureFilePath: string,
  cwd: string
): Promise<WorkUnitType> {
  try {
    // Read feature file and extract work unit ID tag
    const featureContent = await readFile(featureFilePath, 'utf-8');
    const workUnitIdMatch = featureContent.match(/@([A-Z]+-\d+)/);

    if (!workUnitIdMatch) {
      // No work unit ID tag found - assume strictest validation (story)
      return 'story';
    }

    const workUnitId = workUnitIdMatch[1];

    // Load work units file
    const workUnitsPath = join(cwd, 'spec', 'work-units.json');
    const workUnitsContent = await readFile(workUnitsPath, 'utf-8');
    const workUnitsData = JSON.parse(workUnitsContent);

    // Look up work unit type
    const workUnit = workUnitsData.workUnits?.[workUnitId];
    if (workUnit?.type) {
      return workUnit.type as WorkUnitType;
    }

    // Work unit not found - assume strictest validation (story)
    return 'story';
  } catch {
    // Feature file or work units file doesn't exist - assume strictest validation (story)
    return 'story';
  }
}

export async function getScenariosFromFeatureFile(
  featureFilePath: string
): Promise<string[]> {
  try {
    const content = await readFile(featureFilePath, 'utf-8');
    const scenarios: string[] = [];

    // Simple regex to extract scenario names
    // Matches: "Scenario: Name" or "Scenario Outline: Name"
    const scenarioRegex = /^\s*Scenario(?:\s+Outline)?:\s*(.+)$/gm;
    let match;

    while ((match = scenarioRegex.exec(content)) !== null) {
      scenarios.push(match[1].trim());
    }

    return scenarios;
  } catch {
    // Feature file doesn't exist
    return [];
  }
}
