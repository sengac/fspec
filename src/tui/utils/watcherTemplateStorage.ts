/**
 * Watcher Template Storage Utilities
 *
 * Handles persistence of watcher templates to user-level storage.
 * Part of WATCH-023: Watcher Templates and Improved Creation UX
 *
 * @see spec/features/watcher-templates.feature
 */

import { join } from 'path';
import { mkdirSync, writeFileSync } from 'fs';
import { readFile } from 'fs/promises';
import { randomUUID } from 'crypto';
import { getFspecUserDir } from '../../utils/config';
import type {
  WatcherTemplate,
  WatcherInstance,
  WatcherListItem,
} from '../types/watcherTemplate';

const TEMPLATES_FILENAME = 'watcher-templates.json';

/**
 * Get the path to the watcher templates file.
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
 * Load watcher templates from user storage.
 * Returns empty array if file doesn't exist or is invalid.
 */
export async function loadWatcherTemplates(): Promise<WatcherTemplate[]> {
  const templatePath = getTemplatesPath();
  try {
    const content = await readFile(templatePath, 'utf-8');
    return JSON.parse(content) as WatcherTemplate[];
  } catch {
    return [];
  }
}

/**
 * Save watcher templates to user storage.
 * Creates the directory if it doesn't exist.
 */
export function saveWatcherTemplates(templates: WatcherTemplate[]): void {
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
): Promise<WatcherTemplate | undefined> {
  const templates = await loadWatcherTemplates();
  return templates.find(t => t.slug === slug);
}

/**
 * Create a new template with auto-generated ID and timestamps.
 */
export function createTemplate(
  name: string,
  modelId: string,
  authority: 'peer' | 'supervisor',
  brief: string,
  autoInject: boolean
): WatcherTemplate {
  const now = new Date().toISOString();
  return {
    id: randomUUID(),
    name,
    slug: generateSlug(name),
    modelId,
    authority,
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
  template: WatcherTemplate,
  updates: Partial<
    Pick<
      WatcherTemplate,
      'name' | 'modelId' | 'authority' | 'brief' | 'autoInject'
    >
  >
): WatcherTemplate {
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
export function buildFlatWatcherList(
  templates: WatcherTemplate[],
  instances: WatcherInstance[],
  expandedTemplates: Set<string>
): WatcherListItem[] {
  const items: WatcherListItem[] = [];

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
  templates: WatcherTemplate[],
  query: string
): WatcherTemplate[] {
  if (!query.trim()) return templates;
  const lowerQuery = query.toLowerCase();
  return templates.filter(t => t.name.toLowerCase().includes(lowerQuery));
}

/**
 * Format template display string with authority and instance count.
 */
export function formatTemplateDisplay(
  template: WatcherTemplate,
  instanceCount: number
): string {
  const authorityDisplay =
    template.authority === 'supervisor' ? 'Supervisor' : 'Peer';
  const badge = instanceCount > 0 ? ` [${instanceCount} active]` : '';
  return `${template.name} (${authorityDisplay})${badge}`;
}
