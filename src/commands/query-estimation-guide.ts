import { readFile } from 'fs/promises';
import { join } from 'path';

interface WorkUnit {
  id: string;
  estimate?: number;
  actualTokens?: number;
  iterations?: number;
  status?: string;
  [key: string]: unknown;
}

interface WorkUnitsData {
  workUnits: Record<string, WorkUnit>;
  states: Record<string, string[]>;
}

interface EstimationPattern {
  points: number;
  expectedTokens: string;
  expectedIterations: string;
  confidence: string;
}

interface EstimationGuideResult {
  patterns: EstimationPattern[];
}

export async function queryEstimationGuide(options: {
  cwd?: string;
}): Promise<EstimationGuideResult> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec', 'work-units.json');

  try {
    // Read work units
    const content = await readFile(workUnitsFile, 'utf-8');
    const data: WorkUnitsData = JSON.parse(content);

    // Get completed work units
    const completedWorkUnits = Object.values(data.workUnits).filter(wu => wu.status === 'done');

    // Group by story points
    const byPoints: Record<number, { tokens: number[]; iterations: number[] }> = {};

    for (const wu of completedWorkUnits) {
      if (wu.estimate && wu.actualTokens !== undefined && wu.iterations !== undefined) {
        if (!byPoints[wu.estimate]) {
          byPoints[wu.estimate] = { tokens: [], iterations: [] };
        }
        byPoints[wu.estimate].tokens.push(wu.actualTokens);
        byPoints[wu.estimate].iterations.push(wu.iterations);
      }
    }

    // Calculate patterns
    const patterns: EstimationPattern[] = [];

    for (const [points, data] of Object.entries(byPoints)) {
      const pointsNum = parseInt(points);
      const minTokens = Math.min(...data.tokens);
      const maxTokens = Math.max(...data.tokens);
      const minIterations = Math.min(...data.iterations);
      const maxIterations = Math.max(...data.iterations);

      // Format tokens in k notation
      const formatTokens = (tokens: number): string => {
        return `${Math.round(tokens / 1000)}k`;
      };

      // Determine confidence based on sample size
      let confidence = 'low';
      if (data.tokens.length >= 4) {
        confidence = 'high';
      } else if (data.tokens.length >= 2) {
        confidence = 'medium';
      }

      patterns.push({
        points: pointsNum,
        expectedTokens: `${formatTokens(minTokens)}-${formatTokens(maxTokens)}`,
        expectedIterations: `${minIterations}-${maxIterations}`,
        confidence,
      });
    }

    // Sort by points
    patterns.sort((a, b) => a.points - b.points);

    return { patterns };
  } catch (error: unknown) {
    if (error instanceof Error) {
      throw new Error(`Failed to query estimation guide: ${error.message}`);
    }
    throw error;
  }
}
