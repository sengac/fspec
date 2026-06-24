/**
 * E2E: PROV-104 — /model view keyboard navigation against the real binary.
 *
 * Feature: spec/features/model-selector-keyboard-navigation-e2e.feature
 *
 * Spawns the real `fspec` binary in a PTY with HOME redirected to a temp
 * directory containing an `~/.fspec/fspec-config.json` that declares a
 * local-server (openai) profile carrying several `customModels`. Because a
 * profile with custom models is never marked unreachable
 * (`build_local_profile_sections` / MODEL-004), those models surface as
 * SELECTABLE rows in the `/model` view OFFLINE — no credentials, no network,
 * no models.dev catalog needed. This makes the row projection deterministic.
 *
 * This is the end-to-end regression net the PROV-104 unit tests could not
 * provide: those construct a ModelSelectorView with rows pre-injected, so
 * they never exercised the real async `list_providers()` data path. The user
 * symptom ("Up/Down does nothing at all" on a fresh build) only reproduces
 * when rows are populated through the live binary.
 */

import { test, expect } from '@microsoft/tui-test';
import { homedir, tmpdir } from 'os';
import { join } from 'path';
import { mkdtempSync, mkdirSync, writeFileSync } from 'fs';

// The runner (scripts/run-prov104-e2e.sh) stashes the huge codelet/target
// tree out of CWD so tui-test's blanket transform-copy does not try to
// duplicate 170G+ into .tui-test/cache. It copies the built binary to a
// stable location and points us at it via FSPEC_BIN.
const rustFspec =
  process.env.FSPEC_BIN ??
  join(homedir(), 'projects', 'fspec', 'codelet', 'target', 'debug', 'fspec');
const realWorkspace = join(homedir(), 'projects', 'fspec');

// Build a throwaway HOME with a local-server profile carrying enough custom
// models to overflow a short viewport (so the scroll-follow scenario has a
// tall list). Model ids are distinctive + sortable so they are easy to
// locate in the rendered text buffer.
const fakeHome = mkdtempSync(join(tmpdir(), 'prov104-home-'));
const fspecDir = join(fakeHome, '.fspec');
mkdirSync(fspecDir, { recursive: true });

const customModels = Array.from({ length: 20 }, (_, i) => {
  const n = String(i + 1).padStart(2, '0');
  return {
    id: `prov104-model-${n}`,
    displayName: `PROV104 Model ${n}`,
    contextWindow: 8192,
  };
});

