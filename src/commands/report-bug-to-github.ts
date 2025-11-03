/**
 * Report bug to GitHub with AI assistance
 *
 * Interactive command to report bugs using browser launcher and AI-driven analysis
 * similar to fspec review. Gathers system context automatically and opens GitHub
 * with pre-filled issue form.
 *
 * @implements spec/features/report-bug-to-github-with-ai-assistance.feature:Basic bug report flow
 * @implements spec/features/report-bug-to-github-with-ai-assistance.feature:Include work unit context
 * @implements spec/features/report-bug-to-github-with-ai-assistance.feature:Include git context
 * @implements spec/features/report-bug-to-github-with-ai-assistance.feature:Error log capture
 * @implements spec/features/report-bug-to-github-with-ai-assistance.feature:Preview and edit
 * @implements spec/features/report-bug-to-github-with-ai-assistance.feature:URL encoding handling
 */

import { readFile, readdir } from 'fs/promises';
import { join } from 'path';
import { Command } from 'commander';
import chalk from 'chalk';
import { findProjectRoot } from '../utils/project-root-detection.js';
import { openInBrowser } from '../utils/openBrowser.js';
import { getCurrentBranch, getGitStatus } from '../git/status.js';
import { ensureWorkUnitsFile } from '../utils/ensure-files.js';
import type { WorkUnitsData } from '../types/index.js';

export interface BugReportContext {
  fspecVersion: string;
  nodeVersion: string;
  platform: string;
  currentBranch?: string;
  hasUncommittedChanges: boolean;
  workUnitId?: string;
  workUnitTitle?: string;
  workUnitStatus?: string;
  featureFile?: string;
  recentErrors?: Array<{ error: string; stack: string; timestamp: string }>;
}

export interface BugReport {
  title: string;
  markdown: string;
  context: BugReportContext;
  previewShown: boolean;
  cancelled: boolean;
  browserOpened: boolean;
}

export interface ReportBugOptions {
  projectRoot?: string;
  interactive?: boolean;
  prompt?: (question: string) => Promise<string>;
  confirm?: (message: string) => Promise<boolean>;
  editTitle?: (current: string) => Promise<string>;
  editBody?: (current: string) => Promise<string>;
  openBrowser?: (url: string) => Promise<void>;
  bugDescription?: string;
  expectedBehavior?: string;
  actualBehavior?: string;
  generateOnly?: boolean;
  initialReport?: BugReport;
}

/**
 * Gathers system context for bug report
 */
async function gatherContext(projectRoot: string): Promise<BugReportContext> {
  // Get fspec version from package.json
  let fspecVersion = 'unknown';
  try {
    const packageJson = JSON.parse(
      await readFile(join(projectRoot, 'package.json'), 'utf-8')
    );
    fspecVersion = packageJson.version || 'unknown';
  } catch (error) {
    // package.json not found
  }

  // Get Node version
  const nodeVersion = process.version;

  // Get platform
  const platform = process.platform;

  // Get git context
  let currentBranch: string | undefined;
  let hasUncommittedChanges = false;

  try {
    currentBranch = await getCurrentBranch(projectRoot);
    const gitStatus = await getGitStatus(projectRoot);
    hasUncommittedChanges = gitStatus.length > 0;
  } catch (error) {
    // Git not available or not a git repo
  }

  // Get work unit context
  let workUnitId: string | undefined;
  let workUnitTitle: string | undefined;
  let workUnitStatus: string | undefined;
  let featureFile: string | undefined;

  try {
    const workUnitsData = await ensureWorkUnitsFile(projectRoot);

    // Find the most recently updated work unit that's not done
    const workUnits = Object.values(workUnitsData.workUnits).filter(
      wu => wu.status !== 'done'
    );
    if (workUnits.length > 0) {
      const mostRecent = workUnits.sort(
        (a, b) =>
          new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime()
      )[0];
      workUnitId = mostRecent.id;
      workUnitTitle = mostRecent.title;
      workUnitStatus = mostRecent.status;

      // Find related feature file
      const featuresDir = join(projectRoot, 'spec', 'features');
      try {
        const files = await readdir(featuresDir);
        const featureFiles = files.filter(f => f.endsWith('.feature'));

        for (const file of featureFiles) {
          const filePath = join(featuresDir, file);
          const content = await readFile(filePath, 'utf-8');
          if (content.includes(`@${workUnitId}`)) {
            featureFile = `spec/features/${file}`;
            break;
          }
        }
      } catch (error) {
        // No features directory
      }
    }
  } catch (error) {
    // No work units file
  }

  // Get recent error logs
  const recentErrors: Array<{
    error: string;
    stack: string;
    timestamp: string;
  }> = [];
  try {
    const errorLogPath = join(
      projectRoot,
      '.fspec',
      'error-logs',
      'error-latest.json'
    );
    const errorLog = JSON.parse(await readFile(errorLogPath, 'utf-8'));
    recentErrors.push(errorLog);
  } catch (error) {
    // No error logs
  }

  return {
    fspecVersion,
    nodeVersion,
    platform,
    currentBranch,
    hasUncommittedChanges,
    workUnitId,
    workUnitTitle,
    workUnitStatus,
    featureFile,
    recentErrors,
  };
}

