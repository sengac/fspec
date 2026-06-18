# AST / Behaviour Research — `reverse` (RPC-294)

TS source of truth:
- `src/commands/reverse.ts` (687 LOC)
- `src/commands/reverse-help.ts` (help config)
- `src/types/reverse-session.ts` (session + result interfaces)
- `src/utils/reverse-session.ts` (session persistence helpers)
- `src/utils/project-root-detection.ts` (`findProjectRoot`)

## 1. Command surface (Commander.js registration, reverse.ts:669-687)

```
fspec reverse [options]
  --strategy <A|B|C|D>   choose strategy
  --continue             advance to next step
  --status               show session status
  --reset                delete session, start fresh
  --complete             finalize + delete session
  --dry-run              preview analysis, no session write
```
No positional arguments. All flags are mutually exclusive in practice; the
command checks them in a fixed priority order (see §3). `--strategy=D` also
honours an `implementationContext` option that is NOT exposed via Commander
(`reverse.ts:118-121`) — it is only reachable through the structured/dispatcher
path. **Flag to supervisor:** the CLI surface cannot pass `implementationContext`;
the help doc / Commander has no such option, so the standalone binary cannot
trigger Strategy-D persona guidance. Dispatcher path can pass it via JSON.

## 2. Session persistence (utils/reverse-session.ts)

- Session file path: `join(os.tmpdir(), 'fspec-reverse-<hash>.json')`
  where `<hash> = sha256(projectRoot).hexdigest()[:12]` and `projectRoot`
  comes from `findProjectRoot(cwd)` (boundary-marker walk: `.git`,
  `package.json`, `.gitignore`, `Cargo.toml`, `pyproject.toml`; max depth 10).
- `sessionExists(cwd)` → `fs.access` truthiness.
- `loadSession(cwd)` → parse JSON or `null` on any error (bare catch).
- `saveSession(cwd, session)` → `JSON.stringify(session, null, 2)` (2-space).
- `deleteSession(cwd)` → `fs.unlink`, swallow ENOENT.
- `createSession(phase, gaps, strategy?, strategyName?)` →
  `{ phase, gaps, strategy, strategyName, timestamp: new Date().toISOString() }`.
- `setStrategy(session, strategy, strategyName, totalSteps)` →
  `{ ...session, phase:'executing', strategy, strategyName, currentStep:1,
     totalSteps, timestamp }`.
- `incrementStep(session)` → `currentStep = (currentStep ?? 1) + 1`, new timestamp.
- `validateCompletion(session)` → `currentStep && totalSteps && currentStep >= totalSteps`.

**Async/IO note:** all of this is plain blocking `fs` (read/write/unlink/access)
+ sha256 + ISO timestamp. NO child process, NO network. Effectively synchronous —
fits `poll_sync_future`. Rust will use `std::fs` + `sha2` crate + `chrono` (already
a fspec-core dep) for the ISO-8601 timestamp.

## 3. Control flow priority (reverse.ts:29-253) — FIRST MATCH WINS

1. `--reset`  → deleteSession; return `{ message: 'Session reset' }`.
2. `--status` → loadSession; if none → `{ message: 'No active reverse session' }`.
   Else return phase/strategy/strategyName/gapsDetected(progress)/gapList
   (gapList = files mapped to `{ file, completed: idx < currentStep-1 }`).
3. `--complete` → loadSession; none → `{ message:'No active reverse session to
   complete', exitCode:1 }`. Else `validateCompletion`: false →
   `{ message:'Cannot complete: not all steps are finished', exitCode:1 }`.
   true → deleteSession; return success + systemReminder
   ("Session completed successfully.\nAll gaps filled.") + validationComplete:true,
   gapsFilled:true, message '✓ Reverse ACDD session complete'.
4. `--continue` → loadSession; none → `{ message:'No active reverse session',
   exitCode:1 }`. Else incrementStep + saveSession; compute isFinalStep
   (currentStep === totalSteps) + nextFile (gaps.files[currentStep-1]); return
   systemReminder(Step N of M / Process file: <nextFile> / next-cmd hint where
   final step says run --complete else run --continue) + guidance line.