writeFileSync(
  join(fspecDir, 'fspec-config.json'),
  JSON.stringify(
    {
      providers: {
        openai: {
          profiles: {
            'prov104-local': {
              baseUrl: 'http://127.0.0.1:59999/v1',
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
  rows: 30,
  columns: 160,
  env: { HOME: fakeHome, RUST_BACKTRACE: '1' },
});

function bufferToText(buffer: ReadonlyArray<ReadonlyArray<string>>): string {
  return buffer.map(row => row.join('').trimEnd()).join('\n');
}

// The selected SELECTABLE row is painted with a "▸" marker (rows_render.rs).
// Return the text of the line currently carrying that marker, or null.
function markedLine(text: string): string | null {
  for (const line of text.split('\n')) {
    if (line.includes('▸')) {
      return line.trim();
    }
  }
  return null;
}

async function openModelView(terminal: {
  getByText: (re: RegExp, o: object) => { toBeVisible: () => Promise<void> };
  submit: () => void;
  write: (s: string) => void;
}): Promise<void> {
  // Anchor on the board, walk to the DONE column, open the Work Agent.
  await expect(
    terminal.getByText(/BACKLOG/g, { strict: false, full: true })
  ).toBeVisible();
  terminal.submit(); // open Work Agent on the focused card
  await new Promise(r => setTimeout(r, 2000));
  // Open the full-screen model selector via the slash command.
  terminal.write('/model');
  await new Promise(r => setTimeout(r, 200));
  terminal.submit();
  await new Promise(r => setTimeout(r, 1500));
  // Wait for the provider list to finish loading (the title carries the count).
  await expect(
    terminal.getByText(/Select Model \(/g, { strict: false, full: true })
  ).toBeVisible({ timeout: 15_000 });
}

test('Filtering then Down moves the ▸ highlight to the next custom model', async ({
  terminal,
}) => {
  // @step Given the fspec binary is launched with FSPEC_USER_DIR pointing at a temp config containing a local-server profile with several custom models
  // @step And I open a Work Agent and submit "/model"
  await openModelView(terminal);

  // Filtering auto-expands every matching section, turning our custom models
  // into SELECTABLE rows (the only deterministic, network-free way to get
  // landable rows on a fresh install). Enter exits filter edit-mode but keeps
  // the filter applied with the cursor seeded on the first match.
  terminal.write('/');
  await new Promise(r => setTimeout(r, 200));
  terminal.write('PROV104 Model');
  await new Promise(r => setTimeout(r, 400));
  terminal.submit(); // Enter: leave filter edit-mode, keep filter + selection

  // @step And the model rows have rendered with at least two selectable models
  await new Promise(r => setTimeout(r, 400));
  const before = markedLine(bufferToText(terminal.getBuffer()));
  writeFileSync(
    '/tmp/prov104_filtered_before.txt',
    bufferToText(terminal.getBuffer())
  );

  // @step When I press the Down arrow
  terminal.keyDown();
  await new Promise(r => setTimeout(r, 300));
  const after = markedLine(bufferToText(terminal.getBuffer()));
  writeFileSync(
    '/tmp/prov104_filtered_after.txt',
    bufferToText(terminal.getBuffer())
  );

  // @step Then the highlighted model row moves to the next selectable model
  expect(before).not.toBeNull();
  expect(after).not.toBeNull();
  expect(after).not.toBe(before);
});

test('Fresh open with all sections collapsed lets Down reach a section to expand', async ({
  terminal,
}) => {
  // @step Given the fspec binary is launched with a temp config whose local-server profile carries custom models and no current model is set so every section opens collapsed
  // @step And I open the /model view and only collapsed provider headers are shown
  await openModelView(terminal);
  await expect(
    terminal.getByText(/openai: prov104-local/g, { strict: false, full: true })
  ).toBeVisible({ timeout: 15_000 });
  // The custom models must NOT be visible yet (section collapsed by default).
  await expect(
    terminal.getByText(/PROV104 Model 01/g, { strict: false, full: true })
  ).not.toBeVisible();

  // @step When I press the Down arrow to move the cursor onto the custom-model profile header and press the Right arrow to expand it
  // The profile section is the LAST row; clamp-down (TS parity) walks the
  // cursor through every header to it. With the broken header-skipping nav
  // the cursor stays frozen on row 0 and never reaches this section.
  for (let i = 0; i < 25; i++) {
    terminal.keyDown();
    await new Promise(r => setTimeout(r, 50));
  }
  terminal.keyRight(); // expand the section under the cursor
  await new Promise(r => setTimeout(r, 500));
  writeFileSync(
    '/tmp/prov104_collapsed_nav.txt',
    bufferToText(terminal.getBuffer())
  );

  // @step Then the profile's custom models become visible in the list
  await expect(
    terminal.getByText(/PROV104 Model 01/g, { strict: false, full: true })
  ).toBeVisible({ timeout: 5_000 });
});

test('Down to the bottom of a tall list keeps the selected model painted in the viewport', async ({
  terminal,
}) => {
  // @step Given the fspec binary is launched with FSPEC_USER_DIR pointing at a temp config whose local-server profile has more custom models than fit the viewport
  // @step And I open a Work Agent and submit "/model" and the model rows have rendered
  await openModelView(terminal);

  // Filter to surface the 20 custom models as a tall selectable list.
  terminal.write('/');
  await new Promise(r => setTimeout(r, 200));
  terminal.write('PROV104 Model');
  await new Promise(r => setTimeout(r, 400));
  terminal.submit();
  await new Promise(r => setTimeout(r, 400));

  // @step When I press the Down arrow repeatedly to the last model
  for (let i = 0; i < 25; i++) {
    terminal.keyDown();
    await new Promise(r => setTimeout(r, 50));
  }
  await new Promise(r => setTimeout(r, 300));

  // @step Then the last model row is visible in the rendered viewport and remains highlighted
  const text = bufferToText(terminal.getBuffer());
  writeFileSync('/tmp/prov104_bottom.txt', text);
  await expect(
    terminal.getByText(/PROV104 Model 20/g, { strict: false, full: true })
  ).toBeVisible();
  const marked = markedLine(text);
  expect(marked).not.toBeNull();
  expect(marked).toContain('PROV104 Model 20');
});
