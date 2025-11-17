import { validateMermaidSyntax } from '../utils/mermaid-validation';
import type { GenericFoundation } from '../types/generic-foundation';
import type { EventStormBoundedContext } from '../types';

/**
 * Generate Mermaid diagram for bounded contexts with relationships
 * Uses neutral styling (no colors) for dark/light mode compatibility
 */
function generateBoundedContextMermaid(
  boundedContexts: EventStormBoundedContext[]
): string {
  const lines: string[] = [];
  lines.push('graph TB');

  // Create map of context names to IDs for relationship mapping
  const contextMap = new Map<string, string>();
  for (let i = 0; i < boundedContexts.length; i++) {
    const context = boundedContexts[i];
    const nodeId = `BC${i + 1}`;
    contextMap.set(context.text, nodeId);
  }

  // Generate nodes for each bounded context with brief description
  for (let i = 0; i < boundedContexts.length; i++) {
    const context = boundedContexts[i];
    const nodeId = `BC${i + 1}`;

    // Add brief description based on context
    let description = '';
    switch (context.text) {
      case 'Work Management':
        description = 'Stories, Epics, Dependencies';
        break;
      case 'Specification':
        description = 'Features, Scenarios, Steps';
        break;
      case 'Discovery':
        description = 'Rules, Examples, Questions';
        break;
      case 'Event Storming':
        description = 'Events, Commands, Policies';
        break;
      case 'Foundation':
        description = 'Vision, Capabilities, Personas';
        break;
      case 'Testing & Validation':
        description = 'Coverage, Test Mappings';
        break;
      default:
        description = context.text;
    }

    lines.push(`  ${nodeId}["${context.text}<br/>${description}"]`);
  }

  lines.push('');

  // Add relationships between contexts
  const relationships = [
    { from: 'Discovery', to: 'Specification', label: 'generates' },
    { from: 'Work Management', to: 'Specification', label: 'links to' },
    { from: 'Specification', to: 'Testing & Validation', label: 'tracked by' },
    { from: 'Event Storming', to: 'Foundation', label: 'populates' },
    { from: 'Foundation', to: 'Discovery', label: 'guides' },
  ];

  for (const rel of relationships) {
    const fromId = contextMap.get(rel.from);
    const toId = contextMap.get(rel.to);
    if (fromId && toId) {
      lines.push(`  ${fromId} -->|${rel.label}| ${toId}`);
    }
  }

  return lines.join('\n');
}

/**
 * Generate Event Flow diagram for a bounded context
 * Shows Commands → Aggregates → Events pattern
 * Uses neutral styling (no colors) for dark/light mode compatibility
 */
function generateContextEventFlowMermaid(
  contextId: number,
  foundation: GenericFoundation
): string {
  const lines: string[] = [];
  lines.push('flowchart TB');

  // Extract commands, aggregates, and events for this context
  const commands = foundation.eventStorm.items.filter(
    (item): item is any =>
      item.type === 'command' &&
      !item.deleted &&
      'boundedContextId' in item &&
      item.boundedContextId === contextId
  );

  const aggregates = foundation.eventStorm.items.filter(
    (item): item is any =>
      item.type === 'aggregate' &&
      !item.deleted &&
      'boundedContextId' in item &&
      item.boundedContextId === contextId
  );

  const events = foundation.eventStorm.items.filter(
    (item): item is any =>
      item.type === 'event' &&
      !item.deleted &&
      'boundedContextId' in item &&
      item.boundedContextId === contextId
  );

  // Only generate diagram if there are items to display
  if (commands.length === 0 && aggregates.length === 0 && events.length === 0) {
    return '';
  }

  // Commands subgraph
  if (commands.length > 0) {
    lines.push('  subgraph Commands["⚡ Commands"]');
    for (const cmd of commands) {
      const nodeId = `C${cmd.id}`;
      lines.push(`    ${nodeId}[${cmd.text}]`);
    }
    lines.push('  end');
    lines.push('');
  }

  // Aggregates subgraph
  if (aggregates.length > 0) {
    lines.push('  subgraph Aggregates["📦 Aggregates"]');
    for (const agg of aggregates) {
      const nodeId = `A${agg.id}`;
      lines.push(`    ${nodeId}[${agg.text}]`);
    }
    lines.push('  end');
    lines.push('');
  }

  // Events subgraph
  if (events.length > 0) {
    lines.push('  subgraph Events["📢 Events"]');
    for (const evt of events) {
      const nodeId = `E${evt.id}`;
      lines.push(`    ${nodeId}[${evt.text}]`);
    }
    lines.push('  end');
    lines.push('');
  }

  // Add flow arrows
  if (commands.length > 0 && aggregates.length > 0) {
    lines.push('  Commands -.-> Aggregates');
  }
  if (aggregates.length > 0 && events.length > 0) {
    lines.push('  Aggregates -.-> Events');
  }

  return lines.join('\n');
}

