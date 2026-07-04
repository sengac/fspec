/**
 * E2E: PROV-126 — the `/model` selector's CLOUD provider sections must match
 * the TypeScript reference (src/tui/services/cloudSectionBuilder.ts +
 * modelInitializationService.ts). This is the end-to-end regression net that
 * the PROV-125 unit tests could not provide: PROV-125 only proved the pure
 * slug→models.dev-key mapping (together→togetherai, moonshot→moonshotai) in
 * isolation. It never rendered the actual selector, so it could not catch the
 * SECTION-BUILDING defects that make "wrong models show up in the wrong areas":
 *
 *   #2 Empty/uncredentialed cloud sections are NOT dropped — Rust
 *      `list_providers` (handle_impl.rs) pushes every one of the 17 canonical
 *      providers as a header regardless of credentials, so the picker is full
 *      of dead "Provider (0 models)" rows. TS drops them
 *      (cloudSectionBuilder.ts `filter(s => s.hasCredentials)` +
 *      modelInitializationService.ts `filter(s => s.models.length > 0)`).
 *
 * This test seeds a throwaway HOME with:
 *   - ~/.fspec/cache/models.json  → a small models.dev catalog fixture whose
 *     KEYS are real models.dev provider ids (openai, anthropic, togetherai,
 *     moonshotai, cohere, mistral).
 *   - credentials via ENV for ONLY openai/anthropic/together/moonshot.
 *     cohere + mistral are present in the catalog but have NO credentials.
 *
 * Expected (TS parity):
 *   - Credentialed providers render POPULATED sections:
 *       "OpenAI API", "Anthropic", "Together AI (1 models)",
 *       "Moonshot (1 models)".
 *   - Uncredentialed providers are DROPPED entirely — there must be NO
 *       "Cohere (0 models)" and NO "Mistral AI (0 models)" header, and more
 *       generally NO cloud "(0 models)" header at all.
 *
 * The binary MUST be built with --features test-stub-provider so the stub
 * default model ("stub/canned") lets create_session succeed offline and the
 * Work Agent opens without a decline dialog (PROV-117 precedent).
 *
 * Full rendered buffers are written to /tmp/prov126_*.txt for diagnosis.
 */

import { test, expect } from '@microsoft/tui-test';
import { homedir, tmpdir } from 'os';
import { join } from 'path';
import {
  mkdtempSync,
  mkdirSync,
  writeFileSync,
  copyFileSync,
  readFileSync,
} from 'fs';

const rustFspec =
  process.env.FSPEC_BIN ??
  join(homedir(), 'projects', 'fspec', 'codelet', 'target', 'debug', 'fspec');
const realWorkspace = join(homedir(), 'projects', 'fspec');

// Throwaway HOME seeded with the models.dev cache fixture. No profiles: this
// exercises the pure CLOUD path (list_providers → cloud_model_entries).
const fakeHome = mkdtempSync(join(tmpdir(), 'prov126-home-'));
const cacheDir = join(fakeHome, '.fspec', 'cache');
mkdirSync(cacheDir, { recursive: true });
copyFileSync(
  join(realWorkspace, 'e2e', 'fixtures', 'prov126-models.json'),
  join(cacheDir, 'models.json')
);

test.use({
  program: {
    file: rustFspec,
    args: ['--workspace', realWorkspace],
  },
  rows: 44,
  columns: 180,
  env: {
    HOME: fakeHome,
    RUST_BACKTRACE: '1',
    // Credentialed cloud providers ONLY. cohere + mistral intentionally omitted.
    OPENAI_API_KEY: 'sk-openai-test-dummy',
    ANTHROPIC_API_KEY: 'sk-ant-test-dummy',
    TOGETHER_API_KEY: 'together-test-dummy',
    MOONSHOT_API_KEY: 'moonshot-test-dummy',
  },
});

function bufferToText(buffer: ReadonlyArray<ReadonlyArray<string>>): string {
  return buffer.map(row => row.join('').trimEnd()).join('\n');
}

interface Term {
  getByText: (re: RegExp, o: object) => { toBeVisible: () => Promise<void> };
  submit: () => void;
  write: (s: string) => void;
  keyDown: () => void;
  getBuffer: () => ReadonlyArray<ReadonlyArray<string>>;
}

async function openWorkAgent(terminal: Term): Promise<void> {
  await expect(
    terminal.getByText(/BACKLOG/g, { strict: false, full: true })
  ).toBeVisible();
  terminal.submit();
  await new Promise(r => setTimeout(r, 2500));
}

async function openModelView(terminal: Term): Promise<void> {
  terminal.write('/model');
  await new Promise(r => setTimeout(r, 250));
  terminal.submit();
  await new Promise(r => setTimeout(r, 1500));
  await expect(
    terminal.getByText(/Select Model \(/g, { strict: false, full: true })
  ).toBeVisible();
}

test('the /model selector drops empty/uncredentialed cloud provider sections (TS parity)', async ({
  terminal,
}) => {
  // @step Given the fspec binary is launched with a temp HOME whose models.dev cache lists openai, anthropic, togetherai, moonshotai, cohere and mistral
  // @step And credentials are configured only for openai, anthropic, together and moonshot
  await openWorkAgent(terminal);

  // @step When I open the /model view
  await openModelView(terminal);

  // Give the async list_providers() reload time to populate cloud sections,
  // then snapshot the full rendered buffer for diagnosis.
  await new Promise(r => setTimeout(r, 800));
  const text = bufferToText(terminal.getBuffer());
  writeFileSync('/tmp/prov126_model_view.txt', text);

  // @step Then the credentialed providers render populated sections
  // PROV-127: single-model headers pluralize as "(1 model)" (not "(1 models)").
  expect(text).toMatch(/Together AI \(1 model\)/);
  expect(text).toMatch(/Moonshot(?: AI)? \(1 model\)/);
  expect(text).toMatch(/OpenAI API \(\d+ models?\)/);
  expect(text).toMatch(/Anthropic \(\d+ models?\)/);

  // @step And no uncredentialed cloud provider section appears with zero models
  // cohere + mistral are in the catalog but have NO credentials, so under TS
  // parity they must be DROPPED, not shown as dead "(0 models)" headers.
  expect(text).not.toMatch(/Cohere \(0 models\)/);
  expect(text).not.toMatch(/Mistral(?: AI)? \(0 models\)/);

  // @step And no cloud provider header shows a zero-model count at all
  const zeroModelHeaders = text
    .split('\n')
    .filter(l => /\(0 models\)/.test(l) && !/unreachable/.test(l));
  // Surface any offending headers in the failure output. tui-test's bundled
  // `expect` does not accept the (value, message) two-arg form, so assert on a
  // descriptive value instead of passing a message argument.
  expect(zeroModelHeaders.join('\n')).toBe('');
  expect(zeroModelHeaders).toHaveLength(0);
});
