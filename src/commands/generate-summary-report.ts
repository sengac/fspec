import { readFile, writeFile } from 'fs/promises';
import chalk from 'chalk';
import type { Command } from 'commander';
import { join } from 'path';

interface WorkUnit {
  id: string;
  status?: string;
  estimate?: number;
  [key: string]: unknown;
}

interface WorkUnitsData {
  workUnits: Record<string, WorkUnit>;
  states: Record<string, string[]>;
}

interface SummaryReport {
  totalWorkUnits: number;
  byStatus: Record<string, number>;
  totalStoryPoints: number;
  velocity: {
    completedPoints: number;
    completedWorkUnits: number;
  };
  outputFile: string;
}

export async function generateSummaryReport(options: {
  cwd?: string;
  format?: 'markdown' | 'json';
  output?: string;
}): Promise<SummaryReport> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec', 'work-units.json');

  try {
    const fileContent = await readFile(workUnitsFile, 'utf-8');
    const data: WorkUnitsData = JSON.parse(fileContent);

    const workUnits = Object.values(data.workUnits);

    // Count by status
    const byStatus: Record<string, number> = {};
    for (const wu of workUnits) {
      const status = wu.status || 'unknown';
      byStatus[status] = (byStatus[status] || 0) + 1;
    }

    // Calculate story points
    const totalStoryPoints = workUnits.reduce(
      (sum, wu) => sum + (wu.estimate || 0),
      0
    );

    // Calculate velocity (completed work)
    const completedWorkUnits = workUnits.filter(wu => wu.status === 'done');
    const completedPoints = completedWorkUnits.reduce(
      (sum, wu) => sum + (wu.estimate || 0),
      0
    );

    const report = {
      totalWorkUnits: workUnits.length,
      byStatus,
      totalStoryPoints,
      velocity: {
        completedPoints,
        completedWorkUnits: completedWorkUnits.length,
      },
    };

    // Determine output file path
    const format = options.format || 'markdown';
    const extension = format === 'json' ? 'json' : 'md';
    const defaultOutputPath = join('spec', `summary-report.${extension}`);
    const outputPath = options.output || defaultOutputPath;
    const fullOutputPath = join(cwd, outputPath);

    // Generate report content based on format
    let content: string;
    if (format === 'json') {
      content = JSON.stringify(report, null, 2);
    } else {
      content = generateMarkdownReport(report);
    }

    // Write report to file
    await writeFile(fullOutputPath, content, 'utf-8');

    return {
      ...report,
      outputFile: outputPath,
    };
  } catch (error: unknown) {
    if (error instanceof Error) {
      throw new Error(`Failed to generate summary report: ${error.message}`);
    }
    throw error;
  }
}

function generateMarkdownReport(
  report: Omit<SummaryReport, 'outputFile'>
): string {
  let md = '# Project Summary Report\n\n';
  md += `**Total Work Units:** ${report.totalWorkUnits}\n\n`;
  md += `**Total Story Points:** ${report.totalStoryPoints}\n\n`;
  md += '## Breakdown by Status\n\n';

  for (const [status, count] of Object.entries(report.byStatus)) {
    md += `- **${status}:** ${count}\n`;
  }

  md += '\n## Velocity Metrics\n\n';
  md += `- **Completed Work Units:** ${report.velocity.completedWorkUnits}\n`;
  md += `- **Completed Story Points:** ${report.velocity.completedPoints}\n`;

  return md;
}

export function registerGenerateSummaryReportCommand(program: Command): void {
  program
    .command('generate-summary-report')
    .description('Generate a comprehensive project summary report')
    .option('--format <format>', 'Output format: markdown or json', 'markdown')
    .option('--output <file>', 'Output file path')
    .action(async (options: { format?: string; output?: string }) => {
      try {
        const result = await generateSummaryReport({
          format: options.format as 'markdown' | 'json',
          output: options.output,
        });
        console.log(chalk.green(`✓ Report generated: ${result.outputFile}`));
      } catch (error: any) {
        console.error(chalk.red('✗ Failed to generate report:'), error.message);
        process.exit(1);
      }
    });
}
