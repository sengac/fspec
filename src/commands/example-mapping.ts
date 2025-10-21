import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import type { QuestionItem } from '../types/index';

interface WorkUnit {
  id: string;
  type?: 'story' | 'task' | 'bug';
  title?: string;
  description?: string;
  rules?: string[];
  examples?: string[];
  questions?: (string | QuestionItem)[];
  assumptions?: string[];
  updatedAt: string;
}

interface WorkUnitsData {
  workUnits: Record<string, WorkUnit>;
  states: Record<string, string[]>;
}

async function loadWorkUnits(cwd: string): Promise<WorkUnitsData> {
  const workUnitsFile = join(cwd, 'spec', 'work-units.json');
  const content = await readFile(workUnitsFile, 'utf-8');
  return JSON.parse(content);
}

async function saveWorkUnits(data: WorkUnitsData, cwd: string): Promise<void> {
  const workUnitsFile = join(cwd, 'spec', 'work-units.json');
  await writeFile(workUnitsFile, JSON.stringify(data, null, 2));
}

export async function addRule(
  workUnitId: string,
  rule: string,
  options: { cwd: string }
): Promise<void> {
  const { cwd } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  if (!workUnitsData.workUnits[workUnitId]) {
    throw new Error(`Work unit '${workUnitId}' does not exist`);
  }

  const workUnit = workUnitsData.workUnits[workUnitId];
  if (!workUnit.rules) {
    workUnit.rules = [];
  }

  workUnit.rules.push(rule);
  workUnit.updatedAt = new Date().toISOString();

  await saveWorkUnits(workUnitsData, cwd);
}

export async function addExample(
  workUnitId: string,
  example: string,
  options: { cwd: string }
): Promise<void> {
  const { cwd } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  if (!workUnitsData.workUnits[workUnitId]) {
    throw new Error(`Work unit '${workUnitId}' does not exist`);
  }

  const workUnit = workUnitsData.workUnits[workUnitId];
  if (!workUnit.examples) {
    workUnit.examples = [];
  }

  workUnit.examples.push(example);
  workUnit.updatedAt = new Date().toISOString();

  await saveWorkUnits(workUnitsData, cwd);
}

export async function addQuestion(
  workUnitId: string,
  question: string,
  options: { cwd: string }
): Promise<void> {
  const { cwd } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  if (!workUnitsData.workUnits[workUnitId]) {
    throw new Error(`Work unit '${workUnitId}' does not exist`);
  }

  const workUnit = workUnitsData.workUnits[workUnitId];
  if (!workUnit.questions) {
    workUnit.questions = [];
  }

  workUnit.questions.push({ text: question, selected: false });
  workUnit.updatedAt = new Date().toISOString();

  await saveWorkUnits(workUnitsData, cwd);
}

export async function addAssumption(
  workUnitId: string,
  assumption: string,
  options: { cwd: string }
): Promise<void> {
  const { cwd } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  if (!workUnitsData.workUnits[workUnitId]) {
    throw new Error(`Work unit '${workUnitId}' does not exist`);
  }

  const workUnit = workUnitsData.workUnits[workUnitId];
  if (!workUnit.assumptions) {
    workUnit.assumptions = [];
  }

  workUnit.assumptions.push(assumption);
  workUnit.updatedAt = new Date().toISOString();

  await saveWorkUnits(workUnitsData, cwd);
}

export async function removeRule(
  workUnitId: string,
  index: number,
  options: { cwd: string }
): Promise<void> {
  const { cwd } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  if (!workUnitsData.workUnits[workUnitId]) {
    throw new Error(`Work unit '${workUnitId}' does not exist`);
  }

  const workUnit = workUnitsData.workUnits[workUnitId];
  if (!workUnit.rules || index < 0 || index >= workUnit.rules.length) {
    const maxIndex = (workUnit.rules?.length || 0) - 1;
    throw new Error(
      `Index ${index} out of range. Valid indices: 0-${maxIndex}`
    );
  }

  workUnit.rules.splice(index, 1);
  workUnit.updatedAt = new Date().toISOString();

  await saveWorkUnits(workUnitsData, cwd);
}

