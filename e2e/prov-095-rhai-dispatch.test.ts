/**
 * E2E: PROV-095 — Rhai-scripted custom provider dispatch (session-creation path)
 *
 * Feature: spec/features/custom-provider-script-shadowing-builtin-providers.feature
 *
 * --------------------------------------------------------------------------
 * WHY THIS TEST EXISTS
 * --------------------------------------------------------------------------
 * The concrete regression the user hit:
 *
 *   "Failed to create session: Session registration failed: Failed to
 *    select model: [registry] Configuration error: Unknown provider:
 *    'claude-rhai'. Available providers: llama, nebius, poe, ..."
 *
 * That error fires at the NAPI boundary in
 * `create_session_with_id(id, model="claude-rhai/opus-4.7", ...)`
 * when the TUI boots with `tui.lastUsedModel = "claude-rhai/opus-4.7"`
 * (discovered custom provider pre-selected) and the user presses Enter
 * on the board to start a new agent session. Historically that path
 * ran the cloud branch (`select_model()`) which validates against the
 * models.dev registry and rejects anything not in the registry.
 *
 * The fix (PROV-096) adds an `is_custom_model` branch that calls
 * `set_model_direct` and bypasses the registry — same pattern as
 * profile models and codex models.
 *
 * --------------------------------------------------------------------------
 * WHY THE OLD VERSION OF THIS TEST DID NOT CATCH THE BUG
 * --------------------------------------------------------------------------
 * The previous revision of this file:
 *
 *   1. Created the session with whatever the ambient `lastUsedModel`
 *      happened to be (typically `anthropic/claude-opus-4-7`, NOT
 *      `claude-rhai/opus-4.7`) — so the registry check passed.
 *
 *   2. Only THEN opened `/model`, filtered to `claude-rhai`, and
 *      swapped — which routes through `sessionSetModelProfile`
 *      (correct path, has an `isCustomProviderSection()` bypass).
 *
 *   3. Did not seed `tui.lastUsedModel` to claude-rhai, so first-run
 *      boot would pick the default cloud model.
 *
 * That test was a happy-path-after-correction test, not a regression
 * test for the CREATION path. The user's bug reproduced on the
 * creation path. The test passed while the bug was present.
 *
 * --------------------------------------------------------------------------
 * WHAT THIS TEST DOES NOW
 * --------------------------------------------------------------------------
 * 1. Copies the REAL `claude_rhai.rhai` + `claude-rhai.json` out of
 *    the developer's `~/.fspec/providers/` into a fixture
 *    `$HOME/.fspec/providers/` directory.
 *
 * 2. Copies the REAL `~/.fspec/credentials/credentials.json` into the
 *    fixture so the Rust credential store resolves the anthropic API
 *    key without ambient-environment leakage. The extracted key is
 *    ALSO exported as `ANTHROPIC_API_KEY` in the child env because
 *    the Rhai custom provider reads its own `api_key_env_var` from
 *    the process environment (not from the credential store — custom
 *    providers are not in the `get_provider_env_vars` map).
 *
 * 3. Writes `$HOME/.fspec/fspec-config.json` with
 *    `tui.lastUsedModel = "claude-rhai/opus-4.7"` — this is what
 *    forces the TUI to pre-select the custom provider at boot time
 *    and thus exercise the bug path at session creation.
 *
 * 4. Launches `./dist/index.js` with `HOME` and `FSPEC_HOME`
 *    redirected at the fixture root. The TUI's
 *    `persistenceSetDataDirectory(getFspecUserDir())` call will
 *    therefore point the Rust credential store at the fixture.
 *
 * 5. On the board view, presses Enter to trigger the "Start New
 *    Agent?" dialog and confirms. This is the exact path that broke
 *    — no `/model` slash command, no post-creation switch.
 *
 * 6. Types "what is 3 + 2?" and submits.
 *
 * 7. Polls the visible terminal buffer for up to 45 seconds looking
 *    for a valid answer (the digit 5 in an assistant-reply context).
 *    Fails fast if any of an extended list of known error banners
 *    appear — including the exact "Unknown provider:" / "Failed to
 *    select model:" / "Session registration failed:" strings.
 *
 * --------------------------------------------------------------------------
 * GATES (no silent greens)
 * --------------------------------------------------------------------------
 * The test is SKIPPED (with a loud console.warn) if either of:
 *
 *   - The real Rhai provider fixture is not installed in
 *     `~/.fspec/providers/` on this machine.
 *   - `~/.fspec/credentials/credentials.json` does not contain an
 *     `anthropic.apiKey` value.
 *
 * Skipping is preferred over silently passing so that CI without
 * secrets cannot give a false green.
 */

