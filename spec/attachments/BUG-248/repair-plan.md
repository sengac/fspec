# BUG-248: Recover torn JSONL tails without losing later records

## Priority and evidence

**P1 — source-demonstrated corruption/rebuild sequence, not observed live-store corruption.**
An unterminated partial final record can be treated as indexed coverage through EOF.
A later append is not separated or repaired. Its direct offset can initially work, but a
full rebuild sees a merged malformed physical line and cannot recover the later record normally.

## Code to inspect and isolated reproduction

- `rust/core/src/persistence/messages/index.rs:137`: fast UUID extraction accepts a line without validating complete JSON.
- `rust/core/src/persistence/messages/index.rs:152`: scan records coverage through file length.
- `rust/core/src/persistence/messages/write.rs:74`: appends directly without tail framing validation.
- `rust/core/src/persistence/messages/recovery.rs:14`: full rebuild uses the same scanner.

Using only a shared-helper temporary store: write a valid prefix and a partial final JSON
record with a recognizable UUID but no newline; reopen; append a valid new message; remove
the derived index; reopen/rebuild and compare retrieval to the pre-rebuild result.
Also test a valid final JSON record without newline, which must not be confused with corrupt data.

## Required repair

1. Define strict committed-record framing and the last known valid boundary. Fast indexing may optimize
   validation only if it cannot publish incomplete or malformed records as valid committed content.
2. Under the same writer/file lock, validate/recover the tail before any subsequent append.
   Ensure a valid unterminated record is retained and safely delimited.
3. Distinguish uncommitted torn bytes from corrupted referenced records. Quarantine or repair only
   according to an explicit policy; preserve evidence and fail visibly for committed corruption.
4. Handle truncated JSON, partial UUIDs, multibyte UTF-8 cuts, malformed complete lines and interior
   corruption without arbitrary rescanning that invents record boundaries inside strings.
5. Keep index coverage at validated boundaries, rebuild consistent offsets, and synchronize repairs
   according to BUG-247. Avoid publishing an index for a different file generation.
6. Make recovery idempotent and safe under concurrent readers/writers and interrupted recovery.

## Regression scenarios

- Partial JSON tail followed by a successful append remains recoverable after a full index rebuild.
- Valid JSON without a trailing newline is preserved and delimited before the next append.
- Partial UTF-8 tails and truncated UUIDs produce controlled recovery/typed failure, not scan panic.
- Corruption of a record referenced by a manifest is reported and never silently discarded.
- A malformed interior line does not cause unrelated valid committed records to be misidentified.
- Repeated reopen/recovery and concurrent appends retain exact IDs, ordering and content.
- Property-test truncation at every byte boundary of serialized records, including Unicode content.

## Prohibited shortcuts

- Do not always truncate the last line, accept UUID presence as JSON validity, or concatenate a newline
  onto arbitrary malformed JSON and then report successful repair.
- Do not silently skip missing committed records or rely on an in-memory cache to hide corruption.
- Do not mutate the user's real message store to obtain a red test.

## Completion evidence

- [ ] New isolated filesystem tests fail for the current append/rebuild sequence, then pass.
- [ ] Valid unterminated records, committed corruption, and UTF-8 boundaries are differentiated.
- [ ] Recovery is idempotent, synchronized and safe with the existing shared-store locks.
- [ ] Feature scenarios, property tests, scoped quality checks, and recovery documentation are complete.

Related: BUG-246, BUG-247, BUG-249. Investigation:
`docs/investigations/2026-10-23-compaction-and-resume.md`.
