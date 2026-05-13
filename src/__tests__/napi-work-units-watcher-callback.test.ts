/**
 * Feature: spec/features/napi-work-units-watcher-callback.feature
 *
 * RPC-006 Scenario: NAPI startWorkUnitsWatcher callback continues to fire after the lift.
 *
 * After the watcher lift moves the cross-platform `notify` watcher out of
 * `codelet/napi/src/work_units_watcher.rs` into a pure-Rust module under
 * `codelet/core/src/work_units.rs`, the existing NAPI export
 * `startWorkUnitsWatcher` MUST continue to behave identically from the TS
 * side — same call shape, same callback invocations, same `WorkUnitInfo`
 * payload shape — so the existing TUI/Ink frontend keeps working
 * unchanged. This Vitest smoke codifies that invariant.
 */

import { describe, it, expect } from 'vitest';
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'fs';
import { tmpdir } from 'os';
import { join } from 'path';
import {
  startWorkUnitsWatcher,
  stopWorkUnitsWatcher,
} from '../../codelet/napi/index.js';

interface WorkUnitFixture {
  id: string;
  title: string;
  status: string;
}

function writeWorkUnitsJson(path: string, units: WorkUnitFixture[]): void {
  const workUnits: Record<string, Record<string, unknown>> = {};
  for (const u of units) {
    workUnits[u.id] = {
      id: u.id,
      title: u.title,
      type: 'story',
      status: u.status,
    };
  }
  writeFileSync(path, JSON.stringify({ workUnits }), 'utf8');
}

function waitForChunk(
  received: unknown[],
  predicate: (chunk: unknown) => boolean,
  timeoutMs: number
): Promise<unknown> {
  return new Promise((resolve, reject) => {
    const start = Date.now();
    const tick = (): void => {
      const found = received.find(predicate);
      if (found !== undefined) {
        resolve(found);
        return;
      }
      if (Date.now() - start >= timeoutMs) {
        reject(
          new Error(
            `timeout after ${timeoutMs}ms; received=${JSON.stringify(received)}`
          )
        );
        return;
      }
      setTimeout(tick, 25);
    };
    tick();
  });
}

interface WorkUnitsUpdateChunk {
  type: 'WorkUnitsUpdate';
  workUnits: Array<Record<string, unknown>>;
}

function isWorkUnitsUpdateChunk(chunk: unknown): chunk is WorkUnitsUpdateChunk {
  if (typeof chunk !== 'object' || chunk === null) {
    return false;
  }
  const record = chunk as Record<string, unknown>;
  return record.type === 'WorkUnitsUpdate' && Array.isArray(record.workUnits);
}

describe('Feature: NAPI startWorkUnitsWatcher callback continues to fire after the lift', () => {
  describe('Scenario: NAPI startWorkUnitsWatcher callback continues to fire after the lift', () => {
    it('invokes the callback with a WorkUnitInfo[] payload whose shape is unchanged from RPC-005', async () => {
      // @step Given the existing NAPI export startWorkUnitsWatcher implemented as a thin shim over codelet_core::work_units::WorkUnitsWatcher and a temporary workspace observed by that shim
      const root = mkdtempSync(join(tmpdir(), 'rpc006-watcher-'));
      try {
        mkdirSync(join(root, 'spec'), { recursive: true });
        const path = join(root, 'spec', 'work-units.json');
        writeWorkUnitsJson(path, [
          { id: 'AUTH-001', title: 'Login', status: 'done' },
        ]);

        const received: unknown[] = [];
        // napi-rs `ThreadsafeFunction<T>` defaults to `CalleeHandled = true`,
        // so the JS callback signature is `(err, data) => void`. The
        // `ts_arg_type` macro hint affects only the generated `.d.ts`
        // surface, not the runtime call shape — every other consumer of
        // the napi global-chunk callbacks does the same `(err, args)`
        // destructuring (see globalSessionStreamManager.ts). The chunk
        // we care about is therefore the SECOND argument.
        startWorkUnitsWatcher(root, ((
          err: Error | null,
          chunk: unknown
        ): void => {
          if (err !== null) {
            return;
          }
          received.push(chunk);
        }) as unknown as (chunk: unknown) => void);

        // Initial snapshot fires synchronously on start (legacy behaviour preserved).
        await waitForChunk(received, isWorkUnitsUpdateChunk, 2000);
        const initialCount = received.filter(isWorkUnitsUpdateChunk).length;

        // @step When the Vitest smoke test mutates spec/work-units.json once and waits up to two seconds on the registered ThreadsafeFunction callback
        writeWorkUnitsJson(path, [
          { id: 'AUTH-001', title: 'Login', status: 'done' },
          { id: 'AUTH-002', title: 'Reset password', status: 'implementing' },
        ]);

        await waitForChunk(
          received,
          c => isWorkUnitsUpdateChunk(c) && c.workUnits.length === 2,
          2000
        );

        // @step Then the callback is invoked at least once with a WorkUnitInfo[] payload whose shape (id, title, workType, status, description, estimate, epic) is unchanged from RPC-005
        const updateChunks = received.filter(isWorkUnitsUpdateChunk);
        expect(updateChunks.length).toBeGreaterThan(initialCount);
        const latest = updateChunks[updateChunks.length - 1];
        expect(latest.workUnits.length).toBe(2);
        const sample = latest.workUnits[0];
        expect(sample).toHaveProperty('id');
        expect(sample).toHaveProperty('title');
        expect(sample).toHaveProperty('workType');
        expect(sample).toHaveProperty('status');
        // Optional fields may be absent in the payload, but if present
        // they must be of the documented type.
        if ('description' in sample && sample.description !== undefined) {
          expect(typeof sample.description).toBe('string');
        }
        if ('estimate' in sample && sample.estimate !== undefined) {
          expect(typeof sample.estimate).toBe('number');
        }
        if ('epic' in sample && sample.epic !== undefined) {
          expect(typeof sample.epic).toBe('string');
        }
        // Verify the camelCase NAPI naming (workType, not work_type) is
        // preserved — the principal regression risk during the lift.
        expect(sample).not.toHaveProperty('work_type');
      } finally {
        try {
          stopWorkUnitsWatcher();
        } catch {
          /* ignore — cleanup */
        }
        rmSync(root, { recursive: true, force: true });
      }
    });
  });
});