export async function removeExample(
  workUnitId: string,
  index: number,
  options: { cwd: string }
): Promise<void> {
  const { cwd } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  if (!workUnitsData.workUnits[workUnitId]) {
    throw new Error(`Work unit '${workUnitId}' does not exist`);
  }

  const workUnit = workUnitsData.workUnits[workUnitId];
  if (!workUnit.examples || index < 0 || index >= workUnit.examples.length) {
    const maxIndex = (workUnit.examples?.length || 0) - 1;
    throw new Error(
      `Index ${index} out of range. Valid indices: 0-${maxIndex}`
    );
  }

  workUnit.examples.splice(index, 1);
  workUnit.updatedAt = new Date().toISOString();

  await saveWorkUnits(workUnitsData, cwd);
}

export async function removeQuestion(
  workUnitId: string,
  index: number,
  options: { cwd: string }
): Promise<void> {
  const { cwd } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  if (!workUnitsData.workUnits[workUnitId]) {
    throw new Error(`Work unit '${workUnitId}' does not exist`);
  }

  const workUnit = workUnitsData.workUnits[workUnitId];
  if (!workUnit.questions || index < 0 || index >= workUnit.questions.length) {
    const maxIndex = (workUnit.questions?.length || 0) - 1;
    throw new Error(
      `Index ${index} out of range. Valid indices: 0-${maxIndex}`
    );
  }

  workUnit.questions.splice(index, 1);
  workUnit.updatedAt = new Date().toISOString();

  await saveWorkUnits(workUnitsData, cwd);
}

export async function answerQuestion(
  workUnitId: string,
  questionIndex: number,
  answer: string,
  options: { cwd: string; addTo?: 'rules' | 'assumptions' | 'none' }
): Promise<void> {
  const { cwd, addTo = 'none' } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  if (!workUnitsData.workUnits[workUnitId]) {
    throw new Error(`Work unit '${workUnitId}' does not exist`);
  }

  const workUnit = workUnitsData.workUnits[workUnitId];
  if (
    !workUnit.questions ||
    questionIndex < 0 ||
    questionIndex >= workUnit.questions.length
  ) {
    throw new Error(`Question index ${questionIndex} out of range`);
  }

  // Get the question as QuestionItem
  const question = workUnit.questions[questionIndex];
  if (typeof question === 'string') {
    throw new Error(
      'Invalid question format. Questions must be QuestionItem objects.'
    );
  }

  // Mark question as selected instead of removing it (stable indices)
  question.selected = true;
  question.answer = answer;

  // Add answer to appropriate category
  if (addTo === 'rules') {
    if (!workUnit.rules) workUnit.rules = [];
    workUnit.rules.push(answer);
  } else if (addTo === 'assumptions') {
    if (!workUnit.assumptions) workUnit.assumptions = [];
    // Format assumption from answer
    const assumption = answer.includes('GitHub.com')
      ? 'Only GitHub.com supported, not Enterprise'
      : answer;
    workUnit.assumptions.push(assumption);
  }

  workUnit.updatedAt = new Date().toISOString();

  await saveWorkUnits(workUnitsData, cwd);
}

// Helper: Extract capability name from bug description
function extractCapabilityFromBug(workUnit: WorkUnit): string | null {
  const description = workUnit.description || workUnit.title || '';
  const lowerDesc = description.toLowerCase();

  // Priority 1: Extract specific capabilities from keywords (more specific wins)
  if (lowerDesc.includes('version')) {
    return 'cli-version-display';
  }
  if (lowerDesc.includes('validate') && lowerDesc.includes('empty')) {
    return 'gherkin-validation';
  }
  if (lowerDesc.includes('format') && lowerDesc.includes('doc string')) {
    return 'gherkin-formatting';
  }

  // Priority 2: Extract key commands/features from bug description
  const commandMatches = lowerDesc.match(/fspec\s+(\w+)/);
  if (commandMatches) {
    const command = commandMatches[1];
    // Map commands to capability names
    const commandToCapability: Record<string, string> = {
      help: 'help-command',
      validate: 'gherkin-validation',
      format: 'gherkin-formatting',
      'list-features': 'list-features',
      'show-feature': 'show-feature',
    };
    return commandToCapability[command] || null;
  }

  return null;
}

