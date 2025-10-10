import { readFile, writeFile, mkdir } from 'fs/promises';
import { join, dirname } from 'path';
import type { WorkUnitsData } from '../types';

interface ExportDependenciesOptions {
  format: 'mermaid' | 'json';
  output: string;
  cwd?: string;
}

interface ExportDependenciesResult {
  success: boolean;
  outputFile?: string;
}

function generateMermaidDiagram(data: WorkUnitsData): string {
  const lines: string[] = ['graph TB'];

  // Add all work units as nodes
  for (const [id, workUnit] of Object.entries(data.workUnits)) {
    const statusClass = workUnit.status === 'done' ? ':::done' : workUnit.status === 'blocked' ? ':::blocked' : '';
    lines.push(`  ${id}["${workUnit.title || id}"]${statusClass}`);
  }

  // Add edges for dependencies
  const addedEdges = new Set<string>();

  for (const [id, workUnit] of Object.entries(data.workUnits)) {
    // blocks relationships (solid red line)
    if (workUnit.blocks) {
      for (const targetId of workUnit.blocks) {
        const edgeKey = `${id}-blocks-${targetId}`;
        if (!addedEdges.has(edgeKey)) {
          lines.push(`  ${id} -->|blocks| ${targetId}`);
          addedEdges.add(edgeKey);
        }
      }
    }

    // dependsOn relationships (dashed line)
    if (workUnit.dependsOn) {
      for (const targetId of workUnit.dependsOn) {
        const edgeKey = `${id}-dependsOn-${targetId}`;
        if (!addedEdges.has(edgeKey)) {
          lines.push(`  ${id} -.->|depends on| ${targetId}`);
          addedEdges.add(edgeKey);
        }
      }
    }

    // relatesTo relationships (dotted line)
    if (workUnit.relatesTo) {
      for (const targetId of workUnit.relatesTo) {
        // Only add if we haven't added the reverse
        const edgeKey = `${id}-relatesTo-${targetId}`;
        const reverseKey = `${targetId}-relatesTo-${id}`;
        if (!addedEdges.has(edgeKey) && !addedEdges.has(reverseKey)) {
          lines.push(`  ${id} <-.->|relates to| ${targetId}`);
          addedEdges.add(edgeKey);
        }
      }
    }
  }

  // Add style classes
  lines.push('');
  lines.push('  classDef done fill:#90EE90');
  lines.push('  classDef blocked fill:#FFB6C1');

  return lines.join('\n');
}

export async function exportDependencies(
  options: ExportDependenciesOptions
): Promise<ExportDependenciesResult> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec/work-units.json');

  const content = await readFile(workUnitsFile, 'utf-8');
  const data: WorkUnitsData = JSON.parse(content);

  let outputContent: string;

  if (options.format === 'mermaid') {
    outputContent = generateMermaidDiagram(data);
  } else {
    // JSON format
    const dependencies: Record<string, any> = {};
    for (const [id, workUnit] of Object.entries(data.workUnits)) {
      dependencies[id] = {
        blocks: workUnit.blocks || [],
        blockedBy: workUnit.blockedBy || [],
        dependsOn: workUnit.dependsOn || [],
        relatesTo: workUnit.relatesTo || [],
      };
    }
    outputContent = JSON.stringify(dependencies, null, 2);
  }

  // Ensure output directory exists
  await mkdir(dirname(options.output), { recursive: true });

  await writeFile(options.output, outputContent);

  return {
    success: true,
    outputFile: options.output,
  };
}
