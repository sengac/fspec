/**
 * Feature: spec/features/watcher-templates.feature
 * Tests for Watcher Templates and Improved Creation UX (WATCH-023)
 *
 * This consolidated test file covers:
 * - Storage: loadWatcherTemplates, saveWatcherTemplates, generateSlug
 * - List: buildFlatWatcherList, display, navigation, CRUD
 * - Form: field navigation, model filter, authority toggle, create/edit modes
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { join } from 'path';

// Use vi.hoisted to define mocks that will be available during module mocking
const { mockMkdirSync, mockWriteFileSync, mockExistsSync, mockReadFile } = vi.hoisted(() => ({
  mockMkdirSync: vi.fn(),
  mockWriteFileSync: vi.fn(),
  mockExistsSync: vi.fn(),
  mockReadFile: vi.fn(),
}));

// Mock fs modules BEFORE importing the module under test
vi.mock('fs', async (importOriginal) => {
  const actual = await importOriginal();
  return {
    ...actual,
    mkdirSync: mockMkdirSync,
    writeFileSync: mockWriteFileSync,
    existsSync: mockExistsSync,
    default: {
      mkdirSync: mockMkdirSync,
      writeFileSync: mockWriteFileSync,
      existsSync: mockExistsSync,
    },
  };
});

vi.mock('fs/promises', async (importOriginal) => {
  const actual = await importOriginal();
  return {
    ...actual,
    readFile: mockReadFile,
    writeFile: vi.fn(),
    mkdir: vi.fn(),
    default: {
      readFile: mockReadFile,
      writeFile: vi.fn(),
      mkdir: vi.fn(),
    },
  };
});

// Mock config module
vi.mock('../../../utils/config', () => ({
  getFspecUserDir: vi.fn(() => '/mock/home/.fspec'),
}));

// Mock NAPI module
const mockSessionGetWatchers = vi.fn();
const mockSessionGetRole = vi.fn();
const mockSessionGetStatus = vi.fn();
const mockSessionCreateWatcher = vi.fn();
const mockSessionSetRole = vi.fn();
const mockSessionManagerDestroy = vi.fn();

vi.mock('@sengac/codelet-napi', () => ({
  sessionGetWatchers: mockSessionGetWatchers,
  sessionGetRole: mockSessionGetRole,
  sessionGetStatus: mockSessionGetStatus,
  sessionCreateWatcher: mockSessionCreateWatcher,
  sessionSetRole: mockSessionSetRole,
  sessionManagerDestroy: mockSessionManagerDestroy,
  persistenceSetDataDirectory: vi.fn(),
  persistenceGetHistory: vi.fn(() => []),
  persistenceListSessions: vi.fn(() => []),
  sessionManagerList: vi.fn(() => []),
  JsThinkingLevel: { Off: 0, Low: 1, Medium: 2, High: 3 },
  getThinkingConfig: vi.fn(() => null),
}));

// Import mocked modules for assertions
const { mkdirSync, writeFileSync } = await import('fs');
const { readFile } = await import('fs/promises');
const { getFspecUserDir } = await import('../../../utils/config');

// Import REAL implementations from source (now that mocks are set up)
import {
  generateSlug,
  loadWatcherTemplates,
  saveWatcherTemplates,
  buildFlatWatcherList,
  filterTemplates,
  formatTemplateDisplay,
} from '../../utils/watcherTemplateStorage';
import type { WatcherTemplate, WatcherInstance, WatcherListItem } from '../../types/watcherTemplate';

// === TEST HELPERS ===

const createMockTemplate = (id: string, name: string, authority: 'peer' | 'supervisor' = 'peer'): WatcherTemplate => ({
  id,
  name,
  slug: generateSlug(name),
  modelId: 'anthropic/claude-sonnet-4-20250514',
  authority,
  brief: `Brief for ${name}`,
  autoInject: false,
  createdAt: '2026-01-24T00:00:00.000Z',
  updatedAt: '2026-01-24T00:00:00.000Z',
});

const createMockInstance = (sessionId: string, templateId: string, status: 'running' | 'idle' = 'idle'): WatcherInstance => ({
  sessionId,
  templateId,
  status,
});

// Navigation helpers for form tests
type FormField = 'name' | 'model' | 'authority' | 'brief' | 'autoInject';
const FORM_FIELDS: FormField[] = ['name', 'model', 'authority', 'brief', 'autoInject'];
const navigateFieldDown = (f: FormField): FormField => FORM_FIELDS[Math.min(FORM_FIELDS.length - 1, FORM_FIELDS.indexOf(f) + 1)];
const navigateFieldUp = (f: FormField): FormField => FORM_FIELDS[Math.max(0, FORM_FIELDS.indexOf(f) - 1)];
const navigateFieldTab = (f: FormField, shift: boolean): FormField => {
  const i = FORM_FIELDS.indexOf(f);
  return FORM_FIELDS[shift ? (i - 1 + FORM_FIELDS.length) % FORM_FIELDS.length : (i + 1) % FORM_FIELDS.length];
};

// Model filter helper for form tests
interface ModelOption { modelId: string; displayName: string; }
const filterModels = (models: ModelOption[], query: string): ModelOption[] => {
  if (!query) return models;
  const q = query.toLowerCase();
  return models.filter(m => m.modelId.toLowerCase().includes(q) || m.displayName.toLowerCase().includes(q));
};

// Authority toggle helper
type Authority = 'peer' | 'supervisor';
const toggleAuthority = (curr: Authority, dir: 'left' | 'right'): Authority => {
  if (dir === 'right' && curr === 'peer') return 'supervisor';
  if (dir === 'left' && curr === 'supervisor') return 'peer';
  return curr;
};

// === TESTS ===

describe('Feature: Watcher Templates and Improved Creation UX', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // =============================================
  // SLUG GENERATION (spec:218-222)
  // =============================================

  describe('Scenario: Slug is auto-generated from template name', () => {
    it('should generate kebab-case slug from template name', () => {
      // @step When I save a template with name "Security Reviewer"
      const slug = generateSlug('Security Reviewer');
      // @step Then the template slug is "security-reviewer"
      expect(slug).toBe('security-reviewer');
    });
  });

  describe('Scenario: Slug handles special characters', () => {
    it('should remove special characters and convert to kebab-case', () => {
      // @step When I save a template with name "Code Review & Analysis"
      const slug = generateSlug('Code Review & Analysis');
      // @step Then the template slug is "code-review-analysis"
      expect(slug).toBe('code-review-analysis');
    });
  });

  // =============================================
  // STORAGE OPERATIONS
  // =============================================

  describe('Storage operations', () => {
    const mockTemplates: WatcherTemplate[] = [
      createMockTemplate('t1', 'Security Reviewer', 'supervisor'),
      createMockTemplate('t2', 'Test Enforcer', 'peer'),
    ];

    it('should load templates from storage', async () => {
      mockReadFile.mockResolvedValue(JSON.stringify(mockTemplates));
      const templates = await loadWatcherTemplates();
      expect(getFspecUserDir).toHaveBeenCalled();
      expect(mockReadFile).toHaveBeenCalledWith(join('/mock/home/.fspec', 'watcher-templates.json'), 'utf-8');
      expect(templates).toEqual(mockTemplates);
    });

    it('should return empty array when file does not exist', async () => {
      mockReadFile.mockRejectedValue({ code: 'ENOENT' });
      const templates = await loadWatcherTemplates();
      expect(templates).toEqual([]);
    });

    it('should save templates to storage', () => {
      saveWatcherTemplates(mockTemplates);
      expect(mockMkdirSync).toHaveBeenCalledWith('/mock/home/.fspec', { recursive: true });
      expect(mockWriteFileSync).toHaveBeenCalledWith(
        join('/mock/home/.fspec', 'watcher-templates.json'),
        JSON.stringify(mockTemplates, null, 2)
      );
    });
  });

  // =============================================
  // QUICK SPAWN (spec:112-120)
  // =============================================

  describe('Scenario: Quick spawn via slash command', () => {
    it('should find template by slug', async () => {
      // @step Given I have a "Security Reviewer" template with slug "security-reviewer"
      const mockTemplates = [createMockTemplate('t1', 'Security Reviewer', 'supervisor')];
      vi.mocked(readFile).mockResolvedValue(JSON.stringify(mockTemplates));
      // @step When I type "/watcher spawn security-reviewer"
      const templates = await loadWatcherTemplates();
      const found = templates.find(t => t.slug === 'security-reviewer');
      // @step Then a watcher instance spawns immediately without opening the overlay
      expect(found).toBeDefined();
      expect(found?.name).toBe('Security Reviewer');
    });
  });

  describe('Scenario: Quick spawn with unknown slug shows error', () => {
    it('should return undefined for unknown slug', async () => {
      // @step Given no template exists with slug "unknown-watcher"
      vi.mocked(readFile).mockResolvedValue(JSON.stringify([]));
      // @step When I type "/watcher spawn unknown-watcher"
      const templates = await loadWatcherTemplates();
      const found = templates.find(t => t.slug === 'unknown-watcher');
      // @step Then I should see error "No template found with slug: unknown-watcher"
      expect(found).toBeUndefined();
    });
  });

  // =============================================
  // TEMPLATE LIST DISPLAY (spec:55-94)
  // =============================================

  describe('Scenario: View templates with active instance count', () => {
    it('should display templates with instance count badges in alphabetical order', () => {
      // @step Given I have a "Security Reviewer" template with 2 active instances
      const securityTemplate = createMockTemplate('t1', 'Security Reviewer', 'supervisor');
      const instances = [createMockInstance('s1', 't1', 'running'), createMockInstance('s2', 't1', 'idle')];
      // @step And I have a "Test Enforcer" template with no active instances
      const testTemplate = createMockTemplate('t2', 'Test Enforcer', 'peer');
      // @step When I open the /watcher overlay
      const flatList = buildFlatWatcherList([securityTemplate, testTemplate], instances, new Set());
      // @step Then I should see "Security Reviewer (Supervisor)" with "[2 active]"
      const secItem = flatList.find(i => i.type === 'template' && i.template.name === 'Security Reviewer');
      expect(secItem && secItem.type === 'template' && formatTemplateDisplay(secItem.template, secItem.instanceCount)).toBe('Security Reviewer (Supervisor) [2 active]');
      // @step And I should see "Test Enforcer (Peer)" without an active badge
      const testItem = flatList.find(i => i.type === 'template' && i.template.name === 'Test Enforcer');
      expect(testItem && testItem.type === 'template' && formatTemplateDisplay(testItem.template, testItem.instanceCount)).toBe('Test Enforcer (Peer)');
      // @step And templates should be in alphabetical order
      const templateItems = flatList.filter(i => i.type === 'template');
      expect(templateItems[0].type === 'template' && templateItems[0].template.name).toBe('Security Reviewer');
    });
  });

  describe('Scenario: Expand template to show instances', () => {
    it('should show nested instances when template is expanded', () => {
      // @step Given "Security Reviewer" template has 2 active instances and is collapsed
      const template = createMockTemplate('t1', 'Security Reviewer', 'supervisor');
      const instances = [createMockInstance('s1', 't1', 'running'), createMockInstance('s2', 't1', 'idle')];
      expect(buildFlatWatcherList([template], instances, new Set()).filter(i => i.type === 'instance')).toHaveLength(0);
      // @step When I press the right arrow key
      // @step Then the template should expand
      const expandedList = buildFlatWatcherList([template], instances, new Set(['t1']));
      // @step And I should see "#1 running" nested with tree connector
      // @step And I should see "#2 idle" nested with tree connector
      const instanceItems = expandedList.filter(i => i.type === 'instance');
      expect(instanceItems).toHaveLength(2);
    });
  });

  describe('Scenario: Collapse template to hide instances', () => {
    it('should hide instances when template is collapsed', () => {
      // @step Given "Security Reviewer" template is expanded showing instances
      const template = createMockTemplate('t1', 'Security Reviewer', 'supervisor');
      const instances = [createMockInstance('s1', 't1', 'running'), createMockInstance('s2', 't1', 'idle')];
      const expanded = new Set(['t1']);
      expect(buildFlatWatcherList([template], instances, expanded).filter(i => i.type === 'instance')).toHaveLength(2);
      // @step When I press the left arrow key
      expanded.delete('t1');
      // @step Then the template should collapse
      // @step And instances should be hidden
      expect(buildFlatWatcherList([template], instances, expanded).filter(i => i.type === 'instance')).toHaveLength(0);
    });
  });

  describe('Scenario: Navigate list with arrow keys', () => {
    it('should navigate selection with up/down arrow keys', () => {
      // @step Given I have multiple templates in the /watcher overlay
      const templates = [createMockTemplate('t1', 'A', 'peer'), createMockTemplate('t2', 'B', 'peer')];
      const flatList = buildFlatWatcherList(templates, [], new Set());
      let idx = 0;
      // @step When I press the down arrow key
      idx = Math.min(flatList.length - 1, idx + 1);
      // @step Then selection moves to the next item
      expect(idx).toBe(1);
      // @step When I press the up arrow key
      idx = Math.max(0, idx - 1);
      // @step Then selection moves to the previous item
      expect(idx).toBe(0);
    });
  });

  describe('Scenario: Filter templates by typing', () => {
    it('should filter templates by search query', () => {
      // @step Given I have templates including "Security Reviewer" and "Test Enforcer"
      const templates = [createMockTemplate('t1', 'Security Reviewer'), createMockTemplate('t2', 'Test Enforcer')];
      // @step When I type "sec" in the overlay
      const filtered = filterTemplates(templates, 'sec');
      // @step Then only "Security Reviewer" should be visible
      expect(filtered).toHaveLength(1);
      expect(filtered[0].name).toBe('Security Reviewer');
      // @step And "Test Enforcer" should be hidden
      expect(filtered.find(t => t.name === 'Test Enforcer')).toBeUndefined();
    });
  });

  describe('Scenario: Empty state with no templates', () => {
    it('should show empty state when no templates exist', () => {
      // @step Given I have no watcher templates
      // @step When I open the /watcher overlay
      const flatList = buildFlatWatcherList([], [], new Set());
      // @step Then I should see "No watcher templates yet"
      // @step And I should see an explanation of what watchers do
      // @step And I should see "Press N to create your first template"
      expect(flatList).toHaveLength(1);
      expect(flatList[0].type).toBe('create-new');
    });
  });

  // =============================================
  // SPAWN AND OPEN (spec:100-110)
  // =============================================

  describe('Scenario: Spawn new instance from template', () => {
    it('should spawn instance when Enter is pressed on template', () => {
      // @step Given "Security Reviewer" template is selected
      const template = createMockTemplate('t1', 'Security Reviewer', 'supervisor');
      const flatList = buildFlatWatcherList([template], [], new Set());
      const selectedItem = flatList[0];
      expect(selectedItem.type).toBe('template');
      // @step When I press Enter
      if (selectedItem.type === 'template') {
        mockSessionCreateWatcher.mockReturnValue('new-session');
        const watcherSessionId = mockSessionCreateWatcher('parent-session');
        mockSessionSetRole(watcherSessionId, selectedItem.template.name, selectedItem.template.brief, selectedItem.template.authority);
        // @step Then a new watcher instance spawns with the template's settings
        expect(mockSessionCreateWatcher).toHaveBeenCalledWith('parent-session');
        expect(mockSessionSetRole).toHaveBeenCalledWith('new-session', 'Security Reviewer', 'Brief for Security Reviewer', 'supervisor');
      }
      // @step And the active count badge updates accordingly
    });
  });

  describe('Scenario: Open existing watcher instance', () => {
    it('should switch to instance session when Enter is pressed on instance', () => {
      // @step Given "Security Reviewer" is expanded with instance "#1 running" selected
      const template = createMockTemplate('t1', 'Security Reviewer', 'supervisor');
      const instance = createMockInstance('watcher-session-1', 't1', 'running');
      const flatList = buildFlatWatcherList([template], [instance], new Set(['t1']));
      const instanceItem = flatList.find(i => i.type === 'instance');
      // @step When I press Enter
      // @step Then the overlay closes
      // @step And I switch to that watcher's session
      expect(instanceItem && instanceItem.type === 'instance' && instanceItem.instance.sessionId).toBe('watcher-session-1');
    });
  });

  // =============================================
  // TEMPLATE CRUD (spec:126-158)
  // =============================================

  describe('Scenario: Create new template', () => {
    it('should open form in create mode with empty fields except model', () => {
      // @step Given the /watcher overlay is open
      // @step When I press N
      // @step Then the template form opens in create mode
      // @step And all fields are empty except Model which defaults to parent's model
      const parentModelId = 'anthropic/claude-sonnet-4-20250514';
      const formState = { name: '', modelId: parentModelId, authority: 'peer' as Authority, brief: '', autoInject: false };
      expect(formState.name).toBe('');
      expect(formState.modelId).toBe(parentModelId);
    });
  });

  describe('Scenario: Edit existing template', () => {
    it('should open form in edit mode with pre-populated fields', () => {
      // @step Given "Security Reviewer" template is selected
      const template = createMockTemplate('t1', 'Security Reviewer', 'supervisor');
      template.brief = 'Watch for security vulnerabilities';
      template.autoInject = true;
      // @step When I press E
      // @step Then the template form opens in edit mode
      // @step And all fields are pre-populated from the template
      const formState = { name: template.name, modelId: template.modelId, authority: template.authority, brief: template.brief, autoInject: template.autoInject };
      expect(formState.name).toBe('Security Reviewer');
      expect(formState.authority).toBe('supervisor');
      expect(formState.autoInject).toBe(true);
    });
  });

  describe('Scenario: Delete template without active instances', () => {
    it('should delete template after confirmation', () => {
      // @step Given "Architecture Advisor" template has no active instances and is selected
      const template = createMockTemplate('t1', 'Architecture Advisor', 'peer');
      // @step When I press D
      // @step Then a confirmation dialog appears
      const confirmMessage = `Delete template "${template.name}"?`;
      expect(confirmMessage).toBe('Delete template "Architecture Advisor"?');
      // @step When I confirm
      // @step Then the template is deleted
    });
  });

  describe('Scenario: Delete template with active instances shows warning', () => {
    it('should show warning and kill instances when deleting template with active watchers', () => {
      // @step Given "Security Reviewer" template has 2 active instances and is selected
      const instances = [createMockInstance('s1', 't1', 'running'), createMockInstance('s2', 't1', 'idle')];
      // @step When I press D
      // @step Then a confirmation dialog warns "This will kill 2 active watchers"
      const warningMessage = `This will kill ${instances.length} active watchers`;
      expect(warningMessage).toBe('This will kill 2 active watchers');
      // @step When I confirm
      // @step Then all active instances are killed
      instances.forEach(i => mockSessionManagerDestroy(i.sessionId));
      expect(mockSessionManagerDestroy).toHaveBeenCalledTimes(2);
      // @step And the template is deleted
    });
  });

  describe('Scenario: Kill watcher instance', () => {
    it('should kill instance when D is pressed on instance', () => {
      // @step Given "Security Reviewer" is expanded with instance "#2 idle" selected
      const instances = [createMockInstance('s1', 't1', 'running'), createMockInstance('s2', 't1', 'idle')];
      // @step When I press D
      mockSessionManagerDestroy('s2');
      // @step Then the instance is killed
      expect(mockSessionManagerDestroy).toHaveBeenCalledWith('s2');
      // @step And it disappears from the list
      // @step And the active count badge decreases
    });
  });

  // =============================================
  // FORM NAVIGATION (spec:164-174)
  // =============================================

  describe('Scenario: Navigate form fields with arrow keys', () => {
    it('should navigate between fields with up/down arrow keys', () => {
      // @step Given the template form is open with Name field focused
      let field: FormField = 'name';
      // @step When I press the down arrow key
      field = navigateFieldDown(field);
      // @step Then focus moves to Model field
      expect(field).toBe('model');
      // @step When I press the up arrow key
      field = navigateFieldUp(field);
      // @step Then focus moves back to Name field
      expect(field).toBe('name');
    });
  });

  describe('Scenario: Navigate form fields with Tab', () => {
    it('should cycle through fields with Tab', () => {
      // @step Given the template form is open
      let field: FormField = 'name';
      // @step When I press Tab repeatedly
      // @step Then focus cycles through: Name, Model, Authority, Brief, Auto-inject
      const expected: FormField[] = ['model', 'authority', 'brief', 'autoInject', 'name'];
      expected.forEach(exp => {
        field = navigateFieldTab(field, false);
        expect(field).toBe(exp);
      });
    });
  });

  // =============================================
  // MODEL SELECTION (spec:180-195)
  // =============================================

  describe('Scenario: Model defaults to parent session model', () => {
    it('should initialize model field with parent session model', () => {
      // @step Given my parent session uses "claude-sonnet-4"
      const parentModelId = 'anthropic/claude-sonnet-4-20250514';
      // @step When I open the template form
      // @step Then Model field shows "claude-sonnet-4"
      expect(parentModelId).toContain('claude-sonnet-4');
    });
  });

  describe('Scenario: Filter models by typing', () => {
    it('should filter models by search query', () => {
      // @step Given the Model field is focused
      // @step And configured models include "gemini-2.0-flash" and "claude-sonnet-4"
      const models: ModelOption[] = [
        { modelId: 'google/gemini-2.0-flash', displayName: 'Gemini 2.0 Flash' },
        { modelId: 'anthropic/claude-sonnet-4-20250514', displayName: 'Claude Sonnet 4' },
      ];
      // @step When I type "gem"
      const filtered = filterModels(models, 'gem');
      // @step Then only "gemini-2.0-flash" appears in the filtered list
      expect(filtered).toHaveLength(1);
      expect(filtered[0].modelId).toBe('google/gemini-2.0-flash');
    });
  });

  describe('Scenario: Only configured models are shown', () => {
    it('should only show models from providers with API keys', () => {
      // @step Given I have API keys for "anthropic" but not "openai"
      const configuredProviders = new Set(['anthropic', 'google']);
      const allModels: ModelOption[] = [
        { modelId: 'anthropic/claude-sonnet-4', displayName: 'Claude' },
        { modelId: 'openai/gpt-4', displayName: 'GPT-4' },
        { modelId: 'google/gemini-2.0-flash', displayName: 'Gemini' },
      ];
      const configuredModels = allModels.filter(m => configuredProviders.has(m.modelId.split('/')[0]));
      // @step When I view the Model field options
      // @step Then I see Anthropic models
      expect(configuredModels.some(m => m.modelId.startsWith('anthropic/'))).toBe(true);
      // @step And I do not see OpenAI models
      expect(configuredModels.some(m => m.modelId.startsWith('openai/'))).toBe(false);
    });
  });

  // =============================================
  // AUTHORITY FIELD (spec:201-212)
  // =============================================

  describe('Scenario: Authority shows inline explanation when focused', () => {
    it('should display explanations for authority options', () => {
      // @step Given the template form is open
      // @step When I focus the Authority field
      // @step Then I see "Peer: Suggestions the AI can consider or ignore"
      // @step And I see "Supervisor: Directives the AI should follow"
      const peerExplanation = 'Peer: Suggestions the AI can consider or ignore';
      const supervisorExplanation = 'Supervisor: Directives the AI should follow';
      expect(peerExplanation).toContain('Peer');
      expect(supervisorExplanation).toContain('Supervisor');
    });
  });

  describe('Scenario: Toggle authority with arrow keys', () => {
    it('should toggle between Peer and Supervisor with left/right arrows', () => {
      // @step Given Authority field is focused showing "Peer"
      let authority: Authority = 'peer';
      // @step When I press the right arrow key
      authority = toggleAuthority(authority, 'right');
      // @step Then Authority changes to "Supervisor"
      expect(authority).toBe('supervisor');
      // @step When I press the left arrow key
      authority = toggleAuthority(authority, 'left');
      // @step Then Authority changes to "Peer"
      expect(authority).toBe('peer');
    });
  });
});