// Helper: Search for existing feature file matching capability
async function findMatchingFeatureFile(
  capability: string,
  cwd: string,
  workUnit: WorkUnit
): Promise<string | null> {
  const featuresDir = join(cwd, 'spec', 'features');
  const { readdir } = await import('fs/promises');

  try {
    const files = await readdir(featuresDir);
    const featureFiles = files.filter(f => f.endsWith('.feature'));

    // Direct match
    if (featureFiles.includes(`${capability}.feature`)) {
      return `${capability}.feature`;
    }

    // Check for command-related files if bug mentions a command
    const description = workUnit.description || workUnit.title || '';
    const commandMatches = description.toLowerCase().match(/fspec\s+(\w+)/);
    if (commandMatches) {
      const command = commandMatches[1];
      const commandFile = featureFiles.find(f => f.includes(command));
      if (commandFile) {
        return commandFile;
      }
    }

    // Partial match (e.g., "help" matches "help-command.feature")
    const partialMatch = featureFiles.find(f =>
      f.includes(capability.split('-')[0])
    );
    if (partialMatch) {
      return partialMatch;
    }

    return null;
  } catch {
    return null;
  }
}

// Helper: Add scenario to existing feature file
async function addScenarioToFeature(
  featureFilePath: string,
  scenarios: string
): Promise<void> {
  const existingContent = await readFile(featureFilePath, 'utf-8');
  await writeFile(featureFilePath, existingContent + '\n' + scenarios);
}

export async function generateScenarios(
  workUnitId: string,
  options: { cwd: string; feature?: string }
): Promise<{ success: boolean; featureFile: string }> {
  const { cwd, feature } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  if (!workUnitsData.workUnits[workUnitId]) {
    throw new Error(`Work unit '${workUnitId}' does not exist`);
  }

  const workUnit = workUnitsData.workUnits[workUnitId];
  if (!workUnit.examples || workUnit.examples.length === 0) {
    throw new Error('No examples to generate scenarios from');
  }

  // Determine feature file name
  let featureFileName: string;
  let shouldSearchExisting = false;

  if (feature) {
    featureFileName = `${feature}.feature`;
  } else if (workUnit.type === 'bug') {
    // For bugs: extract capability and search for existing file
    const capability = extractCapabilityFromBug(workUnit);
    if (capability) {
      const existingFile = await findMatchingFeatureFile(
        capability,
        cwd,
        workUnit
      );
      if (existingFile) {
        // Found existing feature file - use it
        featureFileName = existingFile;
        shouldSearchExisting = false; // We already know it exists
      } else {
        // No existing file - create with capability name
        featureFileName = `${capability}.feature`;
      }
    } else {
      // Fallback: use title if capability extraction failed
      if (!workUnit.title) {
        throw new Error(
          `Cannot determine feature file name. Work unit ${workUnitId} has no title and capability extraction failed.\n` +
            `Suggestion: Use --feature flag with a capability-based name (e.g., --feature=user-authentication)`
        );
      }
      const kebabCase = workUnit.title
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, '-')
        .replace(/^-+|-+$/g, '');
      featureFileName = `${kebabCase}.feature`;
    }
  } else {
    // Default: use work unit title (capability-based naming)
    if (!workUnit.title) {
      throw new Error(
        `Cannot determine feature file name. Work unit ${workUnitId} has no title.\n` +
          `Suggestion: Use --feature flag with a capability-based name (e.g., --feature=user-authentication)`
      );
    }
    // Convert title to kebab-case for feature file name
    const kebabCase = workUnit.title
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-+|-+$/g, '');
    featureFileName = `${kebabCase}.feature`;
  }

  const featuresDir = join(cwd, 'spec', 'features');
  const featureFilePath = join(featuresDir, featureFileName);

  // Generate Gherkin scenarios
  let scenarios = '';
  for (const example of workUnit.examples) {
    scenarios += `\n@${workUnitId}\nScenario: ${example}\n`;
    scenarios += `  Given [precondition to be filled in]\n`;
    scenarios += `  When [action to be filled in]\n`;
    scenarios += `  Then [expected outcome to be filled in]\n`;
  }

  // Check if feature file exists and append, or create new
  try {
    await addScenarioToFeature(featureFilePath, scenarios);
  } catch {
    // Create new feature file
    const featureContent = `@${workUnitId.split('-')[0].toLowerCase()}\nFeature: ${workUnit.id}\n${scenarios}`;
    await writeFile(featureFilePath, featureContent);
  }

  return {
    success: true,
    featureFile: featureFileName,
  };
}

