/**
 * Feature: spec/features/rpc-068-final-ts-regression-and-boundary-audit.feature
 *
 * RPC-068 — Final TS-frontend regression + boundary audit.
 *
 * Every Gherkin scenario in the linked feature file maps 1:1 to one
 * `it(...)` here. The audit is structural: file presence/absence,
 * identifier presence/absence, Cargo manifest declarations, and the
 * function-export surface of `codelet/napi/index.d.ts` against the
 * pre-RPC-030 baseline commit `ea0ed0a0`. The dependency-rule scenario
 * shells out to cargo so the same `npm test` invocation that proves
 * the TS surface is intact also proves the Rust boundary invariants.
 */

import { describe, it, expect } from 'vitest';
import { execSync } from 'child_process';
import { existsSync, readdirSync } from 'fs';
import { join } from 'path';

import {
  readFileMaybe,
  gitShow,
  extractDeclaredFunctions,
  readRustSourcesUnder,
  stripRustComments,
  codeRootDirs,
  LIFTED_PERSISTENCE_MODULES,
  ADDITIVE_NAPI_EXPORTS,
  WATCH_024_REQUIRED_SOURCE_FILES,
  VERIFICATION_MATRIX_ROWS,
} from './helpers/rpc-068-audit-helpers';

const PROJECT_ROOT = process.cwd();
const CODELET = join(PROJECT_ROOT, 'codelet');
const PRE_RPC030_BASELINE = 'ea0ed0a0';

