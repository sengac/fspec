/**
 * E2E: PROV-117 — pressing Enter on a model ROW selects/applies the model
 * and closes the /model view, against the real binary.
 *
 * Feature: spec/features/model-selector-enter-key-behavior.feature
 * Scenario: "End-to-end: pressing Enter on a model row selects the model
 *            and the selector closes against the real binary"
 *
 * Spawns the real `fspec` binary built with `--features test-stub-provider`
 * (so the deterministic stub LlmProvider is registered at boot and the
 * default model is pinned to "stub/canned"). The stub default model lets
 * `create_session` SUCCEED offline — without it the binary declines session
 * creation ("no default model set") and the /model selection path has no
 * `session_id`, so Enter on a model row would be a silent no-op regardless
 * of the row-selection bug. An ACTIVE session is therefore a precondition
 * for exercising the real select → apply → close flow.
 *
 * HOME is redirected to a temp directory whose `~/.fspec/fspec-config.json`
 * declares a local-server (openai) profile carrying `customModels`. Because
 * a profile with custom models is never marked unreachable
 * (`build_local_profile_sections` / MODEL-004), those models surface as
 * SELECTABLE rows in the `/model` view OFFLINE.
 *
 * IMPORTANT (root cause #1 — registry-normalization mismatch): at least one
 * custom model id carries a `-YYYYMMDD` date suffix
 * (`prov117-model-20240514`). The TS apply path strips that suffix via
 * `extractModelIdForRegistry`; the Rust select path emits the RAW id. This
 * model exercises whether the date-suffix id round-trips through
 * set_session_model + the `(current)` marker. Two suffix-free ids are also
 * present as controls.
 *
 * This is the end-to-end regression net the PROV-117 unit test
 * (`tests_enter_expand.rs::enter_on_model_row_emits_selection`) could not
 * provide: that asserts only the Emit contract with guards pre-satisfied
 * (Home forces has_selection, loaded_view seeds a session). It never
 * exercises the live async data path, the natural header-expand → Down
 * arrival on a model row, nor the downstream apply + chrome repaint + close.
 */

import { test, expect } from '@microsoft/tui-test';
import { homedir, tmpdir } from 'os';
import { join } from 'path';
import { mkdtempSync, mkdirSync, writeFileSync } from 'fs';

// The runner (scripts/run-tui-test.sh / run-prov104-e2e.sh) stashes the huge
// rust/target tree out of CWD and points us at the built binary via
// FSPEC_BIN. The binary MUST be built with --features test-stub-provider.
const rustFspec =
  process.env.FSPEC_BIN ??
  join(homedir(), 'projects', 'fspec', 'rust', 'target', 'debug', 'fspec');
const realWorkspace = join(homedir(), 'projects', 'fspec');

// Build a throwaway HOME with a local-server profile carrying custom models.
// At least one id carries a -YYYYMMDD date suffix (root cause #1); the others
// are suffix-free controls.
const fakeHome = mkdtempSync(join(tmpdir(), 'prov117-home-'));
const fspecDir = join(fakeHome, '.fspec');
mkdirSync(fspecDir, { recursive: true });

interface CustomModel {
  id: string;
  displayName: string;
  contextWindow: number;
}

const customModels: CustomModel[] = [
  // Root cause #1: date-suffixed id. TS strips -20240514 for the registry;
  // Rust emits it raw. If the apply path mishandles it, selection won't stick.
  {
    id: 'prov117-model-20240514',
    displayName: 'PROV117 Dated Model',
    contextWindow: 8192,
  },
  // Suffix-free controls.
  {
    id: 'prov117-plain-alpha',
    displayName: 'PROV117 Plain Alpha',
    contextWindow: 8192,
  },
  {
    id: 'prov117-plain-beta',
    displayName: 'PROV117 Plain Beta',
    contextWindow: 8192,
  },
];

writeFileSync(
  join(fspecDir, 'fspec-config.json'),
  JSON.stringify(
    {
      providers: {
        openai: {
          profiles: {
            'prov117-local': {
              baseUrl: 'http://127.0.0.1:59998/v1',
              apiKey: 'test-key',
              customModels,
            },
          },
        },
      },
    },
    null,
    2
  )
);

test.use({
  program: {
    file: rustFspec,
    args: ['--workspace', realWorkspace],
  },
  rows: 40,
  columns: 180,
  env: { HOME: fakeHome, RUST_BACKTRACE: '1' },
});