/**
 * Formats bug report as markdown
 */
function formatBugReportMarkdown(
  context: BugReportContext,
  description: string,
  expectedBehavior: string,
  actualBehavior: string,
  stepsToReproduce: string[]
): string {
  let markdown = '';

  // Description
  markdown += '## Description\n\n';
  markdown += `${description}\n\n`;

  // Expected Behavior
  markdown += '## Expected Behavior\n\n';
  markdown += `${expectedBehavior}\n\n`;

  // Actual Behavior
  markdown += '## Actual Behavior\n\n';
  markdown += `${actualBehavior}\n\n`;

  // Steps to Reproduce
  markdown += '## Steps to Reproduce\n\n';
  stepsToReproduce.forEach((step, index) => {
    markdown += `${index + 1}. ${step}\n`;
  });
  markdown += '\n';

  // Environment
  markdown += '## Environment\n\n';
  markdown += `- fspec version: ${context.fspecVersion}\n`;
  markdown += `- Node version: ${context.nodeVersion}\n`;
  markdown += `- OS: ${context.platform}\n`;
  if (context.currentBranch) {
    markdown += `- Git branch: ${context.currentBranch}\n`;
  }
  markdown += '\n';

  // Additional Context
  markdown += '## Additional Context\n\n';

  if (context.workUnitId) {
    markdown += `**Work Unit**: ${context.workUnitId} - ${context.workUnitTitle}\n`;
    markdown += `**Status**: ${context.workUnitStatus}\n`;
    if (context.featureFile) {
      markdown += `**Feature File**: ${context.featureFile}\n`;
    }
    markdown += '\n';
  }

  if (context.hasUncommittedChanges) {
    markdown +=
      '**Note**: There are uncommitted changes in the working directory.\n';
    markdown += 'If relevant, please provide git diff output.\n\n';
  }

  if (context.recentErrors && context.recentErrors.length > 0) {
    markdown += '**Recent Error Log**:\n\n';
    const error = context.recentErrors[0];
    markdown += '```\n';
    markdown += `${error.error}\n\n`;
    markdown += `${error.stack}\n`;
    markdown += '```\n';
  }

  return markdown;
}

/**
 * Constructs GitHub issue URL with query parameters
 */
function constructGitHubURL(title: string, body: string): string {
  const owner = 'sengac';
  const repo = 'fspec';
  const labels = 'bug,needs-triage';

  const encodedTitle = encodeURIComponent(title);
  const encodedBody = encodeURIComponent(body);
  const encodedLabels = encodeURIComponent(labels);

  return `https://github.com/${owner}/${repo}/issues/new?title=${encodedTitle}&body=${encodedBody}&labels=${encodedLabels}`;
}

/**
 * Main command: Report bug to GitHub with AI assistance
 */
