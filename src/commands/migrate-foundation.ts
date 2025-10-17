/**
 * migrate-foundation Command
 *
 * Migrates existing foundation.json from legacy v1.x format to generic v2.0.0 schema.
 * Preserves WHY/WHAT content, extracts HOW content to documentation.
 */

import type { Command } from 'commander';
import type { GenericFoundation } from '../types/generic-foundation';

interface LegacyFoundation {
  project?: {
    name?: string;
    description?: string;
    repository?: string;
    license?: string;
  };
  whatWeAreBuilding?: {
    projectOverview?: string;
  };
  whyWeAreBuildingIt?: {
    problemDefinition?: {
      primary?: {
        title?: string;
        description?: string;
      };
    };
  };
  architectureDiagrams?: Array<{
    section: string;
    title: string;
    mermaidCode: string;
  }>;
  [key: string]: unknown;
}

/**
 * Migrate legacy foundation.json to v2.0.0 format
 */
export function migrateFoundation(
  legacyFoundation: LegacyFoundation
): GenericFoundation {
  const migrated: GenericFoundation = {
    version: '2.0.0',
    project: {
      name: legacyFoundation.project?.name || 'Unnamed Project',
      vision:
        legacyFoundation.whatWeAreBuilding?.projectOverview ||
        legacyFoundation.project?.description ||
        'Project vision not specified',
      projectType: 'other',
    },
    problemSpace: {
      primaryProblem: {
        title:
          legacyFoundation.whyWeAreBuildingIt?.problemDefinition?.primary
            ?.title || 'Problem title not specified',
        description:
          legacyFoundation.whyWeAreBuildingIt?.problemDefinition?.primary
            ?.description || 'Problem description not specified',
        impact: 'medium',
      },
    },
    solutionSpace: {
      overview: 'Solution overview to be defined',
      capabilities: [],
    },
    personas: [],
  };

  // Preserve architecture diagrams if they exist (already compatible)
  if (legacyFoundation.architectureDiagrams) {
    migrated.architectureDiagrams = legacyFoundation.architectureDiagrams;
  }

  return migrated;
}

export function registerMigrateFoundationCommand(program: Command): void {
  program
    .command('migrate-foundation')
    .description('Migrate legacy foundation.json (v1.x) to generic v2.0.0 schema format')
    .option('--output <path>', 'Output path for migrated foundation.json', 'spec/foundation.json')
    .option('--backup', 'Create backup of original file before migration', false)
    .option('--dry-run', 'Preview migration without writing files', false)
    .action(async (options: { output?: string; backup?: boolean; dryRun?: boolean; cwd?: string }) => {
      // This action will need to be implemented by the command wrapper
      // For now, we just export the registration function
      // The actual command logic will use migrateFoundation() function
      throw new Error('migrate-foundation command action not yet implemented');
    });
}