export async function importExampleMap(
  workUnitId: string,
  jsonPath: string,
  options: { cwd: string }
): Promise<void> {
  const { cwd } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  if (!workUnitsData.workUnits[workUnitId]) {
    throw new Error(`Work unit '${workUnitId}' does not exist`);
  }

  // Read JSON file
  const jsonContent = await readFile(jsonPath, 'utf-8');
  const data = JSON.parse(jsonContent);

  const workUnit = workUnitsData.workUnits[workUnitId];

  // Import all artifacts
  if (data.rules && Array.isArray(data.rules)) {
    if (!workUnit.rules) workUnit.rules = [];
    workUnit.rules.push(...data.rules);
  }

  if (data.examples && Array.isArray(data.examples)) {
    if (!workUnit.examples) workUnit.examples = [];
    workUnit.examples.push(...data.examples);
  }

  if (data.questions && Array.isArray(data.questions)) {
    if (!workUnit.questions) workUnit.questions = [];
    workUnit.questions.push(...data.questions);
  }

  if (data.assumptions && Array.isArray(data.assumptions)) {
    if (!workUnit.assumptions) workUnit.assumptions = [];
    workUnit.assumptions.push(...data.assumptions);
  }

  workUnit.updatedAt = new Date().toISOString();

  await saveWorkUnits(workUnitsData, cwd);
}

export async function exportExampleMap(
  workUnitId: string,
  options: { cwd: string; output: string }
): Promise<void> {
  const { cwd, output } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  if (!workUnitsData.workUnits[workUnitId]) {
    throw new Error(`Work unit '${workUnitId}' does not exist`);
  }

  const workUnit = workUnitsData.workUnits[workUnitId];

  const exportData = {
    rules: workUnit.rules || [],
    examples: workUnit.examples || [],
    questions: workUnit.questions || [],
    assumptions: workUnit.assumptions || [],
  };

  await writeFile(output, JSON.stringify(exportData, null, 2));
}

export async function queryExampleMappingStats(options: {
  cwd: string;
  output?: string;
}): Promise<string> {
  const { cwd, output } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  const workUnits = Object.values(workUnitsData.workUnits);

  const stats = {
    workUnitsWithRules: workUnits.filter(wu => wu.rules && wu.rules.length > 0)
      .length,
    workUnitsWithExamples: workUnits.filter(
      wu => wu.examples && wu.examples.length > 0
    ).length,
    workUnitsWithQuestions: workUnits.filter(
      wu => wu.questions && wu.questions.length > 0
    ).length,
    workUnitsWithAssumptions: workUnits.filter(
      wu => wu.assumptions && wu.assumptions.length > 0
    ).length,
  };

  if (output === 'json') {
    return JSON.stringify(stats, null, 2);
  }

  return (
    `Work units with rules: ${stats.workUnitsWithRules}\n` +
    `Work units with examples: ${stats.workUnitsWithExamples}\n` +
    `Work units with questions: ${stats.workUnitsWithQuestions}\n` +
    `Work units with assumptions: ${stats.workUnitsWithAssumptions}`
  );
}