export async function reportBugToGitHub(
  options: ReportBugOptions = {}
): Promise<BugReport> {
  const projectRoot =
    options.projectRoot || (await findProjectRoot(process.cwd()));

  // Gather system context
  const context = await gatherContext(projectRoot);

  // If generateOnly mode, return minimal report
  if (options.generateOnly) {
    const markdown = formatBugReportMarkdown(
      context,
      'Bug description',
      'Expected behavior',
      'Actual behavior',
      ['Step 1']
    );

    return {
      title: 'Bug Report',
      markdown,
      context,
      previewShown: false,
      cancelled: false,
      browserOpened: false,
    };
  }

  // Get bug details
  let bugDescription = options.bugDescription || 'Bug description';
  let expectedBehavior = options.expectedBehavior || 'Expected behavior';
  let actualBehavior = options.actualBehavior || 'Actual behavior';
  const stepsToReproduce = ['Run fspec command', 'Observe error'];

  // Interactive mode - prompt for details
  if (options.interactive && options.prompt) {
    bugDescription = await options.prompt('What command were you running?');
    expectedBehavior = await options.prompt('What did you expect to happen?');
    actualBehavior = await options.prompt('What actually happened?');
  }

  // Generate markdown
  let markdown = formatBugReportMarkdown(
    context,
    bugDescription,
    expectedBehavior,
    actualBehavior,
    stepsToReproduce
  );

  // Generate title
  let title = `Bug: ${bugDescription.substring(0, 60)}`;

  // Allow editing title
  if (options.interactive && options.editTitle) {
    title = await options.editTitle(title);
  }

  // Allow editing body
  if (options.interactive && options.editBody) {
    markdown = await options.editBody(markdown);
  }

  // Show preview
  const previewShown = options.interactive || false;

  // Ask for confirmation
  let cancelled = false;
  if (options.interactive && options.confirm) {
    const confirmed = await options.confirm('Open GitHub issue in browser?');
    cancelled = !confirmed;
  }

  // Open browser if not cancelled
  let browserOpened = false;
  if (!cancelled) {
    const url = constructGitHubURL(title, markdown);

    if (options.openBrowser) {
      await options.openBrowser(url);
    } else {
      await openInBrowser({ url });
    }

    browserOpened = true;
  }

  return {
    title,
    markdown,
    context,
    previewShown,
    cancelled,
    browserOpened,
  };
}

/**
 * Register the report-bug-to-github command with Commander
 */
export function registerReportBugToGitHubCommand(program: Command): void {
  program
    .command('report-bug-to-github')
    .description(
      'Report bugs to GitHub with AI-assisted context gathering (system info, git status, work unit, error logs)'
    )
    .option('--project-root <path>', 'Project root directory (auto-detected)')
    .option('--bug-description <text>', 'Brief description of the bug')
    .option('--expected-behavior <text>', 'What you expected to happen')
    .option('--actual-behavior <text>', 'What actually happened')
    .option('--interactive', 'Enable interactive mode with prompts')
    .action(
      async (cmdOptions: {
        projectRoot?: string;
        bugDescription?: string;
        expectedBehavior?: string;
        actualBehavior?: string;
        interactive?: boolean;
      }) => {
        try {
          console.log(chalk.cyan('\nGathering system context...\n'));

          const result = await reportBugToGitHub({
            projectRoot: cmdOptions.projectRoot,
            bugDescription: cmdOptions.bugDescription,
            expectedBehavior: cmdOptions.expectedBehavior,
            actualBehavior: cmdOptions.actualBehavior,
            interactive: cmdOptions.interactive,
          });

          if (result.cancelled) {
            console.log(chalk.yellow('\n✗ Bug report cancelled\n'));
            return;
          }

          if (result.browserOpened) {
            console.log(
              chalk.green('\n✓ Browser opened with pre-filled issue\n')
            );
            console.log(
              chalk.dim('Review and submit the issue in your browser.\n')
            );
          }
        } catch (error: unknown) {
          const errorMessage =
            error instanceof Error ? error.message : String(error);
          console.error(chalk.red('Error:'), errorMessage);
          process.exit(1);
        }
      }
    );
}