import { test, expect } from '@microsoft/tui-test';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

interface StoredCredentials {
  version?: string;
  providers?: Record<string, { apiKey?: string } | undefined>;
}

/**
 * Read the anthropic API key out of `~/.fspec/credentials/credentials.json`.
 *
 * Returns `null` if the file is missing, malformed, or doesn't have a
 * populated `providers.anthropic.apiKey`. We deliberately do not fall
 * back to `process.env.ANTHROPIC_API_KEY` here — the whole point of
 * this test is to exercise the fspec credential store + custom
 * provider flow end-to-end; if no key is in the store we skip.
 */
function readAnthropicKeyFromStore(realCredFile: string): string | null {
  try {
    const raw = fs.readFileSync(realCredFile, 'utf8');
    const parsed = JSON.parse(raw) as StoredCredentials;
    const key = parsed.providers?.anthropic?.apiKey;
    if (typeof key === 'string' && key.length > 0) {
      return key;
    }
    return null;
  } catch {
    return null;
  }
}

// ---------------------------------------------------------------------------
// Fixture setup — must run at module load (test.use() reads env at import
// time). Keep everything deterministic so the first sub-run after `clean`
// gives the same answer as the last.
// ---------------------------------------------------------------------------

const REAL_FSPEC_DIR = path.join(os.homedir(), '.fspec');
const REAL_PROVIDERS_DIR = path.join(REAL_FSPEC_DIR, 'providers');
const REAL_CRED_FILE = path.join(
  REAL_FSPEC_DIR,
  'credentials',
  'credentials.json'
);
const REAL_JSON = path.join(REAL_PROVIDERS_DIR, 'claude-rhai.json');
const REAL_SCRIPT = path.join(REAL_PROVIDERS_DIR, 'claude_rhai.rhai');

const haveRealFixture = fs.existsSync(REAL_JSON) && fs.existsSync(REAL_SCRIPT);

const anthropicKey = fs.existsSync(REAL_CRED_FILE)
  ? readAnthropicKeyFromStore(REAL_CRED_FILE)
  : null;

const fixtureHome = fs.mkdtempSync(path.join(os.tmpdir(), 'fspec-prov095-'));
const fixtureFspecDir = path.join(fixtureHome, '.fspec');
const fixtureCredentialsDir = path.join(fixtureFspecDir, 'credentials');
const fixtureProvidersDir = path.join(fixtureFspecDir, 'providers');

fs.mkdirSync(fixtureCredentialsDir, { recursive: true });
fs.mkdirSync(fixtureProvidersDir, { recursive: true });

if (haveRealFixture) {
  // Copy the REAL script verbatim so line numbers and iteration
  // logic match the version the user actually runs.
  fs.copyFileSync(
    REAL_SCRIPT,
    path.join(fixtureProvidersDir, 'claude_rhai.rhai')
  );
  fs.copyFileSync(
    REAL_JSON,
    path.join(fixtureProvidersDir, 'claude-rhai.json')
  );
}