function bufferToText(buffer: ReadonlyArray<ReadonlyArray<string>>): string {
  return buffer.map(row => row.join('').trimEnd()).join('\n');
}

// The selected SELECTABLE row is painted with a "▸" marker (rows_render.rs).
function markedLine(text: string): string | null {
  for (const line of text.split('\n')) {
    if (line.includes('▸')) {
      return line.trim();
    }
  }
  return null;
}

interface Term {
  getByText: (re: RegExp, o: object) => { toBeVisible: () => Promise<void> };
  submit: () => void;
  write: (s: string) => void;
  keyDown: () => void;
  keyUp: () => void;
  keyRight: () => void;
  getBuffer: () => ReadonlyArray<ReadonlyArray<string>>;
}

async function openWorkAgent(terminal: Term): Promise<void> {
  // Anchor on the board, open the Work Agent on the focused card. With the
  // stub default model, this creates an ACTIVE session (no decline dialog).
  await expect(
    terminal.getByText(/BACKLOG/g, { strict: false, full: true })
  ).toBeVisible();
  terminal.submit(); // open Work Agent on the focused card
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

test('End-to-end: pressing Enter on a model row selects the model and the selector closes against the real binary', async ({
  terminal,
}) => {
  // @step Given the fspec binary is launched with a temp HOME config whose openai local-server profile carries custom models and the /model view is open in an active session
  await openWorkAgent(terminal);
  await openModelView(terminal);

  // Expand our custom-model profile section. Filtering auto-expands every
  // matching section, surfacing the custom models as SELECTABLE rows and
  // seeding the cursor on the first match (PROV-104 parity).
  terminal.write('/');
  await new Promise(r => setTimeout(r, 250));
  terminal.write('PROV117');
  await new Promise(r => setTimeout(r, 500));
  terminal.submit(); // leave filter edit-mode, keep filter + selection
  await new Promise(r => setTimeout(r, 500));

  // @step And I expand a provider section and move the cursor onto a selectable model row
  // Land the cursor on the date-suffixed model row (root cause #1). The
  // dated model is the FIRST selectable row in the section. After filtering,
  // the cursor is seeded near the first match; press Down once to register an
  // explicit navigation, then Up repeatedly to reach the top selectable row.
  terminal.keyDown();
  await new Promise(r => setTimeout(r, 250));
  for (let i = 0; i < 6; i++) {
    const marked = markedLine(bufferToText(terminal.getBuffer()));
    if (marked && marked.includes('PROV117 Dated Model')) {
      break;
    }
    terminal.keyUp();
    await new Promise(r => setTimeout(r, 250));
  }
  const beforeText = bufferToText(terminal.getBuffer());
  writeFileSync('/tmp/prov117_before_enter.txt', beforeText);
  const markedBefore = markedLine(beforeText);
  expect(markedBefore).not.toBeNull();
  expect(markedBefore).toContain('PROV117 Dated Model');

  // @step When I press Enter
  terminal.submit();
  await new Promise(r => setTimeout(r, 1500));
  const afterText = bufferToText(terminal.getBuffer());
  writeFileSync('/tmp/prov117_after_enter.txt', afterText);

  // @step Then the model selector view closes and returns to the agent view
  // The selector title must be gone (the view closed back to the agent).
  await expect(
    terminal.getByText(/Select Model \(/g, { strict: false, full: true })
  ).not.toBeVisible();

  // @step And the chosen model is applied so the agent view reflects it and reopening /model shows the (current) marker on that model
  // (a) Agent chrome shows the selected model in the session header.
  await expect(
    terminal.getByText(/PROV117 Dated Model|prov117-model-20240514/g, {
      strict: false,
      full: true,
    })
  ).toBeVisible();

  // (b) Reopen /model — the chosen row must carry the green (current) marker.
  // On reopen the current model's section is auto-expanded (RPC-342) and the
  // cursor is seeded on the current row, so no filtering is needed (and would
  // empty the list). Wait for the async list_providers() reload to populate.
  await openModelView(terminal);
  await expect(
    terminal.getByText(/openai: prov117-local/g, { strict: false, full: true })
  ).toBeVisible();
  await new Promise(r => setTimeout(r, 400));
  const reopenText = bufferToText(terminal.getBuffer());
  writeFileSync('/tmp/prov117_reopen.txt', reopenText);
  const currentLine = reopenText.split('\n').find(l => l.includes('(current)'));
  expect(currentLine).toBeDefined();
  expect(currentLine).toContain('PROV117 Dated Model');
});
