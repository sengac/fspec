import { readFile, writeFile, mkdir } from 'fs/promises';
import { join } from 'path';

interface QuestionItem {
  text: string;
  selected: boolean;
  answer?: string;
}

interface WorkUnit {
  id: string;
  title: string;
  description?: string;
  status:
    | 'backlog'
    | 'specifying'
    | 'testing'
    | 'implementing'
    | 'validating'
    | 'done'
    | 'blocked';
  epic?: string;
  parent?: string;
  children?: string[];
  estimate?: number;
  relationships?: {
    blocks?: string[];
    blockedBy?: string[];
    dependsOn?: string[];
    relatesTo?: string[];
  };
  rules?: string[];
  examples?: string[];
  questions?: (string | QuestionItem)[];
  assumptions?: string[];
  stateHistory?: Array<{ state: string; timestamp: string; reason?: string }>;
  blockedReason?: string;
  metrics?: {
    actualTokens?: number;
    iterations?: number;
  };
  createdAt: string;
  updatedAt: string;
}

interface WorkUnitsData {
  workUnits: Record<string, WorkUnit>;
  states: {
    backlog: string[];
    specifying: string[];
    testing: string[];
    implementing: string[];
    validating: string[];
    done: string[];
    blocked: string[];
  };
}

interface PrefixesData {
  prefixes: Record<string, { description: string }>;
}

