# Feature Specification: Report Bug to GitHub (fspec report-bug-to-github)

## Overview

A new interactive CLI command that helps users report bugs to the fspec GitHub repository using an AI-driven analysis process similar to `fspec review`, combined with a browser launcher to create GitHub issues via URL query parameters.

## Command Signature

```bash
fspec report-bug-to-github [options]
```

## User Story

**As a** developer using fspec who encounters a bug
**I want to** report it to GitHub through an interactive guided process
**So that** I can provide comprehensive bug information without manually writing issue reports

## Architecture Notes

### Browser Launcher Integration

- **Source**: Shared utility from `~/projects/cage/packages/cli/src/shared/utils/openBrowser.ts`
- **Function**: `openInBrowser({ url, wait })`
- **Package**: Uses `open` npm package
- **Copy Strategy**: Create `src/utils/openBrowser.ts` that mirrors cage's implementation
- **Dependencies**: Add `open` package to fspec's package.json

### GitHub Issue URL Format

Based on https://docs.github.com/en/issues/tracking-your-work-with-issues/using-issues/creating-an-issue#creating-an-issue-from-a-url-query

```
https://github.com/{owner}/{repo}/issues/new?title={title}&body={body}&labels={labels}
```

**Available Query Parameters:**

- `title` - Issue title (URL encoded)
- `body` - Issue body with markdown (URL encoded)
- `labels` - Comma-separated label list (e.g., "bug,needs-triage")
- `assignees` - Comma-separated GitHub usernames
- `milestone` - Milestone name
- `projects` - Project name
- `template` - Issue template name (if using GitHub issue templates)

### Interactive Process (Similar to fspec review)

The command follows the `fspec review` pattern:

1. **Gather Context**
   - Read recent error logs (if available)
   - Capture current work unit context (if in a work unit)
   - Capture git status (uncommitted changes, current branch)
   - Read package.json version
   - Capture OS/platform info (process.platform, process.version)

2. **AI-Driven Analysis**
   - Present context to AI (via system-reminder)
   - AI asks clarifying questions interactively
   - AI identifies:
     - Bug description (what happened vs what was expected)
     - Steps to reproduce
     - Impact/severity
     - Relevant code files
     - Potential workarounds

3. **Generate Issue Content**
   - Title: Concise bug summary (AI-generated)
   - Body: Markdown formatted with sections:
     - **Description**: What happened
     - **Expected Behavior**: What should happen
     - **Actual Behavior**: What actually happened
     - **Steps to Reproduce**: Numbered list
     - **Environment**:
       - fspec version
       - Node version
       - OS (macOS/Linux/Windows)
       - Git branch (if applicable)
     - **Additional Context**: Relevant code snippets, logs, work unit ID

4. **Preview and Confirm**
   - Display formatted issue to user
   - Ask for confirmation before opening browser
   - Allow editing before submission

5. **Open Browser**
   - Construct GitHub URL with query parameters
   - URL-encode title and body
   - Add `bug,needs-triage` labels by default
   - Open browser with `openInBrowser({ url })`
   - User completes submission in GitHub

## Acceptance Criteria

### Scenario 1: Basic Bug Report Flow

**Given** I am in a project using fspec
**And** I have encountered a bug
**When** I run `fspec report-bug-to-github`
**Then** the command should:

- Gather system context automatically
- Prompt me interactively for bug details
- Generate a complete bug report with markdown formatting
- Display preview of the issue
- Ask for confirmation
- Open my browser to GitHub with pre-filled issue

### Scenario 2: Include Work Unit Context

**Given** I am working on work unit AUTH-001
**And** the bug occurs while implementing AUTH-001
**When** I run `fspec report-bug-to-github`
**Then** the bug report should include:

- Work unit ID (AUTH-001)
- Work unit title
- Current work unit status
- Link to related feature file (if exists)

### Scenario 3: Include Git Context

**Given** I have uncommitted changes
**And** the bug reproduces with these changes
**When** I run `fspec report-bug-to-github`
**Then** the bug report should include:

- Current git branch
- Whether there are uncommitted changes
- Note about providing git diff if needed

### Scenario 4: Error Log Capture

**Given** fspec recently crashed with an error
**When** I run `fspec report-bug-to-github`
**Then** the command should:

- Detect recent error logs
- Include stack trace (if available)
- Include error message in bug report

### Scenario 5: Preview and Edit

**Given** the AI has generated a bug report
**When** I review the preview
**And** I want to add additional context
**Then** I should be able to:

- Edit the title before submission
- Edit the body before submission
- Cancel the submission
- Confirm and open browser

### Scenario 6: URL Encoding Handling

**Given** the bug report contains special characters
**And** the report contains markdown code blocks
**When** constructing the GitHub URL
**Then** all content should be properly URL-encoded
**And** markdown formatting should be preserved
**And** code blocks should render correctly in GitHub

## Implementation Details

### File Structure