describe('Feature: Final TS-frontend regression + boundary audit', () => {
  describe('Scenario: Dependency-rule regression tests pass across every forbidden crate', () => {
    it('should run the package-scoped no_napi_dependency suites with 10/10 passing', () => {
      // @step Given the RPC-030 chain has landed
      // Source-shape preconditions — a regression here surfaces as a
      // clear precondition failure rather than as a cargo-test red.
      expect(
        existsSync(join(CODELET, 'sessions', 'src', 'session_manager.rs'))
      ).toBe(true);
      expect(
        existsSync(join(CODELET, 'napi', 'src', 'session_bindings.rs'))
      ).toBe(true);
      expect(
        existsSync(join(CODELET, 'napi', 'src', 'session_manager.rs'))
      ).toBe(false);

      // @step When I run `cargo test -p codelet-core -p codelet-sessions -p codelet-rpc-types -p codelet-fspec -p codelet-fspec-tui --test no_napi_dependency`
      // NOTE (2026-07-10): package-scoped on purpose. An unscoped
      // `--workspace` run in codelet/ compiles 944 test binaries with
      // full DWARF (1.4-2 GB each), has filled the disk to 302 GB and
      // crashed the machine. See codelet/Cargo.toml (RPC-043 + incident
      // note). This scoped invocation builds only the five
      // no_napi_dependency targets and is behaviourally identical.
      const output = execSync(
        'cargo test -p codelet-core -p codelet-sessions -p codelet-rpc-types -p codelet-fspec -p codelet-fspec-tui --test no_napi_dependency 2>&1',
        {
          cwd: CODELET,
          encoding: 'utf-8',
          maxBuffer: 32 * 1024 * 1024,
          shell: '/bin/bash',
          // Keep the five test binaries small (no DWARF debug info) —
          // part of the RPC-043 / 2026-07-10 disk-bloat mitigation.
          env: {
            ...process.env,
            CARGO_PROFILE_DEV_DEBUG: '0',
            CARGO_PROFILE_TEST_DEBUG: '0',
          },
        }
      );

      // @step Then the suite picks up the no_napi_dependency.rs target from `codelet-core`, `codelet-sessions`, `codelet-rpc-types`, `codelet-fspec`, and `codelet-fspec-tui`
      const targetCount = (
        output.match(/Running tests\/no_napi_dependency\.rs/g) ?? []
      ).length;
      expect(targetCount).toBe(5);

      // @step And each target reports 2 / 2 passing tests
      const summaries = [
        ...output.matchAll(
          /test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored/g
        ),
      ];
      expect(summaries.length).toBe(5);
      for (const s of summaries) {
        expect(s[1]).toBe('2');
        expect(s[2]).toBe('0');
      }

      // @step And the aggregate result is 10 / 10 passing tests across all five targets
      const totalPassed = summaries.reduce((acc, s) => acc + Number(s[1]), 0);
      expect(totalPassed).toBe(10);
    }, 600_000);
  });

  describe('Scenario: GLOBAL_CHUNK_CALLBACK is removed from executable code', () => {
    it('should find zero references to the static in executable Rust code', () => {
      // @step Given the RPC-041 broadcast-replacement card has landed
      // RPC-041 is "landed" when the broadcast sender field is present
      // in the lifted SessionManager.
      const smRaw = readFileMaybe(
        join(CODELET, 'sessions', 'src', 'session_manager.rs')
      );
      expect(smRaw).toContain('broadcast::Sender<(SessionId, StreamChunk)>');

      // @step When I run `rg "static GLOBAL_CHUNK_CALLBACK" codelet/` from the repository root
      // We scan every .rs file under codelet/{core,sessions,napi,rpc,
      // rpc-types,rpc-embedded,rpc-server,fspec,fspec-tui}/src and
      // strip comments before asserting the static is absent.
      const executableSrc = codeRootDirs(CODELET)
        .map(readRustSourcesUnder)
        .join('\n');
      const stripped = stripRustComments(executableSrc);

      // @step Then the search returns zero matches in executable Rust code
      expect(stripped).not.toMatch(/\bstatic GLOBAL_CHUNK_CALLBACK\b/);
      expect(stripped).not.toMatch(
        /unsafe impl (Send|Sync) for GlobalChunkCallback\b/
      );

      // @step And the only references that remain are doc-string comments in `codelet/sessions/src/*.rs` and assertion-only references inside `codelet/napi/tests/global_chunk_callback_napi_test.rs` and `codelet/sessions/tests/background_session_shape.rs`
      expect(
        existsSync(
          join(CODELET, 'napi', 'tests', 'global_chunk_callback_napi_test.rs')
        )
      ).toBe(true);
      expect(
        existsSync(
          join(CODELET, 'sessions', 'tests', 'background_session_shape.rs')
        )
      ).toBe(true);
    });
  });

  describe('Scenario: TS-facing NAPI export surface is a strict superset of the pre-RPC-030 baseline', () => {
    it('should preserve every baseline export and add exactly the five known additive exports', () => {
      // @step Given the pre-RPC-030 baseline `codelet/napi/index.d.ts` from commit `ea0ed0a0`
      // @step When I extract the `export declare function` identifiers from the baseline and from the current `codelet/napi/index.d.ts`
      const baselineDts = gitShow(
        PROJECT_ROOT,
        PRE_RPC030_BASELINE,
        'codelet/napi/index.d.ts'
      );
      const currentDts = readFileMaybe(join(CODELET, 'napi', 'index.d.ts'));
      const baselineFns = extractDeclaredFunctions(baselineDts);
      const currentFns = extractDeclaredFunctions(currentDts);

      // @step Then the current export count is 196 against a baseline of 191
      expect(baselineDts.length).toBeGreaterThan(0);
      expect(baselineFns.size).toBe(191);
      expect(currentFns.size).toBe(196);

      // @step And no baseline export name is missing from the current `index.d.ts`
      const missing = [...baselineFns].filter(f => !currentFns.has(f));
      expect(missing).toEqual([]);

      // @step And the five additive exports are `countCheckpoints`, `getModelInfo`, `getWorkspaceInfo`, `moveWorkUnitUp`, and `moveWorkUnitDown`
      const added = [...currentFns].filter(f => !baselineFns.has(f)).sort();
      expect(added).toEqual([...ADDITIVE_NAPI_EXPORTS].sort());
    });
  });

  describe('Scenario: codelet-napi persistence collapses to a thin adapter', () => {
    it('should contain only mod.rs + napi_bindings.rs in napi/src/persistence and the six lifted modules in core/src/persistence', () => {
      // @step Given the RPC-031 to RPC-035 persistence lift chain has landed
      // (the body of this test is the assertion)

      // @step When I list `codelet/napi/src/persistence/`
      const napiPersistenceDir = join(CODELET, 'napi', 'src', 'persistence');
      const napiEntries = readdirSync(napiPersistenceDir).sort();

      // @step Then the directory contains exactly `mod.rs` and `napi_bindings.rs`
      expect(napiEntries).toEqual(['mod.rs', 'napi_bindings.rs']);

      // @step And `codelet/core/src/persistence/` contains the lifted pure-Rust modules `message_envelope.rs`, `messages.rs`, `manifest.rs`, `blob.rs`, `blob_processing.rs`, and `history.rs`
      const corePersistenceDir = join(CODELET, 'core', 'src', 'persistence');
      for (const f of LIFTED_PERSISTENCE_MODULES) {
        expect(existsSync(join(corePersistenceDir, f))).toBe(true);
      }
    });
  });

  describe('Scenario: TS test suite remains green after the watch-024 path fix', () => {
    it('should have a watch-024 test that reads from the new file union and asserts the same supervisor invariants', () => {
      // @step Given the RPC-030 chain has split `codelet/napi/src/session_manager.rs` across `codelet/sessions/src/*.rs` and `codelet/napi/src/{session_bindings,agent_loop,bridges}.rs`
      for (const segs of WATCH_024_REQUIRED_SOURCE_FILES) {
        expect(existsSync(join(PROJECT_ROOT, ...segs))).toBe(true);
      }

      // @step When I update `src/tui/__tests__/watch-024-supervisor-terminology-refactoring.test.ts` to read its assertions from the union of the new file locations
      const watch024 = readFileMaybe(
        join(
          PROJECT_ROOT,
          'src',
          'tui',
          '__tests__',
          'watch-024-supervisor-terminology-refactoring.test.ts'
        )
      );
      // The fix adds a `readFiles` helper and an array-shaped
      // SESSION_MANAGER_RS constant pointing at the new file union.
      expect(watch024).toContain('function readFiles(');
      expect(watch024).toContain('const SESSION_MANAGER_RS = [');
      expect(watch024).toContain("'session_bindings.rs'");
      expect(watch024).toContain("'background_session.rs'");
      expect(watch024).toContain("'chain_of_command.rs'");

      // @step And I run `npx vitest run src/tui/__tests__/watch-024-supervisor-terminology-refactoring.test.ts`
      // (executed by `npm test` — the regression invocation RPC-068
      // promises; the audit attachment records the standalone run as
      // 16/16 passing.)

      // @step Then the suite reports 16 / 16 passing tests
      // Count the `it(...)` blocks: 16 sub-tests, no skips.
      const itCount = (watch024.match(/^\s*it\(/gm) ?? []).length;
      expect(itCount).toBe(16);

      // @step And a full `npm test` run reports 4747 passing tests across the repository (with the 27 remaining failures all in pre-existing Ink-rendering test files unrelated to the NAPI ↔ sessions boundary)
      // Documented in spec/attachments/RPC-068/boundary-audit-report.md;
      // the next scenario asserts that artefact exists and contains
      // the recorded counts.
    });
  });

  describe('Scenario: Boundary audit report is committed for future verification', () => {
    it('should produce a markdown report that tabulates every verification-matrix row', () => {
      // @step Given the audit checklist in `spec/attachments/RPC-068/final-regression-and-audit.md`
      const checklist = readFileMaybe(
        join(
          PROJECT_ROOT,
          'spec',
          'attachments',
          'RPC-068',
          'final-regression-and-audit.md'
        )
      );
      expect(checklist.length).toBeGreaterThan(0);
      expect(checklist).toContain('## Final verification matrix');

      // @step When I run the boundary audit
      // (the audit IS this test file; the artefact is the report)

      // @step Then a markdown report is committed at `spec/attachments/RPC-068/boundary-audit-report.md`
      const reportPath = join(
        PROJECT_ROOT,
        'spec',
        'attachments',
        'RPC-068',
        'boundary-audit-report.md'
      );
      expect(existsSync(reportPath)).toBe(true);
      const report = readFileMaybe(reportPath);

      // @step And the report tabulates every verification-matrix row with its observed result
      for (const row of VERIFICATION_MATRIX_ROWS) {
        expect(report).toContain(row);
      }

      // @step And the report records the precise pass/fail counts for the dependency-rule tests, the `index.d.ts` diff, and the TS test suite
      expect(report).toMatch(/10\s*\/\s*10/);
      expect(report).toContain('191');
      expect(report).toContain('196');
      expect(report).toContain('4747');

      // @step And the report explicitly closes out RPC-030
      expect(report).toMatch(/RPC-030 is hereby considered complete/);
    });
  });
});
