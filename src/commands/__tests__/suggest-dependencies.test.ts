/**
 * Feature: spec/features/work-unit-dependency-management.feature
 * Scenario: Auto-suggest dependency relationships based on work unit metadata (@COV-047)
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtempSync, rmSync } from 'fs';
import { tmpdir } from 'os';
import { join } from 'path';
import { mkdir, writeFile } from 'fs/promises';
import type { WorkUnit } from '../../types/work-unit.js';

describe('Feature: Work Unit Dependency Management', () => {
  let tempDir: string;
  let specDir: string;
  let workUnitsFile: string;

  beforeEach(async () => {
    // Given I have a project with spec directory
    tempDir = mkdtempSync(join(tmpdir(), 'fspec-test-'));
    specDir = join(tempDir, 'spec');
    await mkdir(specDir, { recursive: true });
    workUnitsFile = join(specDir, 'work-units.json');
  });

  afterEach(() => {
    rmSync(tempDir, { recursive: true, force: true });
  });

  describe('Scenario: Auto-suggest dependency relationships based on work unit metadata', () => {
    it('should suggest dependencies based on sequential IDs and title patterns', async () => {
      // And work units exist:
      const workUnits: WorkUnit[] = [
        {
          id: 'AUTH-001',
          title: 'Setup OAuth',
          status: 'implementing',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
        {
          id: 'AUTH-002',
          title: 'User Login Flow',
          status: 'backlog',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
        {
          id: 'API-001',
          title: 'Build User API',
          status: 'implementing',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
        {
          id: 'API-002',
          title: 'Test User API',
          status: 'backlog',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
        {
          id: 'DB-001',
          title: 'Database Schema Migration',
          status: 'implementing',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
        {
          id: 'DB-002',
          title: 'Add User Data',
          status: 'backlog',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
      ];

      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec suggest-dependencies --output=json"
      const { suggestDependencies } = await import('../suggest-dependencies.js');
      const result = await suggestDependencies({ cwd: tempDir, output: 'json' });

      // Then the output should suggest "AUTH-002 depends on AUTH-001" (sequential IDs)
      const auth002Suggestion = result.suggestions.find(
        (s: { from: string; to: string }) => s.from === 'AUTH-002' && s.to === 'AUTH-001'
      );
      expect(auth002Suggestion).toBeDefined();
      expect(auth002Suggestion.reason).toContain('sequential');

      // And the output should suggest "API-002 depends on API-001" (test depends on build)
      const api002Suggestion = result.suggestions.find(
        (s: { from: string; to: string }) => s.from === 'API-002' && s.to === 'API-001'
      );
      expect(api002Suggestion).toBeDefined();
      expect(api002Suggestion.reason).toMatch(/test.*build/i);

      // And the output should suggest "DB-002 depends on DB-001" (data depends on schema)
      const db002Suggestion = result.suggestions.find(
        (s: { from: string; to: string }) => s.from === 'DB-002' && s.to === 'DB-001'
      );
      expect(db002Suggestion).toBeDefined();
      expect(db002Suggestion.reason).toMatch(/schema|migration/i);

      // And each suggestion should indicate the reason/pattern matched
      result.suggestions.forEach((suggestion: { reason: string }) => {
        expect(suggestion.reason).toBeDefined();
        expect(suggestion.reason.length).toBeGreaterThan(0);
      });

      // And no suggestion should create circular dependencies
      const suggestionPairs = result.suggestions.map((s: { from: string; to: string }) => ({
        from: s.from,
        to: s.to,
      }));

      suggestionPairs.forEach((pair: { from: string; to: string }) => {
        const reversePair = suggestionPairs.find(
          (p: { from: string; to: string }) => p.from === pair.to && p.to === pair.from
        );
        expect(reversePair).toBeUndefined();
      });
    });
  });
});