if (anthropicKey !== null) {
  // Write a minimal credentials.json the Rust store can read. Mirrors
  // the schema used by `~/.fspec/credentials/credentials.json`.
  const credsPayload = {
    version: '1',
    providers: {
      anthropic: {
        apiKey: anthropicKey,
        lastUpdated: new Date().toISOString(),
      },
    },
  };
  fs.writeFileSync(
    path.join(fixtureCredentialsDir, 'credentials.json'),
    JSON.stringify(credsPayload, null, 2),
    { mode: 0o600 }
  );
}

// Seed `tui.lastUsedModel` so the TUI boots with claude-rhai pre-selected.
// This is THE key difference vs. the old test: the session is created
// with the custom provider model string, exercising the bug path.
//
// PROV-100: Also seed `tui.defaultThinkingLevel = 3` (High). This forces
// the session to start with a non-Off thinking level so the
// session_manager's thinking-config path fires and the adaptive-
// thinking payload is threaded through `CustomProvider::create_rig_agent`
// and into the Rhai `build_request`. Without this seed, a fresh
// fixture would default to Off and the thinking-config assertion
// below would be impossible to hit (you can't observe a wire-up that
// was never engaged).
const FORCED_MODEL = 'claude-rhai/opus-4.7';
const fixtureConfig = {
  tui: { lastUsedModel: FORCED_MODEL, defaultThinkingLevel: 3 },
};
fs.writeFileSync(
  path.join(fixtureFspecDir, 'fspec-config.json'),
  JSON.stringify(fixtureConfig, null, 2)
);

// ---------------------------------------------------------------------------
// Error patterns that MUST NOT appear in the TUI. The list is deliberately
// exhaustive so the failure message can pinpoint exactly which regression
// fired.
// ---------------------------------------------------------------------------

const ERROR_PATTERNS: Array<{ label: string; regex: RegExp }> = [
  // The exact wording the user reported.
  {
    label: "Unknown provider: 'claude-rhai'",
    regex: /Unknown provider:\s*'?claude-rhai'?/i,
  },
  { label: 'Unknown provider (generic)', regex: /Unknown provider:/i },
  { label: 'Failed to select model', regex: /Failed to select model/i },
  {
    label: 'Session registration failed',
    regex: /Session registration failed/i,
  },
  {
    label: '[registry] Configuration error',
    regex: /\[registry\]\s+Configuration error/i,
  },
  { label: 'Failed to create session', regex: /Failed to create session/i },

  // Downstream agent-loop errors that the strengthened test must also
  // fail on, so we don't regress into the "dispatch reached the Rhai
  // arm but scripting broke" failure mode.
  //
  // NOTE: Patterns below are anchored to banner-style markers
  // (e.g. leading "✗", colon-suffix, "banner:" prefix) so they don't
  // false-positive on tool OUTPUT content. AstGrep can legitimately
  // return source lines that mention `ProviderError`, `api_error`, or
  // `streaming_error` when searching the fspec codebase — that is
  // normal tool output, not a banner.
  {
    label: 'Current provider is not Claude',
    regex: /Current provider is not Claude/i,
  },
  // Actual API Error banner renders as `✗ API Error: <message>`
  { label: 'API Error banner', regex: /✗\s*API Error:/ },
  // Streaming error banner — same shape.
  { label: 'Streaming error banner', regex: /✗\s*Streaming error:/i },
  // Agent-loop surfaces provider failures as `ProviderError:` (colon
  // suffix) on a banner line. The bare word `ProviderError` without
  // the colon is valid tool-output content (e.g. Rust type names) and
  // must not trip the matcher.
  { label: 'ProviderError banner', regex: /ProviderError:\s/ },
  { label: "script '...' failed", regex: /script '[^']+' failed/i },
  {
    label: 'For loop expects iterable type',
    regex: /For loop expects iterable/i,
  },
];