```
src/
├── commands/
│   ├── report-bug-to-github.ts          # Main command implementation
│   └── report-bug-to-github-help.ts     # Help documentation
├── utils/
│   ├── openBrowser.ts                   # Browser launcher (from cage)
│   └── bugReportAnalysis.ts             # AI analysis logic
└── __tests__/
    └── commands/
        └── report-bug-to-github.test.ts # Command tests
```

### Dependencies to Add

```json
{
  "dependencies": {
    "open": "^10.0.0" // Browser launcher
  }
}
```

### Key Functions

```typescript
// src/commands/report-bug-to-github.ts

interface BugReportContext {
  fspecVersion: string;
  nodeVersion: string;
  platform: string;
  currentBranch?: string;
  hasUncommittedChanges: boolean;
  workUnitId?: string;
  workUnitTitle?: string;
  recentErrors?: string[];
}

interface BugReport {
  title: string;
  description: string;
  expectedBehavior: string;
  actualBehavior: string;
  stepsToReproduce: string[];
  environment: {
    fspecVersion: string;
    nodeVersion: string;
    os: string;
    gitBranch?: string;
  };
  additionalContext?: string;
}

async function gatherContext(): Promise<BugReportContext> {
  // Collect system info, git status, work unit context
}

function buildAIAnalysisReminder(context: BugReportContext): string {
  // Similar to review.ts buildAIAnalysisReminder
  // Guide AI to ask questions and generate bug report
}

async function generateBugReport(
  context: BugReportContext
): Promise<BugReport> {
  // AI-driven interactive analysis
}

function formatBugReportMarkdown(report: BugReport): string {
  // Convert BugReport to markdown
}

async function openGitHubIssue(report: BugReport): Promise<void> {
  const title = encodeURIComponent(report.title);
  const body = encodeURIComponent(formatBugReportMarkdown(report));
  const labels = 'bug,needs-triage';

  const url = `https://github.com/sengac/fspec/issues/new?title=${title}&body=${body}&labels=${labels}`;

  await openInBrowser({ url });
}
```

### AI Analysis System-Reminder Structure

```typescript
const systemReminder = `
AI-DRIVEN BUG REPORT GENERATION

Context Information:
- fspec version: ${context.fspecVersion}
- Node version: ${context.nodeVersion}
- Platform: ${context.platform}
${context.workUnitId ? `- Working on: ${context.workUnitId} - ${context.workUnitTitle}` : ''}
${context.hasUncommittedChanges ? '- Has uncommitted changes' : ''}

STEP 1: Ask Clarifying Questions

Ask the user:
1. What command were you running when the bug occurred?
2. What did you expect to happen?
3. What actually happened?
4. Can you provide the exact error message (if any)?
5. Can you reproduce the bug consistently?

STEP 2: Generate Bug Report

Based on the user's answers, create a structured bug report with:
- Title: Concise summary (max 80 chars)
- Description: Clear explanation of the bug
- Expected Behavior: What should happen
- Actual Behavior: What actually happens
- Steps to Reproduce: Numbered list of exact steps
- Additional Context: Any relevant details

STEP 3: Format Output

Format the bug report as markdown with these sections:
## Description
## Expected Behavior
## Actual Behavior
## Steps to Reproduce
## Environment
## Additional Context

Be concise but thorough. Focus on actionable information.
`;
```

## Help Documentation

The command requires comprehensive `--help` documentation following fspec patterns.

### Help File: src/commands/report-bug-to-github-help.ts

```typescript
export const reportBugToGitHubHelp = {
  name: 'report-bug-to-github',
  description:
    'Report a bug to the fspec GitHub repository with AI-assisted analysis',

  usage: `
    fspec report-bug-to-github
  `,

  whenToUse: `
    Use this command when:
    - You encounter a bug or unexpected behavior in fspec
    - You want to report an issue to the maintainers
    - You need help structuring a comprehensive bug report
    - You want to include relevant context automatically
  `,

  prerequisites: `
    - fspec initialized in your project
    - Internet connection (to open GitHub)
    - Browser installed
  `,

  typicalWorkflow: `
    1. Encounter a bug while using fspec
    2. Run: fspec report-bug-to-github
    3. Answer AI's clarifying questions
    4. Review generated bug report preview
    5. Confirm to open browser
    6. Submit issue on GitHub
  `,

  commonPatterns: [
    {
      title: 'Report Command Crash',
      commands: [
        'fspec report-bug-to-github',
        '# Answer: Running "fspec validate" caused crash',
        '# Answer: Expected validation to complete',
        '# Answer: Process exited with ENOENT error',
        '# Confirm preview, browser opens',
      ],
    },
    {
      title: 'Report Incorrect Behavior',
      commands: [
        'fspec report-bug-to-github',
        '# Answer: "fspec list-work-units --status=testing" shows backlog items',
        '# Answer: Should only show items in testing status',
        '# Answer: Shows all work units instead',
        '# Confirm preview, browser opens',
      ],
    },
  ],

  examples: [
    {
      title: 'Basic Bug Report',
      code: `fspec report-bug-to-github`,
      explanation: 'Starts interactive bug reporting process',
    },
  ],

  relatedCommands: [
    'fspec review - Review work unit with AI analysis',
    'fspec check - Run validation checks',
  ],

  notes: [
    'The command automatically gathers system context (version, OS, git status)',
    'Work unit context is included if you are currently working on a work unit',
    'Bug report is AI-assisted but requires human confirmation',
    'Browser must be installed and accessible',
    'Internet connection required to open GitHub',
    'GitHub account required to submit issue',
  ],
};
```

## Auto-Generated Content

The command needs to be registered in the CLI:

### Updates Required

1. **src/index.ts** - Register command:

```typescript
import { registerReportBugToGitHubCommand } from './commands/report-bug-to-github';

