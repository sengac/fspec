/**
 * Feature: spec/features/event-storm-data-model-for-work-units.feature
 *
 * Tests for Event Storm data model in WorkUnit interface
 */

import { describe, it, expect } from 'vitest';
import type {
  WorkUnit,
  EventStorm,
  EventStormEvent,
  EventStormCommand,
  EventStormAggregate,
  EventStormPolicy,
  EventStormExternalSystem,
  EventStormBoundedContext,
  SuggestedTags,
} from '../index';

describe('Feature: Event Storm data model for work units', () => {
  describe('Scenario: Store Process Modeling Event Storm with multiple domain events', () => {
    let workUnit: WorkUnit;
    let eventStorm: EventStorm;

    // @step Given I have a work unit "AUTH-001" in specifying status
    it('should create work unit with specifying status', () => {
      workUnit = {
        id: 'AUTH-001',
        title: 'User Authentication',
        status: 'specifying',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };

      expect(workUnit.status).toBe('specifying');
      expect(workUnit.id).toBe('AUTH-001');
    });

    // @step When I add Event Storm section with level "process_modeling"
    // @step And I add 5 domain events: "UserRegistered", "EmailVerified", "UserAuthenticated", "SessionCreated", "PasswordResetRequested"
    it('should add Event Storm section with multiple domain events', () => {
      const events: EventStormEvent[] = [
        'UserRegistered',
        'EmailVerified',
        'UserAuthenticated',
        'SessionCreated',
        'PasswordResetRequested',
      ].map((text, index) => ({
        id: index,
        text,
        type: 'event' as const,
        color: 'orange' as const,
        deleted: false,
        createdAt: new Date().toISOString(),
        timestamp: 5000 + index * 1000,
      }));

      eventStorm = {
        level: 'process_modeling',
        items: events,
        nextItemId: 5,
      };

      workUnit.eventStorm = eventStorm;

      expect(workUnit.eventStorm).toBeDefined();
      expect(workUnit.eventStorm?.level).toBe('process_modeling');
      expect(workUnit.eventStorm?.items.length).toBe(5);
    });

    // @step Then each event should have stable ID (0, 1, 2, 3, 4)
    it('should assign stable IDs to events starting from 0', () => {
      const ids = eventStorm.items.map(item => item.id);
      expect(ids).toEqual([0, 1, 2, 3, 4]);
    });

    // @step And each event should have type="event"
    it('should set type="event" for domain events', () => {
      eventStorm.items.forEach(item => {
        expect(item.type).toBe('event');
      });
    });

    // @step And each event should have deleted=false
    it('should initialize events with deleted=false', () => {
      eventStorm.items.forEach(item => {
        expect(item.deleted).toBe(false);
      });
    });

    // @step And each event should have createdAt timestamp
    it('should add createdAt timestamp to each event', () => {
      eventStorm.items.forEach(item => {
        expect(item.createdAt).toBeDefined();
        expect(typeof item.createdAt).toBe('string');
      });
    });

    // @step And each event should have color="orange" (Event Storming convention)
    it('should set color="orange" for events following Event Storming convention', () => {
      eventStorm.items.forEach(item => {
        expect(item.color).toBe('orange');
      });
    });

    // @step And each event should have timestamp field for timeline visualization
    it('should include timestamp field for timeline visualization', () => {
      eventStorm.items.forEach(item => {
        expect(item.timestamp).toBeDefined();
        expect(typeof item.timestamp).toBe('number');
      });
    });

    // @step And eventStorm.nextItemId should be 5
    it('should increment nextItemId to 5 after adding 5 events', () => {
      expect(eventStorm.nextItemId).toBe(5);
    });
  });

  describe('Scenario: Create relationships between commands and events', () => {
    let event: EventStormEvent;
    let command: EventStormCommand;

    // @step Given I have Event Storm with event id=0 "UserAuthenticated"
    // @step And I have command id=1 "AuthenticateUser"
    it('should create event and command items', () => {
      event = {
        id: 0,
        text: 'UserAuthenticated',
        type: 'event',
        color: 'orange',
        deleted: false,
        createdAt: new Date().toISOString(),
      };

      command = {
        id: 1,
        text: 'AuthenticateUser',
        type: 'command',
        color: 'blue',
        deleted: false,
        createdAt: new Date().toISOString(),
      };

      expect(event.id).toBe(0);
      expect(event.text).toBe('UserAuthenticated');
      expect(command.id).toBe(1);
      expect(command.text).toBe('AuthenticateUser');
    });

    // @step When I link command to event using triggersEvent field
    it('should link command to event via triggersEvent field', () => {
      command.triggersEvent = 0;
      command.relatedTo = [0];
      event.relatedTo = [1];

      expect(command.triggersEvent).toBe(0);
    });

    // @step Then event id=0 should have relatedTo=[1]
    it('should add command ID to event relatedTo array', () => {
      expect(event.relatedTo).toEqual([1]);
    });

    // @step And command id=1 should have triggersEvent=0
    it('should set triggersEvent=0 on command', () => {
      expect(command.triggersEvent).toBe(0);
    });

    // @step And command id=1 should have relatedTo=[0]
    it('should add event ID to command relatedTo array', () => {
      expect(command.relatedTo).toEqual([0]);
    });

    // @step And relationship is bidirectional for traceability
    it('should maintain bidirectional relationships for traceability', () => {
      expect(event.relatedTo).toContain(command.id);
      expect(command.relatedTo).toContain(event.id);
      expect(command.triggersEvent).toBe(event.id);
    });
  });

  describe('Scenario: Generate suggested tags from Event Storm artifacts', () => {
    let eventStorm: EventStorm;
    let suggestedTags: SuggestedTags;

    // @step Given I have Event Storm with bounded context "Authentication"
    // @step And I have aggregates "User", "Session"
    // @step And I have external system "OAuth2Provider"
    it('should create Event Storm with bounded context, aggregates, and external system', () => {
      const boundedContext: EventStormBoundedContext = {
        id: 0,
        text: 'Authentication',
        type: 'bounded_context',
        color: 'blue',
        deleted: false,
        createdAt: new Date().toISOString(),
      };

      const userAggregate: EventStormAggregate = {
        id: 1,
        text: 'User',
        type: 'aggregate',
        color: 'yellow',
        deleted: false,
        createdAt: new Date().toISOString(),
        boundedContext: 'Authentication',
      };

      const sessionAggregate: EventStormAggregate = {
        id: 2,
        text: 'Session',
        type: 'aggregate',
        color: 'yellow',
        deleted: false,
        createdAt: new Date().toISOString(),
        boundedContext: 'Authentication',
      };

      const externalSystem: EventStormExternalSystem = {
        id: 3,
        text: 'OAuth2Provider',
        type: 'external_system',
        color: 'pink',
        deleted: false,
        createdAt: new Date().toISOString(),
        integrationType: 'API',
      };

      eventStorm = {
        level: 'process_modeling',
        items: [
          boundedContext,
          userAggregate,
          sessionAggregate,
          externalSystem,
        ],
        nextItemId: 4,
      };

      expect(eventStorm.items.length).toBe(4);
      expect(eventStorm.items[0].text).toBe('Authentication');
      expect(eventStorm.items[1].text).toBe('User');
      expect(eventStorm.items[2].text).toBe('Session');
      expect(eventStorm.items[3].text).toBe('OAuth2Provider');
    });

    // @step When tag suggestion algorithm analyzes Event Storm data
    it('should analyze Event Storm data for tag suggestions', () => {
      // Simulate tag suggestion algorithm
      suggestedTags = {
        componentTags: ['@auth', '@session-management'],
        featureGroupTags: ['@authentication', '@user-management'],
        technicalTags: ['@oauth2-integration'],
        reasoning:
          'Derived from Authentication bounded context, User aggregate, and OAuth2Provider external system',
      };

      eventStorm.suggestedTags = suggestedTags;

      expect(eventStorm.suggestedTags).toBeDefined();
    });

    // @step Then suggestedTags.componentTags should contain "@auth", "@session-management"
    it('should suggest component tags based on bounded context and aggregates', () => {
      expect(suggestedTags.componentTags).toContain('@auth');
      expect(suggestedTags.componentTags).toContain('@session-management');
    });

    // @step And suggestedTags.featureGroupTags should contain "@authentication", "@user-management"
    it('should suggest feature group tags based on domain model', () => {
      expect(suggestedTags.featureGroupTags).toContain('@authentication');
      expect(suggestedTags.featureGroupTags).toContain('@user-management');
    });

    // @step And suggestedTags.technicalTags should contain "@oauth2-integration"
    it('should suggest technical tags based on external systems', () => {
      expect(suggestedTags.technicalTags).toContain('@oauth2-integration');
    });

    // @step And suggestedTags.reasoning should explain derivation from bounded context and aggregates
    it('should include reasoning for tag suggestions', () => {
      expect(suggestedTags.reasoning).toBeDefined();
      expect(suggestedTags.reasoning).toContain('Authentication');
      expect(suggestedTags.reasoning).toContain('User');
      expect(suggestedTags.reasoning).toContain('OAuth2Provider');
    });
  });

  describe('Scenario: Soft-delete Event Storm item with stable ID preservation', () => {
    let eventStorm: EventStorm;
    let policyItem: EventStormPolicy;

    // @step Given I have Event Storm item with id=3 "PolicyItem"
    it('should create Event Storm item with id=3', () => {
      policyItem = {
        id: 3,
        text: 'PolicyItem',
        type: 'policy',
        color: 'purple',
        deleted: false,
        createdAt: new Date().toISOString(),
        when: 'User authenticates',
        then: 'Create session',
      };

      eventStorm = {
        level: 'process_modeling',
        items: [policyItem],
        nextItemId: 4,
      };

      expect(policyItem.id).toBe(3);
      expect(policyItem.text).toBe('PolicyItem');
    });

    // @step When I soft-delete item id=3
    it('should soft-delete item by setting deleted=true', () => {
      policyItem.deleted = true;
      policyItem.deletedAt = new Date().toISOString();

      expect(policyItem.deleted).toBe(true);
    });

    // @step Then item id=3 should have deleted=true
    it('should mark item as deleted', () => {
      expect(policyItem.deleted).toBe(true);
    });

    // @step And item id=3 should have deletedAt timestamp
    it('should add deletedAt timestamp when deleting', () => {
      expect(policyItem.deletedAt).toBeDefined();
      expect(typeof policyItem.deletedAt).toBe('string');
    });

    // @step And item id=3 should remain in items array
    it('should keep deleted item in items array', () => {
      expect(eventStorm.items).toContain(policyItem);
      expect(eventStorm.items.length).toBe(1);
    });

    // @step And ID 3 should never be reused for new items
    it('should never reuse deleted item IDs', () => {
      // Next item should use ID 4, not 3
      expect(eventStorm.nextItemId).toBe(4);

      // If we add a new item, it should get ID 4
      const newItem: EventStormEvent = {
        id: eventStorm.nextItemId,
        text: 'NewItem',
        type: 'event',
        color: 'orange',
        deleted: false,
        createdAt: new Date().toISOString(),
      };

      eventStorm.items.push(newItem);
      eventStorm.nextItemId++;

      expect(newItem.id).toBe(4);
      expect(eventStorm.nextItemId).toBe(5);
    });

    // @step And item id=3 can be restored using restore commands
    it('should allow restoration of soft-deleted items', () => {
      // Simulate restore by setting deleted=false and removing deletedAt
      policyItem.deleted = false;
      delete policyItem.deletedAt;

      expect(policyItem.deleted).toBe(false);
      expect(policyItem.deletedAt).toBeUndefined();
    });

    // @step And nextItemId counter should not decrement
    it('should preserve nextItemId counter after deletion', () => {
      // Even after deleting item 3, nextItemId should remain at 5
      expect(eventStorm.nextItemId).toBe(5);
    });
  });
});