test.use({
  program: { file: './dist/index.js' },
  rows: 40,
  columns: 140,
  env: {
    ...process.env,
    // Redirect home so config / providers / credentials all resolve into
    // the fixture dir.
    HOME: fixtureHome,
    // FSPEC_HOME traditionally points at <base>/credentials. Discovery
    // honours this and uses its parent as the .fspec root.
    FSPEC_HOME: fixtureCredentialsDir,
    // The Rhai provider reads `ANTHROPIC_API_KEY` from the process env
    // at request-build time (via `api_key_env_var`). The resolver does
    // NOT set this automatically for custom providers (the env-var map
    // is hardcoded to built-ins), so we forward it explicitly.
    ANTHROPIC_API_KEY: anthropicKey ?? '',
    // Force colour off so regex matches against the buffer don't trip
    // on ANSI colour escapes.
    FORCE_COLOR: '0',
    NO_COLOR: '1',
    // Turn up both TS and Rust logging so we can see the full
    // dispatch path in fixtureHome/.fspec/fspec.log.
    FSPEC_LOG_LEVEL: 'debug',
    FSPEC_RUST_LOG_LEVEL:
      'debug,rig=debug,codelet_providers=debug,codelet_napi=debug,codelet_cli=debug',
  },
});

const shouldRun = haveRealFixture && anthropicKey !== null;

if (!shouldRun) {
  const reasons: string[] = [];
  if (!haveRealFixture) {
    reasons.push(`real Rhai fixture not found at ${REAL_PROVIDERS_DIR}`);
  }
  if (anthropicKey === null) {
    reasons.push(
      `no anthropic.apiKey in ${REAL_CRED_FILE} (run 'fspec credentials set anthropic')`
    );
  }

  console.warn(
    `[PROV-095 e2e] SKIPPED — ${reasons.join('; ')}. Cannot verify a real provider response.`
  );
}

