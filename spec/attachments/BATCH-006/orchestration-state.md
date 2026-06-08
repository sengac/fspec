# Batch 6 Orchestration State

**Supervisor session_id:** `4942d240-b2a2-4d6c-963b-1d9cb688f5b7`
**Cargo Serial Worker session_id:** `dd2e2afa-f485-4e8e-ae5a-af636227d983`
**Started:** 2026-06-07
**Goal:** Port next 10 commands per command-port.md §13 (two batches of 5).

## Batch 6A (slots 1–5) — IN FLIGHT

| Slot | RPC ID  | Command             | Worker session_id                          | Phase | Notes |
|------|---------|---------------------|--------------------------------------------|-------|-------|
| 1    | RPC-242 | list-checkpoints    | `9213eaf9-8009-46b3-aa43-f23583d40174`     | spawned | 89 LOC TS, git-stash list |
| 2    | RPC-301 | show-deleted        | `a4fe522b-9195-425a-98ef-a2be83b34cd6`     | spawned | 109 LOC TS, work-unit soft-deletes |
| 3    | RPC-302 | show-epic           | `b9a6b6af-5e41-4867-98d4-479f30b1a557`     | spawned | 143 LOC TS, reads epics.json |
| 4    | RPC-304 | show-feature        | `776e9b67-4ebc-438f-aa9d-e7eece9d43dc`     | spawned | 208 LOC TS, gherkin re-parse |
| 5    | RPC-310 | tag-stats           | `4dd1ff7a-993a-4e91-930c-ad5bd14b3221`     | spawned | 263 LOC TS, tags.json aggregation |

## Batch 6B (next round) — pending

| Slot | RPC ID  | Command                  | Worker session_id | Phase | Notes |
|------|---------|--------------------------|-------------------|-------|-------|
| 1    | RPC-308 | show-work-unit           |                   | -     | 475 LOC TS, biggest in batch |
| 2    | RPC-257 | query-dependency-stats   |                   | -     | 158 LOC TS |
| 3    | RPC-258 | query-estimate-accuracy  |                   | -     | 218 LOC TS |
| 4    | RPC-263 | query-work-units         |                   | -     | 278 LOC TS, filter chain |
| 5    | RPC-261 | query-metrics            |                   | -     | 255 LOC TS |

## Shared-file change requests (pending supervisor action)

| Requested by | File | Change | Status |
|--------------|------|--------|--------|
|              |      |        |        |