/**
 * Generate FOUNDATION.md from foundation.json (Generic Schema v2.0.0)
 * This is a template-based markdown generator for the generic foundation schema
 */
export async function generateFoundationMd(
  foundation: GenericFoundation
): Promise<string> {
  const sections: string[] = [];

  // Header with auto-generation warning
  sections.push(
    '<!-- THIS FILE IS AUTO-GENERATED FROM spec/foundation.json -->'
  );
  sections.push('<!-- DO NOT EDIT THIS FILE DIRECTLY -->');
  sections.push(
    '<!-- Edit spec/foundation.json and run: fspec generate-foundation-md -->'
  );
  sections.push('');

  // Project header
  if (foundation.project && foundation.project.name) {
    sections.push(`# ${foundation.project.name} Project Foundation`);
    sections.push('');
  } else {
    sections.push('# Project Foundation');
    sections.push('');
  }

  // Project Vision
  if (foundation.project && foundation.project.vision) {
    sections.push('## Vision');
    sections.push('');
    sections.push(foundation.project.vision);
    sections.push('');
    sections.push('---');
    sections.push('');
  }

  // Problem Space
  if (foundation.problemSpace && foundation.problemSpace.primaryProblem) {
    const problem = foundation.problemSpace.primaryProblem;

    sections.push('## Problem Space');
    sections.push('');

    if (problem.title) {
      sections.push(`### ${problem.title}`);
      sections.push('');
    }

    if (problem.description) {
      sections.push(problem.description);
      sections.push('');
    }

    if (problem.impact) {
      sections.push(`**Impact:** ${problem.impact}`);
      sections.push('');
    }

    sections.push('---');
    sections.push('');
  }

  // Solution Space
  if (foundation.solutionSpace) {
    sections.push('## Solution Space');
    sections.push('');

    if (foundation.solutionSpace.overview) {
      sections.push('### Overview');
      sections.push('');
      sections.push(foundation.solutionSpace.overview);
      sections.push('');
    }

    if (
      foundation.solutionSpace.capabilities &&
      foundation.solutionSpace.capabilities.length > 0
    ) {
      sections.push('### Capabilities');
      sections.push('');
      for (const capability of foundation.solutionSpace.capabilities) {
        if (capability.name && capability.description) {
          sections.push(`- **${capability.name}**: ${capability.description}`);
        } else if (capability.name) {
          sections.push(`- **${capability.name}**`);
        }
      }
      sections.push('');
    }

    sections.push('---');
    sections.push('');
  }

  // Personas
  if (foundation.personas && foundation.personas.length > 0) {
    sections.push('## Personas');
    sections.push('');

    for (const persona of foundation.personas) {
      if (persona.name) {
        sections.push(`### ${persona.name}`);
        sections.push('');
      }

      if (persona.description) {
        sections.push(persona.description);
        sections.push('');
      }

      if (persona.goals && persona.goals.length > 0) {
        sections.push('**Goals:**');
        for (const goal of persona.goals) {
          sections.push(`- ${goal}`);
        }
        sections.push('');
      }
    }

    sections.push('---');
    sections.push('');
  }

  // Architecture Diagrams
  if (
    foundation.architectureDiagrams &&
    foundation.architectureDiagrams.length > 0
  ) {
    sections.push('## Architecture Diagrams');
    sections.push('');

    for (const diagram of foundation.architectureDiagrams) {
      if (diagram.title) {
        sections.push(`### ${diagram.title}`);
        sections.push('');
      }

      if (diagram.description) {
        sections.push(diagram.description);
        sections.push('');
      }

      if (diagram.mermaidCode) {
        sections.push('```mermaid');
        sections.push(diagram.mermaidCode);
        sections.push('```');
        sections.push('');
      }
    }

    sections.push('---');
    sections.push('');
  }

  // Domain Architecture (Event Storm)
  if (foundation.eventStorm && foundation.eventStorm.items) {
    // Filter bounded contexts (not deleted)
    const boundedContexts = foundation.eventStorm.items.filter(
      (item): item is EventStormBoundedContext =>
        item.type === 'bounded_context' && !item.deleted
    );

    if (boundedContexts.length > 0) {
      sections.push('# Domain Architecture');
      sections.push('');

      // Bounded Contexts list
      sections.push('## Bounded Contexts');
      sections.push('');
      for (const context of boundedContexts) {
        sections.push(`- ${context.text}`);
      }
      sections.push('');

      // Bounded Context Map (Mermaid diagram)
      sections.push('## Bounded Context Map');
      sections.push('');

      // Generate Mermaid diagram
      const mermaidCode = generateBoundedContextMermaid(boundedContexts);

      // Validate Mermaid syntax before adding to FOUNDATION.md
      const validationResult = await validateMermaidSyntax(mermaidCode);
      if (!validationResult.valid) {
        throw new Error(
          `Mermaid validation failed: ${validationResult.error || 'Unknown error'}`
        );
      }

      sections.push('```mermaid');
      sections.push(mermaidCode);
      sections.push('```');
      sections.push('');

      // Bounded Context Sections (Aggregates, Events, Commands)
      for (const context of boundedContexts) {
        sections.push(`## ${context.text} Context`);
        sections.push('');

        // Generate Event Flow diagram for this context
        const eventFlowMermaid = generateContextEventFlowMermaid(
          context.id,
          foundation
        );

        if (eventFlowMermaid) {
          // Validate Mermaid syntax before adding
          const flowValidation = await validateMermaidSyntax(eventFlowMermaid);
          if (flowValidation.valid) {
            sections.push('### Event Flow');
            sections.push('');
            sections.push('```mermaid');
            sections.push(eventFlowMermaid);
            sections.push('```');
            sections.push('');
          }
        }

        // Filter aggregates for this bounded context
        const aggregates = foundation.eventStorm.items.filter(
          (item): item is any =>
            item.type === 'aggregate' &&
            !item.deleted &&
            'boundedContextId' in item &&
            item.boundedContextId === context.id
        );

        if (aggregates.length > 0) {
          sections.push('**Aggregates:**');
          for (const aggregate of aggregates) {
            const description =
              'description' in aggregate && aggregate.description
                ? ` - ${aggregate.description}`
                : '';
            sections.push(`- ${aggregate.text}${description}`);
          }
          sections.push('');
        }

        // Filter events for this bounded context
        const events = foundation.eventStorm.items.filter(
          (item): item is any =>
            item.type === 'event' &&
            !item.deleted &&
            'boundedContextId' in item &&
            item.boundedContextId === context.id
        );

        if (events.length > 0) {
          sections.push('**Domain Events:**');
          for (const event of events) {
            const description =
              'description' in event && event.description
                ? ` - ${event.description}`
                : '';
            sections.push(`- ${event.text}${description}`);
          }
          sections.push('');
        }

        // Filter commands for this bounded context
        const commands = foundation.eventStorm.items.filter(
          (item): item is any =>
            item.type === 'command' &&
            !item.deleted &&
            'boundedContextId' in item &&
            item.boundedContextId === context.id
        );

        if (commands.length > 0) {
          sections.push('**Commands:**');
          for (const command of commands) {
            const description =
              'description' in command && command.description
                ? ` - ${command.description}`
                : '';
            sections.push(`- ${command.text}${description}`);
          }
          sections.push('');
        }
      }

      sections.push('---');
      sections.push('');
    }
  }

  return sections.join('\n');
}
