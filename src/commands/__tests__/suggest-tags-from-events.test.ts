/**
 * Feature: spec/features/tag-discovery-from-event-storm-artifacts.feature
 * Coverage: EXMAP-008
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, writeFile } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import { fileManager } from '../../utils/file-manager';
import type { WorkUnitsData } from '../../types';
import {
  suggestTagsFromEvents,
  type SuggestTagsResult,
} from '../suggest-tags-from-events';

describe('Feature: Tag discovery from Event Storm artifacts', () => {
  let tmpDir: string;

  beforeEach(async () => {
    tmpDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    await mkdir(join(tmpDir, 'spec'), { recursive: true });
  });

  afterEach(async () => {
    await rm(tmpDir, { recursive: true, force: true });
  });

  describe('Scenario: Suggest component tag from bounded context', () => {
    it('should suggest @user-management component tag with high confidence', async () => {
      // @step Given I have a work unit TEST-001 with Event Storm data
      const workUnitsData: WorkUnitsData = {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        states: {
          backlog: [],
          specifying: ['TEST-001'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
        workUnits: {
          'TEST-001': {
            id: 'TEST-001',
            type: 'story',
            title: 'Test Work Unit',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            eventStorm: {
              level: 'process_modeling',
              items: [],
              nextItemId: 1,
            },
          },
        },
      };

      // @step And the work unit has a bounded context "User Management"
      workUnitsData.workUnits['TEST-001'].eventStorm!.items.push({
        id: 0,
        type: 'bounded_context',
        color: null,
        text: 'User Management',
        deleted: false,
        createdAt: new Date().toISOString(),
      });

      await writeFile(
        join(tmpDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // @step When I run fspec suggest-tags-from-events TEST-001
      const result = await suggestTagsFromEvents({
        workUnitId: 'TEST-001',
        cwd: tmpDir,
      });

      // @step Then the command should succeed
      expect(result.success).toBe(true);

      // @step And the output should suggest a component tag "@user-management"
      const componentSuggestions = result.suggestions?.filter(
        s => s.category === 'component'
      );
      expect(componentSuggestions).toBeDefined();
      expect(componentSuggestions!.length).toBeGreaterThan(0);
      expect(
        componentSuggestions!.some(s => s.tagName === '@user-management')
      ).toBe(true);

      // @step And the suggestion should have confidence "high"
      const userMgmtTag = componentSuggestions!.find(
        s => s.tagName === '@user-management'
      );
      expect(userMgmtTag?.confidence).toBe('high');

      // @step And the suggestion should reference source "bounded_context: User Management"
      expect(userMgmtTag?.source).toContain('bounded_context');
      expect(userMgmtTag?.source).toContain('User Management');
    });
  });

  describe('Scenario: Suggest feature group tag from domain event cluster', () => {
    it('should suggest @authentication feature group tag from related events', async () => {
      // @step Given I have a work unit TEST-002 with Event Storm data
      const workUnitsData: WorkUnitsData = {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        states: {
          backlog: [],
          specifying: ['TEST-002'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
        workUnits: {
          'TEST-002': {
            id: 'TEST-002',
            type: 'story',
            title: 'Test Work Unit',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            eventStorm: {
              level: 'process_modeling',
              items: [],
              nextItemId: 3,
            },
          },
        },
      };

      // @step And the work unit has domain events "UserRegistered", "UserLoggedIn", "PasswordReset"
      workUnitsData.workUnits['TEST-002'].eventStorm!.items.push(
        {
          id: 0,
          type: 'event',
          color: 'orange',
          text: 'UserRegistered',
          deleted: false,
          createdAt: new Date().toISOString(),
        },
        {
          id: 1,
          type: 'event',
          color: 'orange',
          text: 'UserLoggedIn',
          deleted: false,
          createdAt: new Date().toISOString(),
        },
        {
          id: 2,
          type: 'event',
          color: 'orange',
          text: 'PasswordReset',
          deleted: false,
          createdAt: new Date().toISOString(),
        }
      );

      await writeFile(
        join(tmpDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // @step When I run fspec suggest-tags-from-events TEST-002
      const result = await suggestTagsFromEvents({
        workUnitId: 'TEST-002',
        cwd: tmpDir,
      });

      // @step Then the command should succeed
      expect(result.success).toBe(true);

      // @step And the output should suggest a feature group tag "@authentication"
      const featureGroupSuggestions = result.suggestions?.filter(
        s => s.category === 'feature-group'
      );
      expect(featureGroupSuggestions).toBeDefined();
      expect(
        featureGroupSuggestions!.some(s => s.tagName === '@authentication')
      ).toBe(true);

      // @step And the suggestion should reference events "UserRegistered, UserLoggedIn, PasswordReset"
      const authTag = featureGroupSuggestions!.find(
        s => s.tagName === '@authentication'
      );
      expect(authTag?.source).toContain('UserRegistered');
      expect(authTag?.source).toContain('UserLoggedIn');
      expect(authTag?.source).toContain('PasswordReset');
    });
  });

  describe('Scenario: Suggest component tag from aggregate', () => {
    it('should suggest @user component tag from User aggregate', async () => {
      // @step Given I have a work unit TEST-003 with Event Storm data
      const workUnitsData: WorkUnitsData = {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        states: {
          backlog: [],
          specifying: ['TEST-003'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
        workUnits: {
          'TEST-003': {
            id: 'TEST-003',
            type: 'story',
            title: 'Test Work Unit',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            eventStorm: {
              level: 'process_modeling',
              items: [],
              nextItemId: 1,
            },
          },
        },
      };

      // @step And the work unit has an aggregate "User"
      // @step And the aggregate has domain events "UserRegistered", "ProfileUpdated"
      workUnitsData.workUnits['TEST-003'].eventStorm!.items.push({
        id: 0,
        type: 'aggregate',
        color: 'yellow',
        text: 'User',
        deleted: false,
        createdAt: new Date().toISOString(),
        emits: ['UserRegistered', 'ProfileUpdated'],
      });

      await writeFile(
        join(tmpDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // @step When I run fspec suggest-tags-from-events TEST-003
      const result = await suggestTagsFromEvents({
        workUnitId: 'TEST-003',
        cwd: tmpDir,
      });

      // @step Then the command should succeed
      expect(result.success).toBe(true);

      // @step And the output should suggest a component tag "@user"
      const componentSuggestions = result.suggestions?.filter(
        s => s.category === 'component'
      );
      expect(componentSuggestions).toBeDefined();
      expect(componentSuggestions!.some(s => s.tagName === '@user')).toBe(true);

      // @step And the suggestion should reference source "aggregate: User"
      const userTag = componentSuggestions!.find(s => s.tagName === '@user');
      expect(userTag?.source).toContain('aggregate');
      expect(userTag?.source).toContain('User');
    });
  });

  describe('Scenario: Suggest technical tags from external system', () => {
    it('should suggest @oauth and @rest-api technical tags', async () => {
      // @step Given I have a work unit TEST-004 with Event Storm data
      const workUnitsData: WorkUnitsData = {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        states: {
          backlog: [],
          specifying: ['TEST-004'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
        workUnits: {
          'TEST-004': {
            id: 'TEST-004',
            type: 'story',
            title: 'Test Work Unit',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            eventStorm: {
              level: 'process_modeling',
              items: [],
              nextItemId: 1,
            },
          },
        },
      };

      // @step And the work unit has an external system "OAuth2Provider" with integrationType "REST_API"
      workUnitsData.workUnits['TEST-004'].eventStorm!.items.push({
        id: 0,
        type: 'external_system',
        color: 'pink',
        text: 'OAuth2Provider',
        deleted: false,
        createdAt: new Date().toISOString(),
        integrationType: 'REST_API',
      });

      await writeFile(
        join(tmpDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // @step When I run fspec suggest-tags-from-events TEST-004
      const result = await suggestTagsFromEvents({
        workUnitId: 'TEST-004',
        cwd: tmpDir,
      });

      // @step Then the command should succeed
      expect(result.success).toBe(true);

      // @step And the output should suggest technical tag "@oauth"
      const technicalSuggestions = result.suggestions?.filter(
        s => s.category === 'technical'
      );
      expect(technicalSuggestions).toBeDefined();
      expect(technicalSuggestions!.some(s => s.tagName === '@oauth')).toBe(
        true
      );

      // @step And the output should suggest technical tag "@rest-api"
      expect(technicalSuggestions!.some(s => s.tagName === '@rest-api')).toBe(
        true
      );

      // @step And both suggestions should reference source "external_system: OAuth2Provider"
      const oauthTag = technicalSuggestions!.find(s => s.tagName === '@oauth');
      const restApiTag = technicalSuggestions!.find(
        s => s.tagName === '@rest-api'
      );
      expect(oauthTag?.source).toContain('external_system');
      expect(oauthTag?.source).toContain('OAuth2Provider');
      expect(restApiTag?.source).toContain('external_system');
      expect(restApiTag?.source).toContain('OAuth2Provider');
    });
  });

  describe('Scenario: Fail when work unit has no Event Storm artifacts', () => {
    it('should fail with error message when no Event Storm data exists', async () => {
      // @step Given I have a work unit TEST-005 with no Event Storm data
      const workUnitsData: WorkUnitsData = {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        states: {
          backlog: [],
          specifying: ['TEST-005'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
        workUnits: {
          'TEST-005': {
            id: 'TEST-005',
            type: 'story',
            title: 'Test Work Unit',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            // No eventStorm section
          },
        },
      };

      await writeFile(
        join(tmpDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // @step When I run fspec suggest-tags-from-events TEST-005
      const result = await suggestTagsFromEvents({
        workUnitId: 'TEST-005',
        cwd: tmpDir,
      });

      // @step Then the command should fail with exit code 1
      expect(result.success).toBe(false);

      // @step And the error message should contain "No Event Storm artifacts found for TEST-005"
      expect(result.error).toContain('No Event Storm artifacts found');
      expect(result.error).toContain('TEST-005');
    });
  });

  describe('Scenario: Suggest feature group tag from multiple related events', () => {
    it('should suggest @checkpoint-management from checkpoint events in kebab-case', async () => {
      // @step Given I have a work unit TEST-006 with Event Storm data
      const workUnitsData: WorkUnitsData = {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        states: {
          backlog: [],
          specifying: ['TEST-006'],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
        workUnits: {
          'TEST-006': {
            id: 'TEST-006',
            type: 'story',
            title: 'Test Work Unit',
            status: 'specifying',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            eventStorm: {
              level: 'process_modeling',
              items: [],
              nextItemId: 3,
            },
          },
        },
      };

      // @step And the work unit has domain events "CheckpointCreated", "CheckpointRestored", "CheckpointDeleted"
      workUnitsData.workUnits['TEST-006'].eventStorm!.items.push(
        {
          id: 0,
          type: 'event',
          color: 'orange',
          text: 'CheckpointCreated',
          deleted: false,
          createdAt: new Date().toISOString(),
        },
        {
          id: 1,
          type: 'event',
          color: 'orange',
          text: 'CheckpointRestored',
          deleted: false,
          createdAt: new Date().toISOString(),
        },
        {
          id: 2,
          type: 'event',
          color: 'orange',
          text: 'CheckpointDeleted',
          deleted: false,
          createdAt: new Date().toISOString(),
        }
      );

      await writeFile(
        join(tmpDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // @step When I run fspec suggest-tags-from-events TEST-006
      const result = await suggestTagsFromEvents({
        workUnitId: 'TEST-006',
        cwd: tmpDir,
      });

      // @step Then the command should succeed
      expect(result.success).toBe(true);

      // @step And the output should suggest a feature group tag "@checkpoint-management"
      const featureGroupSuggestions = result.suggestions?.filter(
        s => s.category === 'feature-group'
      );
      expect(featureGroupSuggestions).toBeDefined();
      expect(
        featureGroupSuggestions!.some(
          s => s.tagName === '@checkpoint-management'
        )
      ).toBe(true);

      // @step And the tag name should be in kebab-case format
      const checkpointTag = featureGroupSuggestions!.find(
        s => s.tagName === '@checkpoint-management'
      );
      expect(checkpointTag?.tagName).toMatch(/^@[a-z-]+$/); // kebab-case regex
    });
  });
});