test.when(
  shouldRun,
  'PROV-095: pre-selected Rhai-scripted custom provider forwards tool catalogue end-to-end (WebSearch, LS, AstGrep)',
  async ({ terminal }) => {
    // @step Given a claude-rhai provider fixture is installed in $HOME/.fspec/providers
    // @step And $HOME/.fspec/fspec-config.json seeds tui.lastUsedModel = "claude-rhai/opus-4.7"
    // @step And $HOME/.fspec/credentials/credentials.json contains an anthropic apiKey
    //       (all three asserted by the test.when gate above)

    // @step When the board renders
    await expect(
      terminal.getByText(/backlog/gi, { strict: false })
    ).toBeVisible();

    // Before doing anything, scan the initial view for error banners —
    // some failure modes surface on first render (e.g. if model-init
    // itself chokes on the custom provider).
    {
      const initial = terminal
        .getViewableBuffer()
        .map((row: string[]) => row.join(''))
        .join('\n');
      for (const pattern of ERROR_PATTERNS) {
        if (pattern.regex.test(initial)) {
          throw new Error(
            `PROV-095 FAILURE (pre-session): "${pattern.label}" on initial board render.\n\nScreen:\n${initial}`
          );
        }
      }
    }

    // @step And I press Enter to trigger the "Work on …?" / "Start New Agent?" dialog
    //
    // We DO NOT rely on tui-test's `getByText().toBeVisible()` matcher
    // here because its regex semantics over the multi-row viewable
    // buffer have proven flaky for text embedded in box-drawing. We
    // drive the confirmation purely from direct buffer scans.
    const readScreen = (): string =>
      terminal
        .getViewableBuffer()
        .map((row: string[]) => row.join(''))
        .join('\n');

    const dialogRegex = /Work on [A-Z0-9]+-\d+\?|Start New Agent\?/i;

    // Step 1: wait up to 10s for the dialog to appear after pressing Enter.
    terminal.submit();
    const dialogAppeared = await (async () => {
      const deadline = Date.now() + 10_000;
      while (Date.now() < deadline) {
        if (dialogRegex.test(readScreen())) return true;
        await new Promise(r => setTimeout(r, 200));
      }
      return false;
    })();

    let dialogConfirmed = false;

    if (dialogAppeared) {
      // Step 2: known bug — dialog's useInputCompat handler registers
      // in a useEffect (after commit). The first Enter that opens the
      // dialog is processed by BoardView; the SECOND Enter can hit a
      // render-timing window where no active handler claims it
      // (BoardView handler just deactivated via ref; dialog handler
      // not yet registered). Keep pressing Enter until the dialog
      // text disappears.
      const dismissDeadline = Date.now() + 15_000;
      while (Date.now() < dismissDeadline) {
        if (!dialogRegex.test(readScreen())) {
          dialogConfirmed = true;
          break;
        }
        terminal.submit();
        await new Promise(r => setTimeout(r, 500));
      }
      if (!dialogConfirmed) {
        throw new Error(
          `PROV-095 FAILURE (dialog): could not dismiss the "Work on …?" dialog within 15s of repeated Enter presses.\n\nScreen:\n${readScreen()}`
        );
      }
    }
    // If dialogAppeared is false, the session auto-attached (uncommon
    // but supported in some builds). Proceed to the settle/error-scan.

    // @step And the session view settles
    // Poll for up to 10s waiting for either the agent view (prompt area)
    // or an error banner, rather than a blind sleep. This is critical
    // because the bug surfaces RIGHT HERE — session creation fails and
    // an error banner is rendered before we ever type a prompt.
    {
      const settleDeadline = Date.now() + 10_000;
      while (Date.now() < settleDeadline) {
        const buffer = terminal
          .getViewableBuffer()
          .map((row: string[]) => row.join(''))
          .join('\n');
        for (const pattern of ERROR_PATTERNS) {
          if (pattern.regex.test(buffer)) {
            throw new Error(
              `PROV-095 FAILURE (session-creation): "${pattern.label}" appeared${dialogConfirmed ? ' after confirming "Start New Agent?"' : ' after pressing Enter on the board'} — this is the exact regression the fix targeted.\n\nMatched pattern: ${pattern.regex}\n\nScreen:\n${buffer}`
            );
          }
        }
        // Heuristic "agent view is ready" marker: a prompt area or
        // a status line mentioning the model. We don't block on it
        // strictly — if it never appears we'll still detect the
        // failure via the error-banner poll or the answer-poll below.
        if (/claude|opus|rhai|Ask|Type a message/i.test(buffer)) {
          break;
        }
        await new Promise(r => setTimeout(r, 250));
      }
    }

    // @step When I send a prompt that FORCES tool use through the Rhai
    //        dispatch: WebSearch, then LS, then AstGrep.
    //
    // The historical test here asked "what is 3 + 2?" which Claude
    // answers purely from its weights — no tools involved — so it
    // never exercised the tool-definition forwarding path. A
    // regression where `tools_for_rhai()` returns `Vec::new()` would
    // pass that test. This version asks for concrete tool work so the
    // agent must call each of the three target tools to complete it.
    //
    // Typing one char at a time with small gaps mirrors user keystrokes
    // and avoids the useInput handler race described further down.
    await new Promise(r => setTimeout(r, 3000));

    const prompt =
      'Using the tools available, do these three tasks in order and report what each tool returned: ' +
      '1) WebSearch for "rust programming language wikipedia". ' +
      '2) LS the directory "codelet/providers/src/custom" (relative to the cwd) so I can see its contents. ' +
      '3) AstGrep for the pattern `fn $NAME($$$ARGS) -> $RET { $$$BODY }` with language "rust" in path "codelet/providers/src/custom". ' +
      'After each tool call, state in one sentence which tool you used. ' +
      // PROV-100: Explicitly ask for `ultrathink` so the
      // thinking-level detector (see `thinking_level_detection.rs`,
      // HIGH_PATTERNS) bumps the effective level to High. Without a
      // thinking keyword the session defaults to Off and the
      // `thinking_config` we assert on never gets wired through the
      // Rhai dispatch. Avoid the word "briefly" — it's in
      // `DISABLE_KEYWORDS` and would force Off regardless of the
      // session's base level.
      'ultrathink while doing this.';
    for (const ch of prompt) {
      terminal.write(ch);
      await new Promise(r => setTimeout(r, 30));
    }
    // Let the input commit visually before we press Enter. Dump a
    // diagnostic snapshot so the e2e log shows whether typing actually
    // reached the input field.
    await new Promise(r => setTimeout(r, 800));
    const preSubmitScreen = terminal
      .getViewableBuffer()
      .map((row: string[]) => row.join('').trimEnd())
      .join('\n');

    console.log(
      `[PROV-095 e2e] pre-submit screen (chars typed: ${prompt.length}):\n${preSubmitScreen}`
    );
    terminal.submit();

    // @step Then within 180 seconds the TUI must render each of the
    //        three tool calls, and no error banners must appear.
    //
    // A tool call renders as `● ToolName(args)` in the Claude Code
    // style formatter (see `formatToolHeader` in AgentView.tsx). We
    // require that exact marker so the tool-name words appearing in
    // the user's prompt echo don't count as false positives.
    //
    // We ALSO cross-check the fspec.log for:
    //   (a) at least one `CompletionModel::stream ENTER` line
    //       (proving the Rhai dispatch actually ran)
    //   (b) tool_count > 0 on those lines (proving the tool catalogue
    //       was forwarded — this is the regression the fix targets)
    //   (c) `[rhai-dispatch] stream yielding ToolCallComplete` entries
    //       for each expected tool name (proving the tool calls
    //       actually made the full round trip).
    //
    // The log check is authoritative — the TUI buffer can scroll
    // older tool calls out of view on small terminals.
    const deadline = Date.now() + 180_000;
    const expectedTools = ['WebSearch', 'LS', 'AstGrep'];
    const seenToolsInTui = new Set<string>();
    const seenToolsInLog = new Set<string>();
    let lastScreen = '';
    let firstError: { label: string; regex: RegExp } | null = null;
    let sawStreamEnter = false;
    let sawZeroToolCount: string | null = null;
    const logPath = path.join(fixtureFspecDir, 'fspec.log');

    const readLog = (): string => {
      try {
        return fs.readFileSync(logPath, 'utf8');
      } catch {
        return '';
      }
    };

    while (Date.now() < deadline) {
      const rows = terminal.getViewableBuffer();
      lastScreen = rows
        .map((row: string[]) => row.join('').trimEnd())
        .join('\n');

      for (const pattern of ERROR_PATTERNS) {
        if (pattern.regex.test(lastScreen)) {
          firstError = pattern;
          break;
        }
      }
      if (firstError !== null) {
        break;
      }

      // TUI-side: require the `● ToolName(` marker, not a bare word.
      for (const toolName of expectedTools) {
        const re = new RegExp(`●\\s*${toolName}\\s*\\(`);
        if (re.test(lastScreen)) {
          seenToolsInTui.add(toolName);
        }
      }

      // Log-side: authoritative evidence that the Rhai dispatch ran
      // and the tool actually completed a round trip.
      const log = readLog();
      if (log.includes('CompletionModel::stream ENTER')) {
        sawStreamEnter = true;
        // tool_count=0 on ANY stream ENTER line is an immediate fail.
        const lines = log
          .split('\n')
          .filter(l => l.includes('CompletionModel::stream ENTER'));
        for (const line of lines) {
          if (/tool_count\s*=\s*0(?:\b|,)/.test(line)) {
            sawZeroToolCount = line;
            break;
          }
        }
        if (sawZeroToolCount !== null) {
          break;
        }
      }
      for (const toolName of expectedTools) {
        // The Rhai stream loop logs each ToolCallComplete with the
        // tool_name inlined into the message (see
        // `codelet/providers/src/custom/rig_model.rs` — the
        // `tool_name={name}` format specifier ensures the NAPI log
        // bridge surfaces it, since tracing's structured fields get
        // stripped on the way to the TS log sink).
        const re = new RegExp(
          `\\[rhai-dispatch\\][^\\n]*ToolCallComplete[^\\n]*tool_name=${toolName}\\b`
        );
        if (re.test(log)) {
          seenToolsInLog.add(toolName);
        }
      }

      if (sawStreamEnter && seenToolsInLog.size === expectedTools.length) {
        break;
      }

      await new Promise(r => setTimeout(r, 500));
    }

    if (firstError !== null) {
      throw new Error(
        `PROV-095 FAILURE (streaming): "${firstError.label}" appeared after sending the tool-use prompt.\n\nMatched pattern: ${firstError.regex}\n\nScreen:\n${lastScreen}`
      );
    }

    if (!sawStreamEnter) {
      throw new Error(
        `PROV-095 FAILURE: no \`CompletionModel::stream ENTER\` log lines found within 180s of sending the prompt — the Rhai dispatch did not run at all.\n\nLog path: ${logPath}\nLast screen:\n${lastScreen}`
      );
    }

    if (sawZeroToolCount !== null) {
      throw new Error(
        `PROV-095 FAILURE: \`CompletionModel::stream ENTER\` logged tool_count=0 — \`tools_for_rhai()\` regressed to returning an empty Vec.\n\nOffending log line:\n${sawZeroToolCount}`
      );
    }

    // PROV-098: ensure the claude-rhai agent exposes the FULL fspec tool
    // surface — not just the nine baseline preset tools. The baseline
    // preset is Read/Write/Edit/Bash/Grep/Glob/LS/AstGrep/WebSearch; the
    // infrastructure tools attached as native rig tools on top are
    // AstGrepRefactor/SessionSearch/GraphSearch/inject_summary/
    // DeepSearch/AgentManager/request_user_input/Schedule/ConnectMCP.
    // If any of those are missing from `tool_names=[…]` on any stream
    // ENTER line, claude-rhai has regressed to a partial tool surface.
    const REQUIRED_TOOLS_IN_CATALOGUE = [
      'Read',
      'Write',
      'Edit',
      'Bash',
      'Grep',
      'Glob',
      'AstGrep',
      'LS',
      'WebSearch',
      'AstGrepRefactor',
      'SessionSearch',
      'GraphSearch',
      'inject_summary',
      'DeepSearch',
      'AgentManager',
      'request_user_input',
      'Schedule',
      'ConnectMCP',
      // PROV-099: Fspec + Bridge are facade-wrapped tools attached to
      // the Rhai custom provider so a `claude-rhai` agent has full
      // parity with a native `claude` agent. Their absence is how the
      // screenshot bug manifested ("Fspec tool not available").
      'Fspec',
      'Bridge',
    ];
    {
      const log = readLog();
      const streamEnterLines = log
        .split('\n')
        .filter(l => l.includes('CompletionModel::stream ENTER'));
      // Collect every tool name mentioned across all stream ENTER lines
      // by parsing the `tool_names=[…]` bracket list.
      const seenInCatalogue = new Set<string>();
      for (const line of streamEnterLines) {
        const match = /tool_names=\[([^\]]*)\]/.exec(line);
        if (match === null) continue;
        const inner = match[1];
        // Entries look like `"Read", "Write", …`. Extract the quoted
        // strings; fall back to a simple split on comma if the match
        // form ever changes.
        const names = Array.from(inner.matchAll(/"([^"]+)"/g)).map(m => m[1]);
        for (const n of names) seenInCatalogue.add(n);
      }
      const missing = REQUIRED_TOOLS_IN_CATALOGUE.filter(
        t => !seenInCatalogue.has(t)
      );
      if (missing.length > 0) {
        throw new Error(
          `PROV-095 FAILURE: claude-rhai tool catalogue is missing required tools ${JSON.stringify(missing)}.\n\nObserved: ${JSON.stringify([...seenInCatalogue])}\n\nStream ENTER lines (last 3):\n${streamEnterLines.slice(-3).join('\n')}`
        );
      }

      // PROV-100: Every `CompletionModel::stream ENTER` line must carry
      // a non-`none` `thinking_config=…` blob, proving the TUI thinking
      // level (default High in the fixture) flowed through the NAPI
      // layer → CustomProvider::create_rig_agent →
      // additional_params → rig CompletionRequest → Rhai
      // request.thinking_config. A value of `none` means thinking was
      // never wired up for the custom provider at all.
      const thinkingLines = streamEnterLines
        .map(l => /thinking_config=([^\s]+)/.exec(l))
        .filter((m): m is RegExpExecArray => m !== null)
        .map(m => m[1]);
      if (thinkingLines.length === 0) {
        throw new Error(
          `PROV-100 FAILURE: no stream ENTER line carried a \`thinking_config=\` field at all. The rig_model logging regressed or the Rhai dispatch didn't run.\n\nStream ENTER lines (last 3):\n${streamEnterLines.slice(-3).join('\n')}`
        );
      }
      const hasRealThinking = thinkingLines.some(
        t => t !== 'none' && t.includes('"thinking"')
      );
      if (!hasRealThinking) {
        throw new Error(
          `PROV-100 FAILURE: claude-rhai never received a non-empty thinking_config from the session's thinking level. Observed values: ${JSON.stringify(thinkingLines)}.\n\nStream ENTER lines (last 3):\n${streamEnterLines.slice(-3).join('\n')}`
        );
      }

      // And the Rhai script's build_request log must show the
      // thinking block was attached to the body — this proves the
      // full chain lands in the Anthropic API payload.
      const rhaiAttachedThinking = log.includes(
        '[claude-rhai.rhai] build_request: attached thinking config'
      );
      if (!rhaiAttachedThinking) {
        throw new Error(
          `PROV-100 FAILURE: Rhai build_request never attached thinking config to the body. Either the rhai script at $HOME/.fspec/providers/claude_rhai.rhai is stale, or request.thinking_config is a Rhai unit when it should be a map.\n\nStream ENTER lines (last 3):\n${streamEnterLines.slice(-3).join('\n')}`
        );
      }
    }

    const missingInLog = expectedTools.filter(t => !seenToolsInLog.has(t));
    if (missingInLog.length > 0) {
      const streamEnterLines = readLog()
        .split('\n')
        .filter(l => l.includes('CompletionModel::stream ENTER'))
        .slice(-3);
      const rhaiDispatchLines = readLog()
        .split('\n')
        .filter(l => l.includes('[rhai-dispatch]'))
        .slice(-40);
      throw new Error(
        `PROV-095 FAILURE: tools ${JSON.stringify(missingInLog)} never completed a tool call in the Rhai dispatch log within 180s.\n\nStream ENTER lines (last 3):\n${streamEnterLines.join('\n')}\n\nRhai-dispatch lines (last 40):\n${rhaiDispatchLines.join('\n')}\n\nLast screen:\n${lastScreen}`
      );
    }

    const missingInTui = expectedTools.filter(t => !seenToolsInTui.has(t));
    if (missingInTui.length > 0) {
      // Non-fatal warning only — logs are authoritative and the TUI
      // viewport can legitimately scroll tool renders out of view on
      // small buffers. We still want to see if this happens so we log
      // it without failing the test.
      console.warn(
        `[PROV-095 e2e] WARNING: expected TUI tool renders ${JSON.stringify(missingInTui)} scrolled out of view, but they DID complete in the Rhai dispatch log. Treating as pass.`
      );
    }

    // Belt-and-braces: assert no residual error banner on the final
    // screen. This catches races where the answer appeared at the same
    // time as an error above the fold. tui-test's getByText requires
    // a global regex, so we rebuild each pattern with the `g` flag.
    for (const pattern of ERROR_PATTERNS) {
      const globalRegex = new RegExp(
        pattern.regex.source,
        pattern.regex.flags.includes('g')
          ? pattern.regex.flags
          : pattern.regex.flags + 'g'
      );
      await expect(
        terminal.getByText(globalRegex, { strict: false })
      ).not.toBeVisible();
    }
  }
);