5. `--strategy` →
   - If strategy==='D' AND implementationContext present → `handleStrategyD`
     (persona-driven; works WITHOUT a session). (dispatcher-only — see §1.)
   - Else loadSession; none → `{ message:'No active reverse session', exitCode:1 }`.
     totalSteps = gaps.files.length; setStrategy + saveSession; firstFile =
     gaps.files[0]; return systemReminder(Step 1 of N / Strategy: X (name) /
     run --continue) + guidance("Read test file: <firstFile>. Then create
     feature file. Then run fspec link-coverage with --skip-validation.").
6. If `sessionExists(cwd)` (no flag matched, session already present) →
   load; corrupt → `{ message:'Session file corrupted', exitCode:1 }`. Else
   `{ existingSessionDetected:true, exitCode:1,
     message:'Existing reverse session detected', currentPhase, currentStrategy
     ("X (name)"), currentProgress ("Step N of M"), suggestions[4],
     systemReminder('Existing session detected. DO NOT start new session...') }`.
7. Otherwise — INITIAL ANALYSIS (no flag, no session):
   - `analyzeProject(cwd)` → testFiles / featureFiles / implementationFiles /
     coverageAnalysis + summary string.
   - `detectGaps(analysis)` → GapAnalysis (counts + files[]).
   - `suggestStrategy(gaps)` → A|B|C|D (priority A→B→C→D, default A).
   - `getStrategyName`, `formatGaps`.
   - If `--dry-run` → return analysis/gaps/suggestedStrategy/strategyName +
     message 'Dry-run mode - no session created' + systemReminder(DRY-RUN ...)
     + guidance. NO session write.
   - Else createSession('gap-detection', gaps, suggested, name) + saveSession;
     return analysis/gaps/suggested/name + systemReminder(Gap analysis
     complete...) + guidance + effortEstimate. If totalGaps>=100 → add
     pagination {total, perPage:50, page:1} + summary + append narrow-scope hint.

## 4. analyzeProject helpers (reverse.ts:256-340)

- `findTestFiles(cwd)`: scan dirs `['src/__tests__','test','tests','__tests__']`
  (non-recursive `readdir`); keep files matching `/\.test\.(ts|js|tsx|jsx)$/`;
  push `join(dir, name)`.
- `findFeatureFiles(cwd)`: readdir `spec/features` (non-recursive); keep
  `*.feature`; push `join('spec','features',name)`. ENOENT → `[]`.
- `findImplementationFiles(cwd)`: recursive walk of `src/` via `scanDirectory`,
  skipping dir names `__tests__|tests|test`; keep `.ts|.js|.tsx|.jsx` files
  that do NOT end `.test.ts`; push path relative to cwd (`fullPath.replace(cwd+'/','')`).
- `analyzeCoverage(cwd, featureFiles)`: for each feature file, read
  `<feature>.coverage` JSON; count scenarios where `testMappings` empty/absent;
  collect `"<feature>:<scenario.name>"`. ENOENT/parse error → skip. Returns
  undefined if no feature files.

## 5. detectGaps / suggestStrategy / getStrategyName / formatGaps / effort

- `detectGaps`: counts
  - testsWithoutFeatures = (tests>0 && features==0) ? tests.length : 0
  - featuresWithoutTests = (features>0 && tests==0) ? features.length : 0
  - unmappedScenarios = coverage.unmappedCount || 0
  - unmappedImplementation = unmappedImplFiles.length (impl files w/o feature,
    excluding "pure utilities": utils/format, utils/parse, utils/validate,
    helpers/, constants/)
  - files[] selection (first matching): A→testFiles; B→featureFiles;
    C→coverage.scenarios; D→unmappedImplFiles.
- `deriveFeatureName(implPath)`: filename, strip ext, camelCase→kebab,
  PascalCase→kebab, lowercase. (exported, used by hasFeatureFile)
- `hasFeatureFile`: featureFiles include `spec/features/<derived>.feature`
  OR any path containing `/<derived>.feature`.
- `suggestStrategy`: A if testsWithoutFeatures>0 else B if featuresWithoutTests>0
  else C if unmappedScenarios>0 else D if unmappedImplementation>0 else 'A'.
- `getStrategyName`: A 'Spec Gap Filling', B 'Test Gap Filling',
  C 'Coverage Mapping', D 'Full Reverse ACDD', else 'Unknown Strategy'.
- `formatGaps`: first non-zero → "<n> test files without features" /
  "<n> feature files without tests" / "<n> scenarios without coverage mappings"
  / "<n> implementation files without features" else 'No gaps detected'.
- `generateStrategyGuidance`: per-strategy static string (A/B/C/D), else ''.
- `getEffortEstimate`: totalGaps = sum of 4 counts. A → "<2t>-<3t> points";
  B → "<t>-<2t> points"; C → "1 point total"; D → "<3t>-<5t> points"; else 'Unknown'.
- `wrapSystemReminder(content)` → `<system-reminder>\n<content>\n</system-reminder>`.

## 6. CLI wrapper output (reverseCommand, reverse.ts:629-664)

Order of `output.log` emissions:
1. systemReminder (if present)
2. message (if present)
3. guidance (if present)
4. suggestions (if present): "\nNext steps:" then "  - <s>" per suggestion.
Then `process.exit(result.exitCode || 0)`. Catch → `output.error('Error:', msg)`
+ exit 1.

NOTE: the CLI wrapper does NOT print analysis / gaps / suggestedStrategy /
effortEstimate / pagination / summary as standalone lines — those live only in
the structured result object. The help-doc EXAMPLES show richer text
("Found 3 test files...", "Suggested Strategy: A...", "Estimated Effort:...")
that the real CLI wrapper does NOT emit (Framing A: help doc diverges from
broken CLI). **Question to resolve in scenarios:** does the dispatcher emit
JSON (structured result) and the CLI emit the systemReminder+message+guidance
text path? The two-front-doors contract: core `run` returns a String. We will
return the **CLI-wrapper-equivalent rendered text** as the default, matching
what `reverseCommand` would print. There is no `--format json` flag in TS for
reverse, so default text render is the parity target.

## 7. Rust port plan / dependencies

- Core impl: `codelet/fspec-core/src/commands/reverse.rs` — rewrite stub.
  Signature MUST change to `run(args_json, project_root)` (currently
  `run(args_json)`). **SHARED-FILE CHANGE NEEDED (supervisor):**
  `dispatch.rs:741` arm `commands::reverse::run(args_json).await` →
  `commands::reverse::run(args_json, project_root).await`.
- New type module: `codelet/fspec-core/src/types/reverse_session.rs`
  (ReverseSession, GapAnalysis, AnalysisResult). Register in types/mod.rs —
  **SHARED (supervisor)**.
- Dependency: `sha2` for the session-file hash. It exists at
  `codelet/Cargo.toml:181` (workspace) but fspec-core's `[dependencies]` does
  NOT list it. **SHARED-FILE CHANGE NEEDED (supervisor):** add
  `sha2 = { workspace = true }` (or `sha2 = "0.10"`) + `hex` (or use
  `format!("{:x}")` on bytes) to `codelet/fspec-core/Cargo.toml`. No `hex`
  crate currently; will hex-encode manually to avoid a 2nd dep.
- Session path uses `std::env::temp_dir()` (= os.tmpdir()).
- Project-root walk: reuse `crate::io::project_root::find_project_root` (exact
  parity with findProjectRoot — same markers, same depth 10). ✓ already exists.
- Timestamps: `chrono::Utc::now().to_rfc3339()` — but TS uses
  `Date.toISOString()` (millisecond precision, trailing Z). Use
  `chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ")` for parity (timestamp
  is not asserted in tests; cosmetic).
- Help config: `codelet/fspec-core/src/help/configs/reverse.rs` + register
  (mod.rs is SHARED → supervisor adds `pub mod reverse;`).
- CLI bridge: `codelet/fspec/src/<>` — marshal 6 bool/opt flags → JSON.
- clap Mode variant + intercept arm in main.rs (SHARED → supervisor).

## 8. Flags raised to supervisor (summary)
1. dispatch.rs:741 signature change (add project_root).
2. fspec-core/Cargo.toml: add `sha2` dependency.
3. types/mod.rs: register `pub mod reverse_session;`.
4. help/configs/mod.rs: register `pub mod reverse;`.
5. main.rs: Mode::Reverse variant + forward! arm + intercept arm + `mod reverse;`.
6. `implementationContext` (Strategy D) is dispatcher-only (no clap flag) — confirm acceptable.
