/**
 * E2E: PROV-129 — when the user is signed in with Codex (ChatGPT) OAuth, the
 * `/model` selector must synthesize a populated "Codex (ChatGPT)" section by
 * re-parenting the OpenAI cloud catalog (allowlist-filtered) and must NOT show
 * a standalone "OpenAI API" section. This is the end-to-end regression net for
 * discrepancies #1, #5 and #6 from spec/attachments/PROV-126/diagnosis.md:
 *
 *   #1 No Codex-section synthesis / OpenAI re-parenting.
 *   #5 No Codex allowlist filtering of the OpenAI models.
 *   #6 Codex OAuth login yields zero selectable models (both OpenAI API and
 *      Codex (ChatGPT) render empty).
 *
 * TS reference: src/tui/services/cloudSectionBuilder.ts extractCodexSection
 * (:191-237) + filterByCodexAllowlist (src/tui/services/codexAllowlistService.ts).
 *
 * This test seeds a throwaway HOME with:
 *   - ~/.fspec/cache/models.json → a models.dev catalog fixture whose OpenAI
 *     provider lists allowlisted (gpt-5.4, gpt-5.2-codex) AND excluded
 *     (gpt-5-mini, gpt-4o) models.
 *   - ~/.codex/auth.json → Codex OAuth tokens (NO OPENAI_API_KEY in the env),
 *     so the ONLY route to the OpenAI catalog is the Codex re-parenting path.
 *
 * Expected (TS parity):
 *   - A "Codex (ChatGPT) (2 models)" section is shown (gpt-5.4 + gpt-5.2-codex).
 *   - There is NO standalone "OpenAI API" header.
 *
 * The binary MUST be built with --features test-stub-provider so the stub
 * default model lets create_session succeed offline (PROV-117/PROV-126
 * precedent).
 *
 * Full rendered buffer is written to /tmp/prov129_model_view.txt for diagnosis.
 */

import { test, expect } from '@microsoft/tui-test';
import { homedir, tmpdir } from 'os';
import { join } from 'path';
import { mkdtempSync, mkdirSync, writeFileSync, copyFileSync } from 'fs';

const rustFspec =
  process.env.FSPEC_BIN ??
  join(homedir(), 'projects', 'fspec', 'rust', 'target', 'debug', 'fspec');
const realWorkspace = join(homedir(), 'projects', 'fspec');

// Throwaway HOME seeded with the models.dev cache fixture and a Codex OAuth
// auth.json. No OPENAI_API_KEY: this exercises the Codex re-parenting path.
const fakeHome = mkdtempSync(join(tmpdir(), 'prov129-home-'));
const cacheDir = join(fakeHome, '.fspec', 'cache');
mkdirSync(cacheDir, { recursive: true });
copyFileSync(
  join(realWorkspace, 'e2e', 'fixtures', 'prov129-codex-models.json'),
  join(cacheDir, 'models.json')
);
const codexDir = join(fakeHome, '.codex');
mkdirSync(codexDir, { recursive: true });
writeFileSync(
  join(codexDir, 'auth.json'),
  JSON.stringify({
    tokens: {
      id_token: 'id-tok',
      access_token: 'access-tok',
      refresh_token: 'refresh-tok',
      account_id: 'acct-123',
    },
  })
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
    // Anthropic credentialed so the picker is not otherwise empty; NO
    // OPENAI_API_KEY — the OpenAI catalog must arrive purely via Codex OAuth.
    ANTHROPIC_API_KEY: 'sk-ant-test-dummy',
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

test('the /model selector synthesizes a Codex (ChatGPT) section and hides the standalone OpenAI section under Codex OAuth (TS parity)', async ({
  terminal,
}) => {
  // @step Given I am signed in with a Codex (ChatGPT) OAuth credential
  // @step And no OPENAI_API_KEY is set in the environment
  // @step And the models.dev catalog offers OpenAI models including allowlisted ones
  await openWorkAgent(terminal);

  // @step When I open the model selector
  await openModelView(terminal);

  // Give the async list_providers() reload time to populate cloud sections,
  // then snapshot the full rendered buffer for diagnosis.
  await new Promise(r => setTimeout(r, 800));
  const text = bufferToText(terminal.getBuffer());
  writeFileSync('/tmp/prov129_model_view.txt', text);

  // @step Then a "Codex (ChatGPT)" section is shown with at least one selectable model
  // gpt-5.4 (p0) + gpt-5.2-codex (p3) survive the allowlist filter; gpt-5-mini
  // and gpt-4o are excluded.
  expect(text).toMatch(/Codex \(ChatGPT\) \(2 models\)/);

  // @step And no standalone "OpenAI API" section is shown
  expect(text).not.toMatch(/OpenAI API \(/);
});
