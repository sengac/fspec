import chalk from 'chalk';

export interface CommandOption {
  flag: string;
  description: string;
  defaultValue?: string;
}

export interface CommandArgument {
  name: string;
  description: string;
  required: boolean;
}

export interface CommandExample {
  command: string;
  description?: string;
  output?: string;
}

export interface CommonPattern {
  pattern: string;
  example: string;
  description: string;
}

export interface CommandHelpConfig {
  name: string;
  description: string;
  usage?: string;
  arguments?: CommandArgument[];
  options?: CommandOption[];
  examples?: CommandExample[];
  relatedCommands?: string[];
  whenToUse?: string;
  whenNotToUse?: string;
  prerequisites?: string[];
  commonPatterns?: string[] | CommonPattern[];
  typicalWorkflow?: string;
  commonErrors?: Array<{ error: string; fix: string }>;
  notes?: string[];
}

export function formatCommandHelp(config: CommandHelpConfig): string {
  const lines: string[] = [];

  // Header
  lines.push('');
  lines.push(chalk.bold.cyan(`${config.name.toUpperCase()}`));
  lines.push(chalk.dim(config.description));
  lines.push('');

  // When to use (AI-optimized)
  if (config.whenToUse) {
    lines.push(chalk.bold('WHEN TO USE'));
    lines.push(`  ${config.whenToUse}`);
    lines.push('');
  }

  // When NOT to use (AI-optimized)
  if (config.whenNotToUse) {
    lines.push(chalk.bold('WHEN NOT TO USE'));
    lines.push(`  ${config.whenNotToUse}`);
    lines.push('');
  }

  // Prerequisites (AI-optimized)
  if (config.prerequisites && config.prerequisites.length > 0) {
    lines.push(chalk.bold('PREREQUISITES'));
    config.prerequisites.forEach(prereq => {
      lines.push(`  • ${prereq}`);
    });
    lines.push('');
  }

  // Usage
  lines.push(chalk.bold('USAGE'));
  const usageLine = config.usage || `fspec ${config.name}`;
  lines.push(`  ${chalk.cyan(usageLine)}`);
  lines.push('');

  // Arguments
  if (config.arguments && config.arguments.length > 0) {
    lines.push(chalk.bold('ARGUMENTS'));
    config.arguments.forEach(arg => {
      const argName = arg.required ? `<${arg.name}>` : `[${arg.name}]`;
      const required = arg.required
        ? chalk.red('(required)')
        : chalk.dim('(optional)');
      lines.push(`  ${chalk.yellow(argName)} ${required}`);
      lines.push(`    ${arg.description}`);
    });
    lines.push('');
  }

  // Options
  if (config.options && config.options.length > 0) {
    lines.push(chalk.bold('OPTIONS'));
    config.options.forEach(opt => {
      lines.push(`  ${chalk.green(opt.flag)}`);
      lines.push(`    ${opt.description}`);
      if (opt.defaultValue) {
        lines.push(`    ${chalk.dim(`Default: ${opt.defaultValue}`)}`);
      }
    });
    lines.push('');
  } else {
    lines.push(chalk.bold('OPTIONS'));
    lines.push(chalk.dim('  No options available'));
    lines.push('');
  }

  // Common Patterns (AI-optimized)
  if (config.commonPatterns && config.commonPatterns.length > 0) {
    lines.push(chalk.bold('COMMON PATTERNS'));
    config.commonPatterns.forEach(pattern => {
      if (typeof pattern === 'string') {
        // Backward compatibility: string[] format
        lines.push(`  • ${pattern}`);
      } else {
        // Object format: { pattern, example, description }
        lines.push(`  • ${chalk.bold(pattern.pattern)}`);
        lines.push(
          `    ${chalk.dim('Example:')} ${chalk.cyan(pattern.example)}`
        );
        lines.push(`    ${pattern.description}`);
        lines.push('');
      }
    });
    lines.push('');
  }

  // Typical Workflow (AI-optimized)
  if (config.typicalWorkflow) {
    lines.push(chalk.bold('TYPICAL WORKFLOW'));
    lines.push(`  ${config.typicalWorkflow}`);
    lines.push('');
  }

  // Examples
  if (config.examples && config.examples.length > 0) {
    lines.push(chalk.bold('EXAMPLES'));
    config.examples.forEach((example, index) => {
      if (example.description) {
        lines.push(`  ${chalk.dim(`${index + 1}. ${example.description}`)}`);
      }
      lines.push(`  ${chalk.cyan(`$ ${example.command}`)}`);
      if (example.output) {
        lines.push(`  ${chalk.dim(example.output)}`);
      }
      if (index < config.examples!.length - 1) {
        lines.push('');
      }
    });
    lines.push('');
  }

  // Common Errors (AI-optimized)
  if (config.commonErrors && config.commonErrors.length > 0) {
    lines.push(chalk.bold('COMMON ERRORS'));
    config.commonErrors.forEach(err => {
      lines.push(`  ${chalk.red('✗')} ${chalk.bold(err.error)}`);
      lines.push(`    ${chalk.green('Fix:')} ${err.fix}`);
      lines.push('');
    });
  }

  // Related Commands
  if (config.relatedCommands && config.relatedCommands.length > 0) {
    lines.push(chalk.bold('RELATED COMMANDS'));
    config.relatedCommands.forEach(cmd => {
      lines.push(`  ${chalk.cyan(`fspec ${cmd}`)}`);
    });
    lines.push('');
  }

  // Notes
  if (config.notes && config.notes.length > 0) {
    lines.push(chalk.bold('NOTES'));
    config.notes.forEach(note => {
      lines.push(`  • ${note}`);
    });
    lines.push('');
  }

  return lines.join('\n');
}

/**
 * Helper to display help and exit
 */
