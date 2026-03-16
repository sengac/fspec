/**
 * Supervisor Template Storage Utilities
 *
 * Handles persistence of supervisor templates to user-level storage.
 * Part of WATCH-023: Supervisor Templates and Improved Creation UX
 *
 * @see spec/features/supervisor-templates.feature
 */

import { join } from 'path';
import { mkdirSync, writeFileSync } from 'fs';
import { readFile } from 'fs/promises';
import { randomUUID } from 'crypto';
import { getFspecUserDir } from '../../utils/config';
import type {
  SupervisorTemplate,
  SupervisorInstance,
  SupervisorListItem,
} from '../types/supervisorTemplate';

const TEMPLATES_FILENAME = 'supervisor-templates.json';

/**
 * Get the path to the supervisor templates file.
 */
export function getTemplatesPath(): string {
  return join(getFspecUserDir(), TEMPLATES_FILENAME);
}

/**
 * Generate a URL-friendly slug from a template name.
 *
 * @example
 * generateSlug('Security Reviewer') // 'security-reviewer'
 * generateSlug('Code Review & Analysis') // 'code-review-analysis'
 */
export function generateSlug(name: string): string {
  return name
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9\s-]/g, '') // Remove special chars except spaces/hyphens
    .replace(/\s+/g, '-') // Replace spaces with hyphens
    .replace(/-+/g, '-') // Collapse multiple hyphens
    .replace(/^-|-$/g, ''); // Trim leading/trailing hyphens
}

/**
 * Load supervisor templates from user storage.
 * Returns empty array if file doesn't exist or is invalid.
 */
export async function loadSupervisorTemplates(): Promise<SupervisorTemplate[]> {
  const templatePath = getTemplatesPath();
  try {
    const content = await readFile(templatePath, 'utf-8');
    return JSON.parse(content) as SupervisorTemplate[];
  } catch {
    return [];
  }
}

/**
 * Save supervisor templates to user storage.
 * Creates the directory if it doesn't exist.
 */
export function saveSupervisorTemplates(templates: SupervisorTemplate[]): void {
  const userDir = getFspecUserDir();
  mkdirSync(userDir, { recursive: true });
  const templatePath = getTemplatesPath();
  writeFileSync(templatePath, JSON.stringify(templates, null, 2));
}

/**
 * Find a template by its slug.
 */
export async function findTemplateBySlug(
  slug: string
): Promise<SupervisorTemplate | undefined> {
  const templates = await loadSupervisorTemplates();
  return templates.find(t => t.slug === slug);
}

/**
 * Create a new template with auto-generated ID and timestamps.
 */
export function createTemplate(
  name: string,
  modelId: string,
  brief: string,
  autoInject: boolean
): SupervisorTemplate {
  const now = new Date().toISOString();
  return {
    id: randomUUID(),
    name,
    slug: generateSlug(name),
    modelId,
    brief,
    autoInject,
    createdAt: now,
    updatedAt: now,
  };
}

/**
 * Update an existing template (updates timestamp and regenerates slug if name changed).
 */
export function updateTemplate(
  template: SupervisorTemplate,
  updates: Partial<
    Pick<SupervisorTemplate, 'name' | 'modelId' | 'brief' | 'autoInject'>
  >
): SupervisorTemplate {
  const newName = updates.name ?? template.name;
  return {
    ...template,
    ...updates,
    slug: newName !== template.name ? generateSlug(newName) : template.slug,
    updatedAt: new Date().toISOString(),
  };
}

/**
 * Build a flat list of templates and instances for navigation.
 * Templates are sorted alphabetically. Instances appear under expanded templates.
 * Follows the same pattern as buildFlatModelList() in AgentView.tsx.
 */
export function buildFlatSupervisorList(
  templates: SupervisorTemplate[],
  instances: SupervisorInstance[],
  expandedTemplates: Set<string>
): SupervisorListItem[] {
  const items: SupervisorListItem[] = [];

  // Sort templates alphabetically
  const sortedTemplates = [...templates].sort((a, b) =>
    a.name.localeCompare(b.name)
  );

  sortedTemplates.forEach(template => {
    const templateInstances = instances.filter(
      i => i.templateId === template.id
    );
    const hasInstances = templateInstances.length > 0;
    const isExpanded = expandedTemplates.has(template.id) && hasInstances;

    items.push({
      type: 'template',
      template,
      isExpanded,
      instanceCount: templateInstances.length,
    });

    if (isExpanded) {
      templateInstances.forEach(instance => {
        items.push({ type: 'instance', template, instance });
      });
    }
  });

  // Add "create new" option at the end
  items.push({ type: 'create-new' });

  return items;
}

/**
 * Filter templates by name (case-insensitive).
 * Used for type-to-filter search in the template list.
 */
export function filterTemplates(
  templates: SupervisorTemplate[],
  query: string
): SupervisorTemplate[] {
  if (!query.trim()) return templates;
  const lowerQuery = query.toLowerCase();
  return templates.filter(t => t.name.toLowerCase().includes(lowerQuery));
}

/**
 * Format template display string with instance count.
 */
export function formatTemplateDisplay(
  template: SupervisorTemplate,
  instanceCount: number
): string {
  const badge = instanceCount > 0 ? ` [${instanceCount} active]` : '';
  return `${template.name}${badge}`;
}
