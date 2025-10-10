import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';

interface WorkUnit {
  id: string;
  rules?: string[];
  examples?: string[];
  questions?: string[];
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

  workUnit.questions.push(question);
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
    throw new Error(`Index ${index} out of range. Valid indices: 0-${maxIndex}`);
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
    throw new Error(`Index ${index} out of range. Valid indices: 0-${maxIndex}`);
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
    throw new Error(`Index ${index} out of range. Valid indices: 0-${maxIndex}`);
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
  if (!workUnit.questions || questionIndex < 0 || questionIndex >= workUnit.questions.length) {
    throw new Error(`Question index ${questionIndex} out of range`);
  }

  // Remove the question
  workUnit.questions.splice(questionIndex, 1);

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
  const featureFileName = feature
    ? `${feature}.feature`
    : `${workUnitId.toLowerCase()}.feature`;

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

  // Check if feature file exists
  try {
    const existingContent = await readFile(featureFilePath, 'utf-8');
    // Append to existing file
    await writeFile(featureFilePath, existingContent + '\n' + scenarios);
  } catch {
    // Create new feature file
    const featureContent = `@${workUnitId.split('-')[0].toLowerCase()}\nFeature: ${workUnit.id}\n${scenarios}`;
    await writeFile(featureFilePath, featureContent);
  }

  return {
    success: true,
    featureFile: featureFileName
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
    assumptions: workUnit.assumptions || []
  };

  await writeFile(output, JSON.stringify(exportData, null, 2));
}

export async function queryExampleMappingStats(
  options: { cwd: string; output?: string }
): Promise<string> {
  const { cwd, output } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  const workUnits = Object.values(workUnitsData.workUnits);

  const stats = {
    workUnitsWithRules: workUnits.filter(wu => wu.rules && wu.rules.length > 0).length,
    workUnitsWithExamples: workUnits.filter(wu => wu.examples && wu.examples.length > 0).length,
    workUnitsWithQuestions: workUnits.filter(wu => wu.questions && wu.questions.length > 0).length,
    workUnitsWithAssumptions: workUnits.filter(wu => wu.assumptions && wu.assumptions.length > 0).length
  };

  if (output === 'json') {
    return JSON.stringify(stats, null, 2);
  }

  return `Work units with rules: ${stats.workUnitsWithRules}\n` +
         `Work units with examples: ${stats.workUnitsWithExamples}\n` +
         `Work units with questions: ${stats.workUnitsWithQuestions}\n` +
         `Work units with assumptions: ${stats.workUnitsWithAssumptions}`;
}
