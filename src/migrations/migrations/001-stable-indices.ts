/**
 * Migration 001: Stable Indices with Soft Delete
 *
 * Converts string-based collections to object-based collections with stable IDs.
 * This migration prevents data loss during sequential AI removal operations.
 *
 * Collections migrated:
 * - rules: string[] → RuleItem[]
 * - examples: string[] → ExampleItem[]
 * - architectureNotes: string[] → ArchitectureNoteItem[]
 * - questions: QuestionItem[] (add id, deleted, createdAt, deletedAt? fields)
 */

import type { Migration, WorkUnitsData } from '../types';

const migration001: Migration = {
  version: '0.7.0',
  name: 'stable-indices',
  description: 'Convert string arrays to objects with stable IDs',

  up: (data: WorkUnitsData): WorkUnitsData => {
    const now = new Date().toISOString();

    for (const workUnitId in data.workUnits) {
      const workUnit = data.workUnits[workUnitId] as any;

      // Migrate rules: string[] → RuleItem[]
      if (workUnit.rules && Array.isArray(workUnit.rules)) {
        workUnit.rules = workUnit.rules.map((item: any, index: number) => {
          // Handle mixed format (partially migrated data)
          if (typeof item === 'object' && 'id' in item && 'deleted' in item) {
            return item; // Already migrated
          }
          // Convert string to RuleItem
          const text =
            typeof item === 'string'
              ? item
              : typeof item.text === 'string'
                ? item.text
                : String(item.text || item);
          return {
            id: index,
            text,
            deleted: false,
            createdAt: now,
          };
        });
        workUnit.nextRuleId = workUnit.rules.length;
      } else if (!workUnit.nextRuleId) {
        workUnit.nextRuleId = 0;
      }

      // Migrate examples: string[] → ExampleItem[]
      if (workUnit.examples && Array.isArray(workUnit.examples)) {
        workUnit.examples = workUnit.examples.map(
          (item: any, index: number) => {
            if (typeof item === 'object' && 'id' in item && 'deleted' in item) {
              return item;
            }
            const text =
              typeof item === 'string'
                ? item
                : typeof item.text === 'string'
                  ? item.text
                  : String(item.text || item);
            return {
              id: index,
              text,
              deleted: false,
              createdAt: now,
            };
          }
        );
        workUnit.nextExampleId = workUnit.examples.length;
      } else if (!workUnit.nextExampleId) {
        workUnit.nextExampleId = 0;
      }

      // Migrate architectureNotes: string[] → ArchitectureNoteItem[]
      if (
        workUnit.architectureNotes &&
        Array.isArray(workUnit.architectureNotes)
      ) {
        workUnit.architectureNotes = workUnit.architectureNotes.map(
          (item: any, index: number) => {
            if (typeof item === 'object' && 'id' in item && 'deleted' in item) {
              return item;
            }
            const text =
              typeof item === 'string'
                ? item
                : typeof item.text === 'string'
                  ? item.text
                  : String(item.text || item);
            return {
              id: index,
              text,
              deleted: false,
              createdAt: now,
            };
          }
        );
        workUnit.nextNoteId = workUnit.architectureNotes.length;
      } else if (!workUnit.nextNoteId) {
        workUnit.nextNoteId = 0;
      }

      // Align questions with ItemWithId structure
      if (workUnit.questions && Array.isArray(workUnit.questions)) {
        workUnit.questions = workUnit.questions.map((q: any, index: number) => {
          if ('id' in q && 'deleted' in q && 'createdAt' in q) {
            return q; // Already migrated
          }
          return {
            ...q,
            id: index,
            deleted: false,
            createdAt: now,
          };
        });
        workUnit.nextQuestionId = workUnit.questions.length;
      } else if (!workUnit.nextQuestionId) {
        workUnit.nextQuestionId = 0;
      }
    }

    return data;
  },

  down: (data: WorkUnitsData): WorkUnitsData => {
    // Rollback: convert objects back to strings (remove soft-deleted items)
    for (const workUnitId in data.workUnits) {
      const workUnit = data.workUnits[workUnitId] as any;

      if (workUnit.rules && Array.isArray(workUnit.rules)) {
        workUnit.rules = workUnit.rules
          .filter((r: any) => !r.deleted) // Remove soft-deleted items
          .map((r: any) => r.text);
        delete workUnit.nextRuleId;
      }

      if (workUnit.examples && Array.isArray(workUnit.examples)) {
        workUnit.examples = workUnit.examples
          .filter((e: any) => !e.deleted)
          .map((e: any) => e.text);
        delete workUnit.nextExampleId;
      }

      if (
        workUnit.architectureNotes &&
        Array.isArray(workUnit.architectureNotes)
      ) {
        workUnit.architectureNotes = workUnit.architectureNotes
          .filter((n: any) => !n.deleted)
          .map((n: any) => n.text);
        delete workUnit.nextNoteId;
      }

      if (workUnit.questions && Array.isArray(workUnit.questions)) {
        workUnit.questions = workUnit.questions
          .filter((q: any) => !q.deleted)
          .map(({ id, deleted, createdAt, deletedAt, ...rest }: any) => rest);
        delete workUnit.nextQuestionId;
      }
    }

    return data;
  },
};

export default migration001;
