import { existsSync, readFileSync, writeFileSync, mkdirSync } from 'fs';
import { join } from 'path';
import { formatAgentOutput } from '../utils/agentRuntimeConfig.js';

interface ToolsConfig {
  test?: {
    command: string;
  };
  qualityCheck?: {
    commands: string[];
  };
}

interface ConfigFile {
  agent?: string;
  tools?: ToolsConfig;
}

interface ConfigureToolsOptions {
  testCommand?: string;
  qualityCommands?: string[];
  reconfigure?: boolean;
  cwd: string;
}

interface CheckResult {
  type: 'system-reminder';
  message: string;
}

export async function checkTestCommand(cwd: string): Promise<CheckResult> {
  const configPath = join(cwd, 'spec', 'fspec-config.json');

  if (!existsSync(configPath)) {
    const currentYear = new Date().getFullYear();
    return {
      type: 'system-reminder',
      message: formatAgentOutput(
        cwd,
        `NO TEST COMMAND CONFIGURED

No test command configured. Use Read/Glob tools to detect test framework, then run:

  fspec configure-tools --test-command <cmd>

If no test tools detected, search for current best practices:
  Query: "best <platform> testing tools ${currentYear}"

Replace <platform> with detected project type (Node.js, Python, Rust, Go, etc.)

Example:
  fspec configure-tools --test-command "npm test"
  fspec configure-tools --test-command "pytest"
  fspec configure-tools --test-command "cargo test"
`
      ),
    };
  }

  const config: ConfigFile = JSON.parse(readFileSync(configPath, 'utf-8'));

  if (!config.tools?.test?.command) {
    const currentYear = new Date().getFullYear();
    return {
      type: 'system-reminder',
      message: formatAgentOutput(
        cwd,
        `NO TEST COMMAND CONFIGURED

No test command configured. Use Read/Glob tools to detect test framework, then run:

  fspec configure-tools --test-command <cmd>

If no test tools detected, search for current best practices:
  Query: "best <platform> testing tools ${currentYear}"

Replace <platform> with detected project type (Node.js, Python, Rust, Go, etc.)
`
      ),
    };
  }

  return {
    type: 'system-reminder',
    message: formatAgentOutput(
      cwd,
      `RUN TESTS

Run tests: ${config.tools.test.command}
`
    ),
  };
}

export async function checkQualityCommands(cwd: string): Promise<CheckResult> {
  const configPath = join(cwd, 'spec', 'fspec-config.json');

  if (!existsSync(configPath)) {
    return {
      type: 'system-reminder',
      message: formatAgentOutput(cwd, 'No quality check commands configured'),
    };
  }

  const config: ConfigFile = JSON.parse(readFileSync(configPath, 'utf-8'));

  if (!config.tools?.qualityCheck?.commands) {
    return {
      type: 'system-reminder',
      message: formatAgentOutput(cwd, 'No quality check commands configured'),
    };
  }

  const chainedCommand = config.tools.qualityCheck.commands.join(' && ');

  return {
    type: 'system-reminder',
    message: formatAgentOutput(
      cwd,
      `RUN QUALITY CHECKS

Run quality checks: ${chainedCommand}
`
    ),
  };
}

export async function configureTools(
  options: ConfigureToolsOptions
): Promise<CheckResult | void> {
  const { testCommand, qualityCommands, reconfigure, cwd } = options;
  const configPath = join(cwd, 'spec', 'fspec-config.json');
  const specDir = join(cwd, 'spec');

  if (!existsSync(specDir)) {
    mkdirSync(specDir, { recursive: true });
  }

  if (reconfigure) {
    return {
      type: 'system-reminder',
      message: formatAgentOutput(
        cwd,
        `RECONFIGURE TOOLS

Use Read/Glob tools to detect test frameworks and quality check tools, then run:

  fspec configure-tools --test-command <cmd>
  fspec configure-tools --quality-commands '<tool1>' '<tool2>' '<tool3>'
`
      ),
    };
  }

  let config: ConfigFile = { agent: 'claude' };

  if (existsSync(configPath)) {
    config = JSON.parse(readFileSync(configPath, 'utf-8'));
  }

  if (!config.tools) {
    config.tools = {};
  }

  if (testCommand) {
    config.tools.test = { command: testCommand };
  }

  if (qualityCommands) {
    config.tools.qualityCheck = { commands: qualityCommands };
  }

  writeFileSync(configPath, JSON.stringify(config, null, 2), 'utf-8');
}
