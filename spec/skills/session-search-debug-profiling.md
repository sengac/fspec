# Profile and Debug Session Search

Use this skill to investigate slow session searches, delayed cancellation, or
inconsistent tool-history replay. It applies regardless of the model provider or
agent client in use. Adapt commands to the installed version and operating system.

## Safety and privacy

- Profile a separate debug process against an isolated data snapshot. Do not
  attach to a live user process or replace an installed binary.
- Prefer synthetic session history. If real history is necessary, obtain
  permission, restrict access to the snapshot, and keep it out of version control.
- Never print credentials or include them in commands, logs, or reports. Prefer
  dedicated test credentials; copy existing credentials only with permission.
- Do not hard-link or symlink writable live data into the test environment.
- Treat transcripts, stack samples, and diagnostic logs as potentially sensitive.
  Before sharing, remove personal names, account identifiers, absolute home paths,
  session and tool-call identifiers, private project details, and message content.
- Keep raw evidence in restricted local storage. Use placeholders and relative
  timings in shared reports; never embed real incident history in this skill.

## 1. Define the experiment

Write down the symptom and a falsifiable question before collecting samples.
Examples:

- Does time pass in search execution, storage recovery, or result persistence?
- Does cancellation stop the search worker or only stop waiting for its result?
- Is a missing tool outcome absent from durable history or only from replay?

Record the build revision, optimization level, operating system, dataset size,
query, filters, and concurrency. Use a non-sensitive query. Keep those inputs
constant when comparing builds, and distinguish cold-start from warm runs.

## 2. Build a symbol-enabled binary

Check available disk space first: compiling and linking may require several GiB.
From `rust/`, a bounded debug build is:

```sh
CARGO_PROFILE_DEV_DEBUG=1 CARGO_INCREMENTAL=0 \
  cargo build --offline -p codelet-fspec --bin fspec -j 2
```

Use offline mode only when dependencies are cached. Locate the output using the
configured Cargo target directory; the default is `rust/target/debug/fspec` from
the repository root.

Debug builds are useful for stack attribution, not release-latency claims. For
timing comparisons, use equivalent optimization and symbol settings on both
builds. Do not rebuild or overwrite the installed executable.

## 3. Prepare an isolated snapshot

1. Create a private temporary directory with owner-only permissions. Use a
   restrictive umask for files created during the investigation.
2. Populate it with synthetic data or a consistent snapshot of the required
   session manifests, message store, indexes, and blobs. Stop the source writer or
   use a storage-supported snapshot procedure; copying changing files separately
   can produce an inconsistent store.
3. Use independent copies or copy-on-write clones where supported. Never share
   writable files with the live store.
4. Copy only the configuration needed for the experiment. Configure a test model
   if inference is necessary, without assuming any particular provider.
5. Redirect `HOME`, runtime directories, and any provider-specific configuration
   or cache directories into the temporary environment. Verify effective paths
   before starting the process; changing `HOME` alone may not isolate every store.
6. Preserve project-path relationships required by the search filters. Use an
   isolated project directory where possible. If a fixture needs path adjustment,
   change only the fixture through supported mechanisms.

A copied store may invalidate identity-bound verification metadata and trigger
recovery on its first write. Do not fabricate verification metadata or modify the
live store to bypass recovery. Measure this cold-copy behavior separately, then
wait for recovery to finish before collecting warm-search measurements.

## 4. Start and verify the test process

From the intended project directory, start the debug binary with an explicit
workspace and a private PID file:

```sh
"$DEBUG_BINARY" --workspace "$TEST_PROJECT" daemon --pidfile "$TEST_PIDFILE"
```

Set these variables to the verified debug executable, test workspace, and private
PID-file path. Bind test services to loopback where supported. Record the test
process PID and endpoint, and confirm that they do not belong to a live instance.

Connect through the supported client or RPC interface. For an MCP stdio client,
initialize the connection, send `notifications/initialized`, and discover tool
schemas before calling tools. Do not assume schemas from a different version.

Create a test session, submit the intended SessionSearch request, and inspect its
buffered output or events. Confirm the actual query, project scope, filters, tool
invocation, and result. Successful input dispatch is not proof of tool completion.
Check the effective project context: the process working directory and workspace
argument may influence different parts of session filtering.

## 5. Capture a bounded profile

Choose a profiler supported by the operating system and attach only to the test
PID. For example, on macOS:

```sh
/usr/bin/sample "$TEST_PID" 10 -file "$PROFILE_OUTPUT"
```

On other systems, use an available sampling profiler or tracing tool with symbols
enabled. Check permissions and profiler overhead before interpreting results.

Capture samples while the target operation is active. Separate these phases:

1. Process startup and store recovery.
2. Prompt admission and input persistence.
3. Model latency and tool dispatch.
4. Search execution, including reads, parsing, filtering, and lock waits.
5. Result persistence and output delivery.
6. Cancellation acknowledgement, durable settlement, and worker termination.

Inspect both hot stacks and blocked threads. Sampling frequency is not an exact
duration or proof of causality, and CPU samples alone may miss I/O waits. Correlate
stacks with lifecycle events and elapsed times. Repeat warm runs and report
variation. A broad all-project search is not equivalent to a project-scoped one.

## 6. Investigate cancellation and replay when relevant

For cancellation, test interruption during prompt admission and during active
search separately. Record when the request is acknowledged, when output stops,
when durable settlement finishes, when the session becomes runnable, and when
the worker actually exits. Do not treat these events as interchangeable.

Dropping an async future does not necessarily stop an already-running blocking
worker. Check for continued reads, CPU use, or lock contention after cancellation.
Ensure later measurements are not competing with abandoned work.

For replay failures, compare the original durable invocation and outcome with the
active in-memory history and the provider's replay representation. Search previews
are not authoritative evidence of missing records. Distinguish a missing durable
outcome from a projection or checkpoint mismatch. Do not rerun a potentially
side-effecting tool merely because its result is absent from replay.

## 7. Report findings and choose the next test

Produce a sanitized report containing:

- The question, reproduction steps, build settings, and dataset characteristics.
- Cold and warm measurements, repetitions, and known contention or profiler costs.
- The observed lifecycle sequence and relevant symbolized stack paths.
- Measured facts, hypotheses, and untested possibilities clearly separated.
- The smallest follow-up experiment or regression scenario that can test the
  suspected cause.

Do not generalize a debug-build timing into production latency or infer live-store
behavior from a cold snapshot. Preserve durability and tool-call/outcome integrity
when proposing improvements. Follow the project's specification-first and
test-first workflow before implementing a fix.

## 8. Clean up safely

Stop only the isolated process and confirm that it and any child workers have
exited before deleting test data. Try graceful shutdown first; if blocked work
prevents it, explicitly terminate only verified test processes.

Remove the temporary environment, including copied credentials and private
configuration. Retain only necessary evidence under restricted access, and share
only sanitized reports. Do not delete live history, change the installed binary,
or clean up processes by a broad name match.
