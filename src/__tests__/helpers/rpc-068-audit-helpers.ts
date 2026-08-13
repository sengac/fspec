/**
 * Shared helpers and fixture data for the RPC-068 boundary-audit
 * assertions. Kept in a separate module so the main test file stays
 * under the 300-line guideline in CLAUDE.md while the scenarios in
 * `rpc-068-boundary-audit.test.ts` remain easy to read alongside the
 * Gherkin in `spec/features/rpc-068-final-ts-regression-and-boundary-audit.feature`.
 */

import { execSync } from 'child_process';
import { readFileSync, existsSync, readdirSync, statSync } from 'fs';
import { join } from 'path';

/**
 * Read a file, returning the empty string when it does not exist.
 */
export function readFileMaybe(path: string): string {
  try {
    return readFileSync(path, 'utf-8');
  } catch {
    return '';
  }
}

/**
 * Run `git show <rev>:<path>` and return its contents.
 */
export function gitShow(cwd: string, rev: string, path: string): string {
  return execSync(`git show ${rev}:${path}`, {
    cwd,
    encoding: 'utf-8',
    maxBuffer: 32 * 1024 * 1024,
  });
}

/**
 * Extract the names of every `export declare function NAME` line from
 * a TypeScript declaration file.
 */
export function extractDeclaredFunctions(dts: string): Set<string> {
  const re = /^export declare function ([a-zA-Z_][a-zA-Z0-9_]*)/gm;
  const names = new Set<string>();
  let m: RegExpExecArray | null;
  while ((m = re.exec(dts)) !== null) {
    names.add(m[1]);
  }
  return names;
}

/**
 * Recursively read all `.rs` files under a directory and concatenate
 * them. Missing directories are silently skipped.
 */
export function readRustSourcesUnder(srcDir: string): string {
  const parts: string[] = [];
  function walk(dir: string): void {
    if (!existsSync(dir)) {
      return;
    }
    for (const entry of readdirSync(dir)) {
      const full = join(dir, entry);
      const st = statSync(full);
      if (st.isDirectory()) {
        walk(full);
      } else if (entry.endsWith('.rs')) {
        parts.push(readFileMaybe(full));
      }
    }
  }
  walk(srcDir);
  return parts.join('\n');
}

/**
 * Strip Rust line and block comments so identifier scans only inspect
 * executable code. This is the same approach `codelet-test-helpers`
 * uses in the Rust-side dependency-rule tests.
 */
export function stripRustComments(src: string): string {
  // Remove block comments (non-nesting is fine for the limited domain
  // we scan — no source file in the workspace nests /* ... */).
  let out = src.replace(/\/\*[\s\S]*?\*\//g, '');
  // Remove line comments
  out = out.replace(/\/\/[^\n]*/g, '');
  return out;
}

/**
 * Every workspace crate `src/` directory that the dependency-rule
 * regression must cover. Used by the GLOBAL_CHUNK_CALLBACK scan to
 * concatenate executable Rust source and assert the static is absent.
 */
export function codeRootDirs(codelet: string): string[] {
  return [
    join(codelet, 'core', 'src'),
    join(codelet, 'sessions', 'src'),
    join(codelet, 'napi', 'src'),
    join(codelet, 'rpc', 'src'),
    join(codelet, 'rpc-types', 'src'),
    join(codelet, 'rpc-embedded', 'src'),
    join(codelet, 'rpc-server', 'src'),
    join(codelet, 'fspec', 'src'),
    join(codelet, 'fspec-tui', 'src'),
  ];
}

/**
 * The six pure-Rust persistence modules that RPC-031..RPC-035 lifted
 * out of `rust/napi/src/persistence/` into
 * `rust/core/src/persistence/`.
 */
export const LIFTED_PERSISTENCE_MODULES: readonly string[] = [
  'message_envelope.rs',
  'messages.rs',
  'manifest.rs',
  'blob.rs',
  'blob_processing.rs',
  'history.rs',
];

/**
 * The five additive `export declare function` identifiers that landed
 * in parallel with the RPC-030 chain. Every baseline export from
 * `ea0ed0a0` MUST still be present; these five are the only additions.
 */
export const ADDITIVE_NAPI_EXPORTS: readonly string[] = [
  'countCheckpoints',
  'getModelInfo',
  'getWorkspaceInfo',
  'moveWorkUnitDown',
  'moveWorkUnitUp',
];

/**
 * Files the post-RPC-030 watch-024 test must reach to assert
 * supervisor-terminology invariants across the lifted surface.
 */
export const WATCH_024_REQUIRED_SOURCE_FILES: ReadonlyArray<
  readonly [string, string, string, string]
> = [
  ['rust', 'sessions', 'src', 'session_manager.rs'],
  ['rust', 'sessions', 'src', 'background_session.rs'],
  ['rust', 'sessions', 'src', 'chain_of_command.rs'],
  ['rust', 'sessions', 'src', 'handle_impl.rs'],
  ['rust', 'napi', 'src', 'session_bindings.rs'],
  ['rust', 'napi', 'src', 'agent_loop.rs'],
  ['rust', 'napi', 'src', 'bridges.rs'],
];

/**
 * Verification-matrix row labels that the boundary-audit report must
 * tabulate, mirrored from `final-regression-and-audit.md`.
 */
export const VERIFICATION_MATRIX_ROWS: readonly string[] = [
  '`rust/napi/src/session_manager.rs`',
  '`rust/napi/src/session_bindings.rs`',
  '`rust/napi/src/persistence/` contents',
  '`rust/sessions/src/lib.rs`',
  '`rust/sessions/src/background_session.rs`',
  '`rust/sessions/src/session_manager.rs`',
  '`rust/core/src/persistence/`',
  '`GLOBAL_CHUNK_CALLBACK`',
  '`unsafe impl Send/Sync for GlobalChunkCallback`',
  '`rpc → napi`',
  '`fspec → napi`',
  '`fspec-tui → napi`',
  '`sessions → napi`',
  '`core → napi`',
  '`rpc-types → napi`',
];
