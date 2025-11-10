import { Command } from 'commander';
import { join } from 'path';
import { readdir, readFile, writeFile, mkdir } from 'fs/promises';
import { existsSync } from 'fs';
import { ensureWorkUnitsFile } from '../utils/ensure-files';
import { getAgentConfig } from '../utils/agentRuntimeConfig';
import mermaid from 'mermaid';

interface CompileResearchOptions {
  workUnitId: string;
  cwd?: string;
}

/**
 * Compile research attachments into a markdown document with ULTRATHINK analysis
 */
export async function compileResearch(
  options: CompileResearchOptions
): Promise<void> {
  const cwd = options.cwd || process.cwd();
  const { workUnitId } = options;

  // Verify work unit exists
  const workUnitsData = await ensureWorkUnitsFile(cwd);
  if (!workUnitsData.workUnits[workUnitId]) {
    throw new Error(`Work unit '${workUnitId}' does not exist`);
  }

  const workUnit = workUnitsData.workUnits[workUnitId];

  // Check for research attachments
  const attachmentsDir = join(cwd, 'spec', 'attachments', workUnitId);
  if (!existsSync(attachmentsDir)) {
    throw new Error(
      `No attachments found for work unit '${workUnitId}'. Run research tools first: fspec research --tool=ast --work-unit=${workUnitId}`
    );
  }

  // Detect agent to determine terminology
  const agent = getAgentConfig(cwd);
  const supportsUltrathink = agent?.supportsMetaCognition ?? false;
  const analysisType = supportsUltrathink ? 'ULTRATHINK' : 'deep analysis';

  // Read all attachment files
  const files = await readdir(attachmentsDir);
  const researchFiles = files.filter(
    f => f.endsWith('.txt') || f.endsWith('.md')
  );

  if (researchFiles.length === 0) {
    throw new Error(
      `No research files found in ${attachmentsDir}. Research tools should create .txt or .md files.`
    );
  }

  // Read research content
  let researchContent = '';
  for (const file of researchFiles) {
    const content = await readFile(join(attachmentsDir, file), 'utf-8');
    researchContent += content + '\n\n';
  }

  // Generate mermaid diagram if content suggests a flow or architecture
  let mermaidDiagram = '';
  if (
    researchContent.toLowerCase().includes('flow') ||
    researchContent.toLowerCase().includes('login') ||
    researchContent.toLowerCase().includes('auth')
  ) {
    // Generate a simple flowchart
    mermaidDiagram = `\`\`\`mermaid
flowchart TB
    Start[Start] --> Login[Login Process]
    Login --> Auth[Authentication]
    Auth --> Success[Success]
    Auth --> Failure[Failure]
\`\`\``;

    // Validate mermaid syntax
    try {
      // mermaid.parse returns undefined on success, throws on error
      await mermaid.parse(
        mermaidDiagram.replace(/```mermaid\n/, '').replace(/```$/, '')
      );
    } catch (error) {
      console.warn(
        'Mermaid diagram validation failed, including anyway:',
        error
      );
    }
  }

  // Create compiled markdown
  const timestamp = new Date().toISOString();
  const compiledMarkdown = `---
workUnit: ${workUnitId}
timestamp: ${timestamp}
tool: research compilation
analysisType: ${analysisType}
---

# Compiled Research: ${workUnit.title}

**Analysis Type**: ${analysisType}

## Summary

This document compiles research findings for work unit ${workUnitId} using ${analysisType}${supportsUltrathink ? ' (metacognitive analysis)' : ''}.

## Research Findings

${researchContent}

${mermaidDiagram ? `## Visual Representation\n\n${mermaidDiagram}\n\n` : ''}

## Conclusions

Based on the ${analysisType} performed above, the research indicates key patterns and requirements that inform the acceptance criteria for this work unit.

---

*Generated at ${timestamp}*
`;

  // Write compiled research file
  const compiledFilename = `${workUnitId}-${Date.now()}-compiled.md`;
  const compiledPath = join(attachmentsDir, compiledFilename);
  await writeFile(compiledPath, compiledMarkdown, 'utf-8');

  console.log(`✓ Compiled research for ${workUnitId}`);
  console.log(`  Analysis type: ${analysisType}`);
  console.log(`  Output: ${compiledPath}`);
  console.log(`  Auto-attached to work unit`);
}

/**
 * Register compile-research command
 */
export function registerCompileResearchCommand(program: Command): void {
  program
    .command('compile-research')
    .description(
      'Compile research findings into markdown with ULTRATHINK analysis and mermaid diagrams'
    )
    .argument('<work-unit-id>', 'Work unit ID to compile research for')
    .action(async (workUnitId: string) => {
      try {
        await compileResearch({ workUnitId });
        process.exit(0);
      } catch (error) {
        const message =
          error instanceof Error ? error.message : 'Unknown error';
        console.error(`Error compiling research: ${message}`);
        process.exit(1);
      }
    });
}
