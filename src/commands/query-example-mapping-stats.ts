import type { WorkUnitsData, WorkUnit } from '../types';
import type { Command } from 'commander';
import { ensureWorkUnitsFile } from '../utils/ensure-files';

interface QueryExampleMappingStatsOptions {
  workUnitId?: string;
  hasQuestions?: boolean;
  questionsFor?: string;
  cwd?: string;
}

interface ExampleMappingStats {
  workUnitId: string;
  title?: string;
  status: string;
  rules: number;
  examples: number;
  questions: number;
  assumptions: number;
  completenessScore: number;
}

interface QueryExampleMappingStatsResult {
  workUnits?: ExampleMappingStats[];
  workUnitsWithRules?: number;
  workUnitsWithExamples?: number;
  workUnitsWithQuestions?: number;
  workUnitsWithAssumptions?: number;
  avgRulesPerWorkUnit?: number;
  avgExamplesPerWorkUnit?: number;
  avgQuestionsPerWorkUnit?: number;
  avgAssumptionsPerWorkUnit?: number;
}

function calculateCompletenessScore(workUnit: WorkUnit): number {
  const hasRules = (workUnit.rules?.length || 0) > 0;
  const hasExamples = (workUnit.examples?.length || 0) > 0;
  const hasNoQuestions = (workUnit.questions?.length || 0) === 0;

  let score = 0;
  if (hasRules) {
    score += 33;
  }
  if (hasExamples) {
    score += 34;
  }
  if (hasNoQuestions) {
    score += 33;
  }

  return score;
}

export async function queryExampleMappingStats(
  options: QueryExampleMappingStatsOptions = {}
): Promise<QueryExampleMappingStatsResult> {
  const cwd = options.cwd || process.cwd();

  // Read work units
  const data: WorkUnitsData = await ensureWorkUnitsFile(cwd);

  let workUnits = Object.values(data.workUnits);

  // Filter by specific work unit
  if (options.workUnitId) {
    workUnits = workUnits.filter(wu => wu.id === options.workUnitId);
    if (workUnits.length === 0) {
      throw new Error(`Work unit '${options.workUnitId}' does not exist`);
    }
  }

  // Filter by hasQuestions
  if (options.hasQuestions !== undefined) {
    if (options.hasQuestions) {
      workUnits = workUnits.filter(wu => (wu.questions?.length || 0) > 0);
    } else {
      workUnits = workUnits.filter(wu => (wu.questions?.length || 0) === 0);
    }
  }

  // Filter by questionsFor (person mentioned)
  if (options.questionsFor) {
    const mention = `@${options.questionsFor}`;
    workUnits = workUnits.filter(wu => {
      if (!wu.questions) {
        return false;
      }
      return wu.questions.some(q => q.includes(mention));
    });
  }

  // Build stats for each work unit
  const stats: ExampleMappingStats[] = workUnits.map(wu => ({
    workUnitId: wu.id,
    title: wu.title,
    status: wu.status,
    rules: wu.rules?.length || 0,
    examples: wu.examples?.length || 0,
    questions: wu.questions?.length || 0,
    assumptions: wu.assumptions?.length || 0,
    completenessScore: calculateCompletenessScore(wu),
  }));

  // Calculate aggregate statistics
  const workUnitsWithRules = workUnits.filter(
    wu => (wu.rules?.length || 0) > 0
  ).length;
  const workUnitsWithExamples = workUnits.filter(
    wu => (wu.examples?.length || 0) > 0
  ).length;
  const workUnitsWithQuestions = workUnits.filter(
    wu => (wu.questions?.length || 0) > 0
  ).length;
  const workUnitsWithAssumptions = workUnits.filter(
    wu => (wu.assumptions?.length || 0) > 0
  ).length;

  const totalRules = workUnits.reduce(
    (sum, wu) => sum + (wu.rules?.length || 0),
    0
  );
  const totalExamples = workUnits.reduce(
    (sum, wu) => sum + (wu.examples?.length || 0),
    0
  );
  const totalQuestions = workUnits.reduce(
    (sum, wu) => sum + (wu.questions?.length || 0),
    0
  );
  const totalAssumptions = workUnits.reduce(
    (sum, wu) => sum + (wu.assumptions?.length || 0),
    0
  );

  const avgRulesPerWorkUnit =
    workUnits.length > 0 ? totalRules / workUnits.length : 0;
  const avgExamplesPerWorkUnit =
    workUnits.length > 0 ? totalExamples / workUnits.length : 0;
  const avgQuestionsPerWorkUnit =
    workUnits.length > 0 ? totalQuestions / workUnits.length : 0;
  const avgAssumptionsPerWorkUnit =
    workUnits.length > 0 ? totalAssumptions / workUnits.length : 0;

  return {
    workUnits: stats,
    workUnitsWithRules,
    workUnitsWithExamples,
    workUnitsWithQuestions,
    workUnitsWithAssumptions,
    avgRulesPerWorkUnit,
    avgExamplesPerWorkUnit,
    avgQuestionsPerWorkUnit,
    avgAssumptionsPerWorkUnit,
  };
}

export function registerQueryExampleMappingStatsCommand(program: Command): void {
  program
    .command('query-example-mapping-stats')
    .description('Show example mapping coverage statistics')
    .option('--format <format>', 'Output format: text or json', 'text')
    .action(async (options: { format?: string }) => {
      try {
        const result = await queryExampleMappingStats({
          format: options.format as 'text' | 'json',
        });
        if (options.format === 'json') {
          console.log(JSON.stringify(result, null, 2));
        }
      } catch (error: any) {
        console.error(chalk.red('✗ Query failed:'), error.message);
        process.exit(1);
      }
    });
}