export function displayHelpAndExit(config: CommandHelpConfig): void {
  console.log(formatCommandHelp(config));
  process.exit(0);
}

/**
 * Research Tool Help Configuration
 * Structured configuration for research tool help display
 */
export interface ResearchToolHelpConfig {
  name: string;
  description: string;
  usage?: string;
  whenToUse?: string;
  whenNotToUse?: string;
  prerequisites?: string[];
  options?: CommandOption[];
  examples?: CommandExample[];
  commonErrors?: Array<{ error: string; fix: string }>;
  exitCodes?: Array<{ code: number; description: string }>;
  configuration?: {
    required: boolean;
    location: string;
    example: string;
    instructions?: string;
  };
  features?: string[];
  notes?: string[];
  customSections?: Array<{ title: string; content: string }>;
}

/**
 * Format research tool help configuration into display string
 * Uses similar structure to formatCommandHelp but optimized for research tools
 */
export function formatResearchToolHelp(config: ResearchToolHelpConfig): string {
  const lines: string[] = [];

  // Header
  lines.push('');
  lines.push(chalk.bold.cyan(`${config.name.toUpperCase()} RESEARCH TOOL`));
  lines.push(chalk.dim(config.description));
  lines.push('');

  // When to use (AI-optimized)
  if (config.whenToUse) {
    lines.push(chalk.bold('WHEN TO USE'));
    lines.push(`  ${config.whenToUse}`);
    lines.push('');
  }

  // When NOT to use (AI-optimized)
  if (config.whenNotToUse) {
    lines.push(chalk.bold('WHEN NOT TO USE'));
    lines.push(`  ${config.whenNotToUse}`);
    lines.push('');
  }

  // Prerequisites (AI-optimized)
  if (config.prerequisites && config.prerequisites.length > 0) {
    lines.push(chalk.bold('PREREQUISITES'));
    config.prerequisites.forEach(prereq => {
      lines.push(`  • ${prereq}`);
    });
    lines.push('');
  }

  // Usage
  lines.push(chalk.bold('USAGE'));
  const usageLine =
    config.usage || `fspec research --tool=${config.name} [options]`;
  lines.push(`  ${chalk.cyan(usageLine)}`);
  lines.push('');

  // Options
  if (config.options && config.options.length > 0) {
    lines.push(chalk.bold('OPTIONS'));
    config.options.forEach(opt => {
      lines.push(`  ${chalk.green(opt.flag)}`);
      lines.push(`    ${opt.description}`);
      if (opt.defaultValue) {
        lines.push(`    ${chalk.dim(`Default: ${opt.defaultValue}`)}`);
      }
    });
    lines.push('');
  }

  // Configuration (research tool specific)
  if (config.configuration) {
    lines.push(chalk.bold('CONFIGURATION'));
    if (config.configuration.required) {
      lines.push(chalk.red('  Required'));
    }
    lines.push(`  ${chalk.dim('Location:')} ${config.configuration.location}`);
    if (config.configuration.instructions) {
      lines.push(`  ${config.configuration.instructions}`);
    }
    if (config.configuration.example) {
      lines.push('');
      lines.push(chalk.dim('  Example:'));
      lines.push(`  ${chalk.dim(config.configuration.example)}`);
    }
    lines.push('');
  }

  // Examples
  if (config.examples && config.examples.length > 0) {
    lines.push(chalk.bold('EXAMPLES'));
    config.examples.forEach((example, index) => {
      if (example.description) {
        lines.push(`  ${chalk.dim(`${index + 1}. ${example.description}`)}`);
      }
      lines.push(
        `  ${chalk.cyan(`$ fspec research --tool=${config.name} ${example.command}`)}`
      );
      if (example.output) {
        lines.push(`  ${chalk.dim(example.output)}`);
      }
      if (index < config.examples!.length - 1) {
        lines.push('');
      }
    });
    lines.push('');
  }

  // Features (research tool specific)
  if (config.features && config.features.length > 0) {
    lines.push(chalk.bold('FEATURES'));
    config.features.forEach(feature => {
      lines.push(`  • ${feature}`);
    });
    lines.push('');
  }

  // Common Errors (AI-optimized)
  if (config.commonErrors && config.commonErrors.length > 0) {
    lines.push(chalk.bold('COMMON ERRORS'));
    config.commonErrors.forEach(err => {
      lines.push(`  ${chalk.red('✗')} ${chalk.bold(err.error)}`);
      lines.push(`    ${chalk.green('Fix:')} ${err.fix}`);
      lines.push('');
    });
  }

  // Exit Codes (research tool specific)
  if (config.exitCodes && config.exitCodes.length > 0) {
    lines.push(chalk.bold('EXIT CODES'));
    config.exitCodes.forEach(exit => {
      const codeColor = exit.code === 0 ? chalk.green : chalk.yellow;
      lines.push(`  ${codeColor(exit.code.toString())}  ${exit.description}`);
    });
    lines.push('');
  }

  // Custom Sections (support for advanced documentation)
  if (config.customSections && config.customSections.length > 0) {
    config.customSections.forEach(section => {
      lines.push(chalk.bold(section.title.toUpperCase()));
      lines.push(`  ${section.content}`);
      lines.push('');
    });
  }

  // Notes
  if (config.notes && config.notes.length > 0) {
    lines.push(chalk.bold('NOTES'));
    config.notes.forEach(note => {
      lines.push(`  • ${note}`);
    });
    lines.push('');
  }

  return lines.join('\n');
}

/**
 * Display research tool help
 * Works for BOTH bundled tools (src/research-tools/) AND custom tools (spec/research-tools/)
 */
export function displayResearchToolHelp(tool: any): void {
  const config = tool.getHelpConfig();
  const helpText = formatResearchToolHelp(config);
  console.log(helpText);
}