// In program setup
registerReportBugToGitHubCommand(program);
```

2. **src/commands/report-bug-to-github.ts** - Export register function:

```typescript
export function registerReportBugToGitHubCommand(program: Command): void {
  program
    .command('report-bug-to-github')
    .description('Report a bug to the fspec GitHub repository')
    .action(async () => {
      try {
        await reportBugToGitHub();
      } catch (error) {
        console.error(chalk.red('Error:'), error.message);
        process.exit(1);
      }
    });
}
```

3. **package.json** - Add dependency:

```json
{
  "dependencies": {
    "open": "^10.0.0"
  }
}
```

## Documentation Updates

1. **README.md** - Add command to CLI reference:

```markdown
### Bug Reporting

- `fspec report-bug-to-github` - Report a bug to GitHub with AI assistance
```

2. **spec/CLAUDE.md** - Add to command list:

````markdown
## Reporting Issues

When you encounter a bug:

```bash
fspec report-bug-to-github
```
````

This command provides an interactive AI-assisted process to create comprehensive bug reports.

````

## Testing Strategy

### Unit Tests

```typescript
describe('Feature: Report Bug to GitHub', () => {
  describe('Scenario: Gather system context', () => {
    it('should collect fspec version, node version, and platform', async () => {
      const context = await gatherContext();
      expect(context.fspecVersion).toBeDefined();
      expect(context.nodeVersion).toBeDefined();
      expect(context.platform).toBeDefined();
    });
  });

  describe('Scenario: Format bug report markdown', () => {
    it('should generate properly formatted markdown', () => {
      const report = {
        title: 'Test Bug',
        description: 'Bug description',
        expectedBehavior: 'Expected',
        actualBehavior: 'Actual',
        stepsToReproduce: ['Step 1', 'Step 2'],
        environment: {
          fspecVersion: '1.0.0',
          nodeVersion: 'v18.0.0',
          os: 'macOS'
        }
      };

      const markdown = formatBugReportMarkdown(report);
      expect(markdown).toContain('## Description');
      expect(markdown).toContain('## Steps to Reproduce');
      expect(markdown).toContain('fspec version: 1.0.0');
    });
  });

  describe('Scenario: URL encoding', () => {
    it('should properly encode special characters', () => {
      const title = 'Bug: Command fails with "error"';
      const encoded = encodeURIComponent(title);
      expect(encoded).toContain('%22');
      expect(encoded).not.toContain('"');
    });
  });
});
````

### Integration Tests

- Test browser launcher integration
- Test GitHub URL construction
- Test AI system-reminder generation

## Dependencies on Other Features

### Browser Launcher Utility Extraction

**Work Unit**: Should be created as separate work unit
**Title**: "Extract shared browser launcher utility from cage"
**Epic**: Shared utilities
**Description**: Copy and adapt `openBrowser.ts` from cage to fspec

This is a dependency that should be completed BEFORE report-bug-to-github.

## Open Questions

1. **GitHub Repository URL**: Hardcode to `anthropics/fspec` or make configurable?
   - **Recommendation**: Hardcode for simplicity

2. **Error Log Persistence**: Where/how to store recent error logs?
   - **Recommendation**: Use `.fspec/error-logs/` directory with rotating logs

3. **Offline Behavior**: What if no internet connection?
   - **Recommendation**: Generate markdown file, instruct user to copy/paste

4. **Template Customization**: Allow custom bug report templates?
   - **Recommendation**: Not in initial version, add later if needed

## Success Metrics

- Users can report bugs without leaving terminal
- Bug reports include comprehensive context automatically
- AI assists in structuring clear, actionable reports
- Reduces time to create bug reports from 10+ minutes to <3 minutes

## Future Enhancements

1. **Offline Mode**: Generate markdown file if no internet
2. **Attachment Support**: Upload screenshots/logs via GitHub API
3. **Template Selection**: Choose from multiple issue templates
4. **Draft Saving**: Save draft reports for later submission
5. **Similar Issue Detection**: Check for existing similar issues before creating
