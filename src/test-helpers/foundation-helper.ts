/**
 * Test helper for creating minimal foundation.json files
 */

import { writeFile } from 'fs/promises';
import { join } from 'path';

/**
 * Create a minimal foundation.json file for testing
 */
export async function createMinimalFoundation(testDir: string): Promise<void> {
  await writeFile(
    join(testDir, 'spec', 'foundation.json'),
    JSON.stringify({
      version: '2.0.0',
      project: {
        name: 'Test Project',
        vision: 'Test project vision',
        projectType: 'cli-tool',
      },
      problemSpace: {
        primaryProblem: {
          title: 'Test Problem',
          description: 'Test problem description',
          impact: 'high',
        },
      },
      solutionSpace: {
        overview: 'Test solution',
        capabilities: [],
      },
      personas: [],
    })
  );
}