interface EpicsData {
  epics: Record<string, { id: string; title: string; workUnits: string[] }>;
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

async function loadPrefixes(cwd: string): Promise<PrefixesData> {
  const prefixesFile = join(cwd, 'spec', 'prefixes.json');
  const content = await readFile(prefixesFile, 'utf-8');
  return JSON.parse(content);
}

async function loadEpics(cwd: string): Promise<EpicsData> {
  const epicsFile = join(cwd, 'spec', 'epics.json');
  const content = await readFile(epicsFile, 'utf-8');
  return JSON.parse(content);
}

async function saveEpics(data: EpicsData, cwd: string): Promise<void> {
  const epicsFile = join(cwd, 'spec', 'epics.json');
  await writeFile(epicsFile, JSON.stringify(data, null, 2));
}

function getNextWorkUnitId(
  prefix: string,
  workUnits: Record<string, WorkUnit>
): string {
  const existingIds = Object.keys(workUnits)
    .filter(id => id.startsWith(prefix + '-'))
    .map(id => {
      const parts = id.split('-');
      return parseInt(parts[1], 10);
    })
    .filter(num => !isNaN(num));

  const maxId = existingIds.length > 0 ? Math.max(...existingIds) : 0;
  const nextId = maxId + 1;
  return `${prefix}-${String(nextId).padStart(3, '0')}`;
}

function calculateNestingDepth(
  workUnitId: string,
  workUnits: Record<string, WorkUnit>
): number {
  let depth = 0;
  let currentId: string | undefined = workUnitId;

  while (currentId && workUnits[currentId]?.parent) {
    depth++;
    currentId = workUnits[currentId].parent;
    if (depth > 10) break; // Safety check
  }

  return depth;
}

export async function createWorkUnit(
  prefix: string,
  title: string,
  options: {
    cwd: string;
    description?: string;
    epic?: string;
    parent?: string;
  }
): Promise<void> {
  const { cwd, description, epic, parent } = options;

  // Validate title
  if (!title || title.trim() === '') {
    throw new Error('Title is required');
  }

  // Load data
  const prefixes = await loadPrefixes(cwd);
  if (!prefixes.prefixes[prefix]) {
    throw new Error(
      `Prefix '${prefix}' is not registered. Run 'fspec create-prefix ${prefix}' first`
    );
  }

  const workUnitsData = await loadWorkUnits(cwd);

  // Validate parent exists
  if (parent && !workUnitsData.workUnits[parent]) {
    throw new Error(`Parent work unit '${parent}' does not exist`);
  }

  // Check nesting depth if parent exists
  if (parent) {
    const depth = calculateNestingDepth(parent, workUnitsData.workUnits);
    // Maximum depth of 3 means 3 levels (depths 0,1,2), so parent at depth 2+ cannot have children
    if (depth >= 2) {
      throw new Error('Maximum nesting depth (3) exceeded');
    }
  }

  // Validate epic exists
  if (epic) {
    const epics = await loadEpics(cwd);
    if (!epics.epics[epic]) {
      throw new Error(`Epic '${epic}' does not exist`);
    }
  }

  // Generate ID
  const id = getNextWorkUnitId(prefix, workUnitsData.workUnits);

  // Create work unit
  const now = new Date().toISOString();
  const workUnit: WorkUnit = {
    id,
    title,
    status: 'backlog',
    createdAt: now,
    updatedAt: now,
    stateHistory: [{ state: 'backlog', timestamp: now }],
  };

  if (description) workUnit.description = description;
  if (epic) workUnit.epic = epic;
  if (parent) {
    workUnit.parent = parent;
    // Update parent's children array
    if (!workUnitsData.workUnits[parent].children) {
      workUnitsData.workUnits[parent].children = [];
    }
    workUnitsData.workUnits[parent].children.push(id);
  }

  // Add to work units
  workUnitsData.workUnits[id] = workUnit;

  // Add to states index
  workUnitsData.states.backlog.push(id);

  // Save
  await saveWorkUnits(workUnitsData, cwd);

  // Update epic if specified
  if (epic) {
    const epics = await loadEpics(cwd);
    if (!epics.epics[epic].workUnits) {
      epics.epics[epic].workUnits = [];
    }
    epics.epics[epic].workUnits.push(id);
    await saveEpics(epics, cwd);
  }
}

export async function updateWorkUnit(
  workUnitId: string,
  updates: {
    title?: string;
    description?: string;
    epic?: string;
    status?: string;
    parent?: string;
    blockedReason?: string;
    reason?: string;
  },
  options: { cwd: string }
): Promise<{ warnings?: string[] }> {
  const { cwd } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  if (!workUnitsData.workUnits[workUnitId]) {
    throw new Error(`Work unit '${workUnitId}' does not exist`);
  }

  const workUnit = workUnitsData.workUnits[workUnitId];
  const warnings: string[] = [];

  // Update title
  if (updates.title !== undefined) {
    workUnit.title = updates.title;
  }

  // Update description
  if (updates.description !== undefined) {
    workUnit.description = updates.description;
  }

  // Update epic
  if (updates.epic !== undefined) {
    const epics = await loadEpics(cwd);
    if (!epics.epics[updates.epic]) {
      throw new Error(`Epic '${updates.epic}' does not exist`);
    }

    // Remove from old epic
    if (workUnit.epic) {
      const oldEpic = epics.epics[workUnit.epic];
      if (oldEpic?.workUnits) {
        oldEpic.workUnits = oldEpic.workUnits.filter(id => id !== workUnitId);
      }
    }

    // Add to new epic
    if (!epics.epics[updates.epic].workUnits) {
      epics.epics[updates.epic].workUnits = [];
    }
    epics.epics[updates.epic].workUnits.push(workUnitId);
    workUnit.epic = updates.epic;

    await saveEpics(epics, cwd);
  }

  // Update parent
  if (updates.parent !== undefined) {
    if (!workUnitsData.workUnits[updates.parent]) {
      throw new Error(`Parent work unit '${updates.parent}' does not exist`);
    }

    // Check for circular relationship
    let current: string | undefined = updates.parent;
    while (current) {
      if (current === workUnitId) {
        throw new Error('Circular parent relationship detected');
      }
      current = workUnitsData.workUnits[current]?.parent;
    }

    workUnit.parent = updates.parent;
  }

  // Update status
  if (updates.status !== undefined) {
    await updateWorkUnitStatus(workUnitId, updates.status, {
      cwd,
      blockedReason: updates.blockedReason,
      reason: updates.reason,
    });
    return { warnings };
  }

  // Update timestamp
  workUnit.updatedAt = new Date().toISOString();

  await saveWorkUnits(workUnitsData, cwd);

  return { warnings };
}

async function updateWorkUnitStatus(
  workUnitId: string,
  newStatus: string,
  options: { cwd: string; blockedReason?: string; reason?: string }
): Promise<void> {
  const { cwd, blockedReason, reason } = options;
  const workUnitsData = await loadWorkUnits(cwd);
  const workUnit = workUnitsData.workUnits[workUnitId];
  const oldStatus = workUnit.status;

  // Validate state transitions (ACDD enforcement)
  const validTransitions: Record<string, string[]> = {
    backlog: ['specifying', 'blocked'],
    specifying: ['testing', 'blocked'],
    testing: ['implementing', 'blocked'],
    implementing: ['validating', 'blocked', 'testing', 'specifying'],
    validating: ['done', 'blocked', 'implementing', 'specifying'],
    done: ['specifying', 'testing', 'implementing', 'validating', 'blocked'], // Can move backward when mistakes discovered
    blocked: ['backlog', 'specifying', 'testing', 'implementing', 'validating'],
  };

  if (!validTransitions[oldStatus]?.includes(newStatus)) {
    if (newStatus === 'backlog' && oldStatus !== 'backlog') {
      throw new Error(
        "Cannot move work back to backlog. Use 'blocked' state if work cannot progress"
      );
    }

    if (oldStatus === 'backlog' && newStatus === 'testing') {
      throw new Error(
        "Invalid state transition. Must move to 'specifying' state first. ACDD requires specification before testing"
      );
    }

    if (oldStatus === 'backlog' && newStatus === 'implementing') {
      throw new Error(
        "Invalid state transition. Must move to 'specifying' state first. ACDD requires specification before testing"
      );
    }

    if (oldStatus === 'specifying' && newStatus === 'implementing') {
      throw new Error(
        "Invalid state transition. Must move to 'testing' state first. ACDD requires tests before implementation"
      );
    }

    throw new Error(
      `Invalid state transition from ${oldStatus} to ${newStatus}`
    );
  }

  // Validate blocked state requires reason
  if (newStatus === 'blocked' && !blockedReason) {
    throw new Error(
      "Blocked reason is required. Use --blocked-reason='description of blocker'"
    );
  }

  // Check for unanswered questions before moving to testing
  if (oldStatus === 'specifying' && newStatus === 'testing') {
    if (workUnit.questions && workUnit.questions.length > 0) {
      // Filter for unselected questions only
      const unansweredQuestions = workUnit.questions.filter(q => {
        if (typeof q === 'string') {
          throw new Error(
            'Invalid question format. Questions must be QuestionItem objects.'
          );
        }
        return !q.selected;
      });

      if (unansweredQuestions.length > 0) {
        const questionList = unansweredQuestions
          .map((q, i) => {
            const questionItem = q as QuestionItem;
            const originalIndex = workUnit.questions!.indexOf(q);
            return `\n  ${originalIndex}. ${questionItem.text}`;
          })
          .join('');

        throw new Error(
          `Unanswered questions prevent state transition to testing. ${unansweredQuestions.length} question(s) must be answered first.${questionList}\n\nAnswer questions with 'fspec answer-question' or remove them using 'fspec remove-question'`
        );
      }
    }

    // Check for scenarios before moving to testing
    const { glob } = await import('tinyglobby');
    const featuresDir = join(cwd, 'spec', 'features');
    const featureFiles = await glob('**/*.feature', { cwd: featuresDir });

    let hasScenarios = false;
    for (const file of featureFiles) {
      const content = await readFile(join(featuresDir, file), 'utf-8');
      if (content.includes(`@${workUnitId}`)) {
        hasScenarios = true;
        break;
      }
    }

    if (!hasScenarios) {
      throw new Error(
        `No Gherkin scenarios found. At least one scenario must be tagged with @${workUnitId}. Use 'fspec generate-scenarios ${workUnitId}' or manually tag scenarios`
      );
    }
  }

  // Check parent cannot be done with incomplete children
  if (
    newStatus === 'done' &&
    workUnit.children &&
    workUnit.children.length > 0
  ) {
    const incompleteChildren = workUnit.children.filter(
      childId => workUnitsData.workUnits[childId]?.status !== 'done'
    );

    if (incompleteChildren.length > 0) {
      const childStatuses = incompleteChildren.map(
        id => `${id} (status: ${workUnitsData.workUnits[id].status})`
      );
      throw new Error(
        `Cannot mark parent as done. Incomplete children: ${childStatuses.join(', ')}. Complete all children first`
      );
    }
  }

  // Remove from old state array
  const oldStateArray =
    workUnitsData.states[oldStatus as keyof typeof workUnitsData.states];
  const index = oldStateArray.indexOf(workUnitId);
  if (index > -1) {
    oldStateArray.splice(index, 1);
  }

  // Add to new state array
  const newStateArray =
    workUnitsData.states[newStatus as keyof typeof workUnitsData.states];
  if (newStateArray && !newStateArray.includes(workUnitId)) {
    newStateArray.push(workUnitId);
  }

  // Update work unit
  workUnit.status = newStatus as WorkUnit['status'];
  workUnit.updatedAt = new Date().toISOString();

  // Update state history
  if (!workUnit.stateHistory) {
    workUnit.stateHistory = [];
  }
  workUnit.stateHistory.push({
    state: newStatus,
    timestamp: workUnit.updatedAt,
    reason,
  });

  // Handle blocked state
  if (newStatus === 'blocked') {
    workUnit.blockedReason = blockedReason;
  } else if (oldStatus === 'blocked') {
    delete workUnit.blockedReason;
  }

  // Auto-unblock work units when this blocker is marked as done
  if (newStatus === 'done' && workUnit.relationships?.blocks) {
    for (const blockedId of workUnit.relationships.blocks) {
      const blockedUnit = workUnitsData.workUnits[blockedId];
      if (blockedUnit && blockedUnit.status === 'blocked') {
        // Check if all blockers are now done
        const blockedBy = blockedUnit.relationships?.blockedBy || [];
        const stillBlocked = blockedBy.some(blockerId => {
          const blocker = workUnitsData.workUnits[blockerId];
          return blocker && blocker.status !== 'done';
        });

        if (!stillBlocked) {
          // Unblock the work unit
          blockedUnit.status = 'backlog';
          delete blockedUnit.blockedReason;
          blockedUnit.updatedAt = new Date().toISOString();

          // Move to backlog state array
          const blockedIndex = workUnitsData.states.blocked.indexOf(blockedId);
          if (blockedIndex > -1) {
            workUnitsData.states.blocked.splice(blockedIndex, 1);
          }
          if (!workUnitsData.states.backlog.includes(blockedId)) {
            workUnitsData.states.backlog.push(blockedId);
          }
        }
      }
    }
  }

  await saveWorkUnits(workUnitsData, cwd);
}

export async function deleteWorkUnit(
  workUnitId: string,
  options: { cwd: string; force?: boolean; cascadeDependencies?: boolean }
): Promise<void> {
  const { cwd, force, cascadeDependencies } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  if (!workUnitsData.workUnits[workUnitId]) {
    throw new Error(`Work unit '${workUnitId}' does not exist`);
  }

  const workUnit = workUnitsData.workUnits[workUnitId];

  // Check for children
  if (workUnit.children && workUnit.children.length > 0) {
    throw new Error(
      `Cannot delete work unit with children: ${workUnit.children.join(', ')}. Delete children first or remove parent relationship`
    );
  }

  // Check if blocking other work
  if (
    workUnit.relationships?.blocks &&
    workUnit.relationships.blocks.length > 0
  ) {
    if (!cascadeDependencies) {
      throw new Error(
        `Cannot delete work unit that blocks other work: ${workUnit.relationships.blocks.join(', ')}. Remove blocking relationships first`
      );
    }
  }

  // Remove from states index
  for (const state of Object.keys(workUnitsData.states)) {
    const stateArray =
      workUnitsData.states[state as keyof typeof workUnitsData.states];
    const index = stateArray.indexOf(workUnitId);
    if (index > -1) {
      stateArray.splice(index, 1);
    }
  }

  // Clean up bidirectional relationships
  if (cascadeDependencies && workUnit.relationships) {
    if (workUnit.relationships.blocks) {
      for (const blockedId of workUnit.relationships.blocks) {
        const blocked = workUnitsData.workUnits[blockedId];
        if (blocked?.relationships?.blockedBy) {
          blocked.relationships.blockedBy =
            blocked.relationships.blockedBy.filter(id => id !== workUnitId);
        }
      }
    }
  }

  // Delete work unit
  delete workUnitsData.workUnits[workUnitId];

  await saveWorkUnits(workUnitsData, cwd);
}

export async function showWorkUnit(
  workUnitId: string,
  options: { cwd: string; output?: string }
): Promise<string> {
  const { cwd, output } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  if (!workUnitsData.workUnits[workUnitId]) {
    throw new Error(`Work unit '${workUnitId}' does not exist`);
  }

  const workUnit = workUnitsData.workUnits[workUnitId];

  if (output === 'json') {
    return JSON.stringify(workUnit, null, 2);
  }

  // Format output
  let result = `Work Unit: ${workUnit.id}\n`;
  result += `Title: ${workUnit.title}\n`;
  if (workUnit.description) result += `Description: ${workUnit.description}\n`;
  result += `Status: ${workUnit.status}\n`;
  if (workUnit.estimate) result += `Estimate: ${workUnit.estimate} points\n`;
  if (workUnit.epic) result += `Epic: ${workUnit.epic}\n`;
  if (workUnit.parent) result += `Parent: ${workUnit.parent}\n`;
  if (workUnit.children && workUnit.children.length > 0) {
    result += `Children: ${workUnit.children.join(', ')}\n`;
  }

  // Show relationships
  if (workUnit.relationships) {
    if (
      workUnit.relationships.blocks &&
      workUnit.relationships.blocks.length > 0
    ) {
      result += `Blocks: ${workUnit.relationships.blocks.join(', ')}\n`;
    }
    if (
      workUnit.relationships.dependsOn &&
      workUnit.relationships.dependsOn.length > 0
    ) {
      result += `Depends On: ${workUnit.relationships.dependsOn.join(', ')}\n`;
    }
    if (
      workUnit.relationships.relatesTo &&
      workUnit.relationships.relatesTo.length > 0
    ) {
      result += `Related To: ${workUnit.relationships.relatesTo.join(', ')}\n`;
    }
  }

  // Show example mapping data
  if (workUnit.rules && workUnit.rules.length > 0) {
    result += `\nRules:\n${workUnit.rules.map((r, i) => `  ${i}. ${r}`).join('\n')}\n`;
  }
  if (workUnit.examples && workUnit.examples.length > 0) {
    result += `\nExamples:\n${workUnit.examples.map((e, i) => `  ${i}. ${e}`).join('\n')}\n`;
  }
  if (workUnit.questions && workUnit.questions.length > 0) {
    // Filter for unselected questions only
    const unselectedQuestions = workUnit.questions
      .map((q, index) => {
        if (typeof q === 'string') {
          throw new Error(
            'Invalid question format. Questions must be QuestionItem objects.'
          );
        }
        return { index, ...q };
      })
      .filter(q => !q.selected);

    if (unselectedQuestions.length > 0) {
      result += `\nQuestions:\n${unselectedQuestions.map(q => `  ${q.index}. ${q.text}`).join('\n')}\n`;
    }
  }
  if (workUnit.assumptions && workUnit.assumptions.length > 0) {
    result += `\nAssumptions:\n${workUnit.assumptions.map((a, i) => `  ${i}. ${a}`).join('\n')}\n`;
  }

  return result;
}

export async function listWorkUnits(options: {
  cwd: string;
  status?: string;
  prefix?: string;
  epic?: string;
}): Promise<string> {
  const { cwd, status, prefix, epic } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  let workUnits = Object.values(workUnitsData.workUnits);

  // Filter by status
  if (status) {
    workUnits = workUnits.filter(wu => wu.status === status);
  }

  // Filter by prefix
  if (prefix) {
    workUnits = workUnits.filter(wu => wu.id.startsWith(prefix + '-'));
  }

  // Filter by epic
  if (epic) {
    workUnits = workUnits.filter(wu => wu.epic === epic);
  }

  // Format output
  let result = '';
  for (const wu of workUnits) {
    result += `${wu.id}: ${wu.title} [${wu.status}]\n`;
  }

  return result;
}

export async function validateWorkUnits(options: {
  cwd: string;
}): Promise<{ valid: boolean; errors: string; checks: string[] }> {
  const { cwd } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  const checks = [
    'JSON schema compliance',
    'parent/child consistency',
    'unique work unit IDs',
    'dependency arrays contain valid work unit IDs',
    'bidirectional consistency',
    'rules are strings',
    'examples are strings',
    'questions are QuestionItem objects',
    'assumptions are strings',
  ];

  const errors: string[] = [];

  // Validate each work unit
  for (const [id, wu] of Object.entries(workUnitsData.workUnits)) {
    // Validate status (only if present)
    if (wu.status) {
      const validStatuses = [
        'backlog',
        'specifying',
        'testing',
        'implementing',
        'validating',
        'done',
        'blocked',
      ];
      if (!validStatuses.includes(wu.status)) {
        errors.push(
          `Invalid status value for ${id}: ${wu.status}. Valid values: ${validStatuses.join(', ')}`
        );
      }

      // Validate state consistency
      let foundInState = false;
      for (const [state, ids] of Object.entries(workUnitsData.states)) {
        if (ids.includes(id)) {
          if (state !== wu.status) {
            errors.push(
              `State consistency error: ${id} has status '${wu.status}' but is in '${state}' array. Run 'fspec repair-work-units' to fix inconsistencies`
            );
          }
          foundInState = true;
        }
      }
    }

    // Validate dependency arrays contain valid work unit IDs
    if (wu.relationships) {
      const relationshipTypes = [
        'blocks',
        'blockedBy',
        'dependsOn',
        'relatesTo',
      ] as const;
      for (const relType of relationshipTypes) {
        const deps = wu.relationships[relType];
        if (deps) {
          for (const depId of deps) {
            if (!workUnitsData.workUnits[depId]) {
              errors.push(
                `Dependency validation error: ${id} has ${relType} relationship with non-existent work unit '${depId}'`
              );
            }
          }
        }
      }

      // Validate bidirectional consistency for blocks/blockedBy
      if (wu.relationships.blocks) {
        for (const blockedId of wu.relationships.blocks) {
          const blockedUnit = workUnitsData.workUnits[blockedId];
          if (blockedUnit && blockedUnit.relationships?.blockedBy) {
            if (!blockedUnit.relationships.blockedBy.includes(id)) {
              errors.push(
                `Bidirectional consistency error: ${id} blocks ${blockedId} but ${blockedId} does not list ${id} in blockedBy`
              );
            }
          } else if (blockedUnit) {
            errors.push(
              `Bidirectional consistency error: ${id} blocks ${blockedId} but ${blockedId} has no blockedBy relationship`
            );
          }
        }
      }

      if (wu.relationships.blockedBy) {
        for (const blockerId of wu.relationships.blockedBy) {
          const blockerUnit = workUnitsData.workUnits[blockerId];
          if (blockerUnit && blockerUnit.relationships?.blocks) {
            if (!blockerUnit.relationships.blocks.includes(id)) {
              errors.push(
                `Bidirectional consistency error: ${id} blockedBy ${blockerId} but ${blockerId} does not list ${id} in blocks`
              );
            }
          } else if (blockerUnit) {
            errors.push(
              `Bidirectional consistency error: ${id} blockedBy ${blockerId} but ${blockerId} has no blocks relationship`
            );
          }
        }
      }
    }
  }

  return {
    valid: errors.length === 0,
    errors: errors.join('\n'),
    checks,
  };
}

export async function repairWorkUnits(options: {
  cwd: string;
}): Promise<{ success: boolean; message: string }> {
  const { cwd } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  const repairs: string[] = [];

  // Fix state index inconsistencies
  for (const [id, wu] of Object.entries(workUnitsData.workUnits)) {
    for (const [state, ids] of Object.entries(workUnitsData.states)) {
      const isInState = ids.includes(id);
      const shouldBeInState = state === wu.status;

      if (isInState && !shouldBeInState) {
        // Remove from wrong state
        workUnitsData.states[state as keyof typeof workUnitsData.states] =
          ids.filter(i => i !== id);
        repairs.push(`Moved ${id} from ${state} to ${wu.status}`);
      }

      if (!isInState && shouldBeInState) {
        // Add to correct state
        workUnitsData.states[wu.status].push(id);
      }
    }
  }

  // Fix bidirectional dependencies
  for (const [id, wu] of Object.entries(workUnitsData.workUnits)) {
    if (wu.relationships?.blocks) {
      for (const blockedId of wu.relationships.blocks) {
        const blocked = workUnitsData.workUnits[blockedId];
        if (blocked) {
          if (!blocked.relationships) blocked.relationships = {};
          if (!blocked.relationships.blockedBy)
            blocked.relationships.blockedBy = [];
          if (!blocked.relationships.blockedBy.includes(id)) {
            blocked.relationships.blockedBy.push(id);
            repairs.push(`Repaired 1 bidirectional dependency`);
          }
        }
      }
    }
  }

  await saveWorkUnits(workUnitsData, cwd);

  return {
    success: true,
    message: repairs.length > 0 ? repairs.join(', ') : 'No repairs needed',
  };
}

export async function queryWorkUnit(
  workUnitId: string | null,
  options: {
    cwd: string;
    status?: string;
    hasQuestions?: boolean;
    questionsFor?: string;
    showCycleTime?: boolean;
    output?: string;
  }
): Promise<string> {
  const { cwd, status, hasQuestions, questionsFor, showCycleTime, output } =
    options;
  const workUnitsData = await loadWorkUnits(cwd);

  // Query specific work unit
  if (workUnitId && showCycleTime) {
    const wu = workUnitsData.workUnits[workUnitId];
    if (!wu || !wu.stateHistory) {
      throw new Error(
        `Work unit '${workUnitId}' does not exist or has no state history`
      );
    }

    let result = 'State Durations:\n';
    let totalHours = 0;

    for (let i = 0; i < wu.stateHistory.length - 1; i++) {
      const current = wu.stateHistory[i];
      const next = wu.stateHistory[i + 1];
      const duration =
        (new Date(next.timestamp).getTime() -
          new Date(current.timestamp).getTime()) /
        (1000 * 60 * 60);
      result += `${current.state}: ${duration} hour${duration !== 1 ? 's' : ''}\n`;
      totalHours += duration;
    }

    result += `\nTotal cycle time: ${totalHours} hours`;
    return result;
  }

  // Query multiple work units
  let workUnits = Object.values(workUnitsData.workUnits);

  if (status) {
    workUnits = workUnits.filter(wu => wu.status === status);
  }

  if (hasQuestions) {
    workUnits = workUnits.filter(wu => wu.questions && wu.questions.length > 0);
  }

  if (questionsFor) {
    const mention = questionsFor.startsWith('@')
      ? questionsFor
      : `@${questionsFor}`;
    workUnits = workUnits.filter(wu =>
      wu.questions?.some(q => {
        if (typeof q === 'string') {
          throw new Error(
            'Invalid question format. Questions must be QuestionItem objects.'
          );
        }
        return q.text.includes(mention);
      })
    );
  }

  if (output === 'json') {
    return JSON.stringify(workUnits, null, 2);
  }

  return workUnits.map(wu => `${wu.id}: ${wu.title || ''}`).join('\n');
}

export async function prioritizeWorkUnit(
  workUnitId: string,
  options: {
    position?: 'top' | 'bottom' | number;
    before?: string;
    after?: string;
  },
  config: { cwd: string }
): Promise<void> {
  const { cwd } = config;
  const { position, before, after } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  const backlog = workUnitsData.states.backlog;

  // If work unit object exists, validate it's in backlog status first
  const workUnit = workUnitsData.workUnits[workUnitId];
  if (workUnit && workUnit.status !== 'backlog') {
    throw new Error(
      `Can only prioritize work units in backlog state. ${workUnitId} is in '${workUnit.status}' state`
    );
  }

  const currentIndex = backlog.indexOf(workUnitId);
  if (currentIndex === -1) {
    throw new Error(`Work unit '${workUnitId}' is not in backlog array`);
  }

  // Remove from current position
  backlog.splice(currentIndex, 1);

  // Reposition based on options
  if (before) {
    const beforeIndex = backlog.indexOf(before);
    if (beforeIndex === -1) {
      // Check if work unit exists at all to give better error message
      if (!workUnitsData.workUnits[before]) {
        throw new Error(`Work unit '${before}' does not exist`);
      }
      throw new Error(`Work unit '${before}' is not in backlog`);
    }
    backlog.splice(beforeIndex, 0, workUnitId);
  } else if (after) {
    const afterIndex = backlog.indexOf(after);
    if (afterIndex === -1) {
      // Check if work unit exists at all to give better error message
      if (!workUnitsData.workUnits[after]) {
        throw new Error(`Work unit '${after}' does not exist`);
      }
      throw new Error(`Work unit '${after}' is not in backlog`);
    }
    backlog.splice(afterIndex + 1, 0, workUnitId);
  } else if (position === 'top') {
    backlog.unshift(workUnitId);
  } else if (position === 'bottom') {
    backlog.push(workUnitId);
  } else if (typeof position === 'number') {
    backlog.splice(position, 0, workUnitId);
  }

  await saveWorkUnits(workUnitsData, cwd);
}

export async function displayBoard(options: { cwd: string }): Promise<string> {
  const { cwd } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  const stateOrder = [
    'backlog',
    'specifying',
    'testing',
    'implementing',
    'validating',
    'done',
  ] as const;
  let output = '';

  let totalInProgress = 0;
  let totalCompleted = 0;

  for (const state of stateOrder) {
    const workUnitIds = workUnitsData.states[state];
    output += `\n${state}:\n`;

    if (workUnitIds.length === 0) {
      output += '  (empty)\n';
    } else {
      for (const id of workUnitIds) {
        const wu = workUnitsData.workUnits[id];
        if (wu) {
          const estimate = wu.estimate ? `${wu.estimate} pts` : 'no estimate';
          output += `  ${id}: ${wu.title} (${estimate})\n`;

          if (wu.estimate) {
            if (state === 'done') {
              totalCompleted += wu.estimate;
            } else {
              totalInProgress += wu.estimate;
            }
          }
        }
      }
    }
  }

  output += `\nSummary:\n`;
  output += `${totalInProgress} points in progress\n`;
  output += `${totalCompleted} points completed\n`;

  return output;
}
