/**
 * Add domain event to Event Storm section of work unit
 * Coverage: EXMAP-006
 */

import { existsSync } from 'fs';
import { join } from 'path';
import type { Command } from 'commander';
import chalk from 'chalk';
import type { WorkUnitsData, EventStormEvent } from '../types';
import { fileManager } from '../utils/file-manager';

export interface AddDomainEventOptions {
  workUnitId: string;
  text: string;
  timestamp?: number;
  boundedContext?: string;
  cwd?: string;
}

export interface AddDomainEventResult {
  success: boolean;
  error?: string;
  eventId?: number;
}

/**
 * Add domain event to work unit's Event Storm section
 *
 * @param options - Command options
 * @returns Result with success status and event ID
 */
export async function addDomainEvent(
  options: AddDomainEventOptions
): Promise<AddDomainEventResult> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec', 'work-units.json');

  // Check if work-units.json exists
  if (!existsSync(workUnitsFile)) {
    return {
      success: false,
      error: 'spec/work-units.json not found. Run fspec init first.',
    };
  }

  try {
    // Read work units data
    const workUnitsData = await fileManager.readJSON<WorkUnitsData>(
      workUnitsFile,
      {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
        workUnits: {},
      }
    );

    // Validate work unit exists
    const workUnit = workUnitsData.workUnits[options.workUnitId];
    if (!workUnit) {
      return {
        success: false,
        error: `Work unit ${options.workUnitId} not found`,
      };
    }

    // Validate work unit is not in done/blocked state
    if (workUnit.status === 'done' || workUnit.status === 'blocked') {
      return {
        success: false,
        error: `Cannot add Event Storm items to work unit in ${workUnit.status} state`,
      };
    }

    // Initialize eventStorm section if not present
    if (!workUnit.eventStorm) {
      workUnit.eventStorm = {
        level: 'process_modeling',
        items: [],
        nextItemId: 0,
      };
    }

    // BUG-087: Check for duplicate events (case-insensitive, non-deleted only)
    const existingEvent = workUnit.eventStorm.items.find(
      item =>
        item.type === 'event' &&
        !item.deleted &&
        item.text.toLowerCase() === options.text.toLowerCase()
    );

    if (existingEvent) {
      return {
        success: false,
        error: `Event '${options.text}' already exists (ID: ${existingEvent.id})`,
      };
    }

    // Create domain event item
    const eventId = workUnit.eventStorm.nextItemId;
    const event: EventStormEvent = {
      id: eventId,
      type: 'event',
      color: 'orange',
      text: options.text,
      deleted: false,
      createdAt: new Date().toISOString(),
    };

    // Add optional fields
    if (options.timestamp !== undefined) {
      event.timestamp = options.timestamp;
    }
    if (options.boundedContext) {
      event.boundedContext = options.boundedContext;
    }

    // Add event to items array
    workUnit.eventStorm.items.push(event);
    workUnit.eventStorm.nextItemId++;

    // Update work unit timestamp
    workUnit.updatedAt = new Date().toISOString();

    // Update meta
    if (workUnitsData.meta) {
      workUnitsData.meta.lastUpdated = new Date().toISOString();
    }

    // Write updated data using transaction
    await fileManager.transaction(workUnitsFile, async data => {
      const typedData = data as WorkUnitsData;
      typedData.workUnits[options.workUnitId] = workUnit;
      if (workUnitsData.meta) {
        typedData.meta = workUnitsData.meta;
      }
    });

    return {
      success: true,
      eventId,
    };
  } catch (error: any) {
    return {
      success: false,
      error: error.message || 'Unknown error occurred',
    };
  }
}

export function registerAddDomainEventCommand(program: Command): void {
  program
    .command('add-domain-event')
    .description('Add domain event to Event Storm section of work unit')
    .argument('<workUnitId>', 'Work unit ID')
    .argument('<text>', 'Event text/name')
    .option('--timestamp <ms>', 'Timeline timestamp in milliseconds', parseInt)
    .option(
      '--bounded-context <context>',
      'Bounded context for domain association'
    )
    .action(async (workUnitId: string, text: string, options: any) => {
      try {
        const result = await addDomainEvent({
          workUnitId,
          text,
          timestamp: options.timestamp,
          boundedContext: options.boundedContext,
        });

        if (!result.success) {
          console.error(
            chalk.red('✗ Failed to add domain event:'),
            result.error
          );
          process.exit(1);
        }

        console.log(
          chalk.green(
            `✓ Added domain event "${text}" to ${workUnitId} (ID: ${result.eventId})`
          )
        );
      } catch (error: any) {
        console.error(
          chalk.red('✗ Failed to add domain event:'),
          error.message
        );
        process.exit(1);
      }
    });
}
