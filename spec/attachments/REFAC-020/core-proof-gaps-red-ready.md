# REFAC-020 revised core RED readiness after independent test review

This supersedes the counts/pins in `core-red-targets-ready.md`, not its historical
ACDD chronology. Addressed `/tmp/refac020-core-red-test-review.md` using only
focused tests, their acceptance feature and this evidence. Production is still
NOT authorized. This worker ran no Cargo/build/behavioral test or migration.

## Five bounded changes

1. Selected concurrency now observes both children remaining pending while the
   external guard is held, rechecks each immediately before release and checks
   exact original sidecar bytes. Only afterward may one succeed and one reject
   its stale hash. Their feature spellings differ (`thing`/`thing.feature`).
2. Unlink's full expected scenarios array now comes from the original fixture,
   not a previous actual result. It includes an unrelated scenario sentinel
   before the selected named scenario. Both row removal and subsequent selected
   implementation removal compare independent complete arrays, including all
   unknown scenario members and retained duplicate arrays. Fresh hash expectations
   also derive from those independently expected states.
3. Two new 1:1 scenarios/tests reject duplicate raw scenario names: indexed
   link/unlink without arbitrary selection, and selected generation preview/apply
   without merging or partial additions. Both compare original raw bytes/inventory.
   The existing mapped-removal scenario now additionally tests `testMappings:[{}]`,
   retaining the original complete two-row lineage case as its first matrix entry.
4. Legacy link and unlink each run both accepted feature-name spellings against
   the same constant sidecar guard. Generation runs selected short, selected
   suffixed and global discovery paths against that same guard. Latest mapping
   and unknown-member receipts must survive each post-release operation.
5. Successful selected generation compares the entire file/directory/byte tree
   after excluding exactly the target sidecar and its one known sibling lock.
   Arbitrary new files of any suffix now fail. The successful nested-path case
   has the same exact inventory check. Existing unrelated-byte assertions remain.

### Lock-observation limitation remains explicit

The worker's `entered` marker is emitted BEFORE dispatch. It is not an exact
acknowledgement that the implementation reached or attempted a lock call.
The 500ms pending-result/process probe is a practical scheduling observation,
not a deterministic proof of that internal boundary under every scheduler.
Tests additionally require unchanged sidecar bytes while held and full latest
scenario preservation after an intervening mapped-row/unknown-member commit
under the guard. They do not substitute the timed probe for those state checks
or claim repository-wide transaction safety.

## Static validation and current boundary

- Native format, feature validation and tag validation pass for the revised
  acceptance feature. It has 17 scenarios and 17 corresponding business tests
  (7 selected, 7 generation, 3 transaction); static exact step matching has zero
  mismatches. The additional ignored child-worker test remains infrastructure.
- All 20 maintained Rust test sources are below 300 physical lines; maximum133.
- A read-only rustfmt check reports formatting differences, with no parser-error
  diagnostics. Formatting remains pending; this is not compiled or GREEN evidence.
  Log: `/tmp/refac020-core-proof-gap-format-check.txt`. A final one-line matrix
  fixture adjustment after that check retains the original complete two-row case.
- Parent's permitted shared test-helper dev dependency is now observed in
  `rust/fspec-core/Cargo.toml`; this worker did not edit Cargo.
- All four production command hashes below remain unchanged. No help metadata,
  production modules, CLI sources, protected contracts, histories or security
  configuration were edited by this follow-up.
- Participating writers only: link/unlink/global and selected generation.
  Scenario/create/delete commands, report-to-sidecar writes and the old daemon
  remain nonparticipants. Exclusive pinned migration must prohibit their writes.
  No sidecar bootstrap/global-generation exception is exercised by this worker.

## Parent-run exact targets

Working directory `rust/`; parent owns Cargo scheduling, compilation/fixture
repairs, causal RED, production authorization, review and actual migration.

```sh
CARGO_NET_OFFLINE=true CARGO_PROFILE_CI_TEST_DEBUG=0 cargo test --profile ci-test -p codelet-fspec-core --test refac020_selected_coverage -j 2
CARGO_NET_OFFLINE=true CARGO_PROFILE_CI_TEST_DEBUG=0 cargo test --profile ci-test -p codelet-fspec-core --test refac020_scoped_generation -j 2
CARGO_NET_OFFLINE=true CARGO_PROFILE_CI_TEST_DEBUG=0 cargo test --profile ci-test -p codelet-fspec-core --test refac020_coverage_transactions -j 2
```

Compiler/fixture failures are not causal RED. Normal validation and unselected
duplicate-selection cases remain preservation controls, not new-RED claims.
After formatting or fixture changes, refresh pins and record exact sources used
for parent RED. No executed pass/fail count is claimed here.

## Pre-RED production pins

Paths relative to `rust/fspec-core/src/commands/`:

| Source | SHA256 |
| --- | --- |
| link_coverage.rs | f57790771cf73df585ac2be6f41272ee8667dd4ecf3c6e80f6f5e18519272810 |
| unlink_coverage.rs | 461f64e20b95e790b4a35e6cc3374e712bdde4cd814bc143492bd4008bf0eda2 |
| generate_coverage.rs | 9991756bb07e19f6ba1a7dad372d73ddb8c5e276bdf297f84bcee7308ead37ad |
| show_coverage.rs | 147af9d62a6a7397eec1bfcec9a6eba8e14ac62aecd301179fafb8f99420639d |

## Revised spec/test pins

Feature SHA256: `e2a26638091013a6eea00af11f0ef8a1e83d79e8951ef5abc37ddbe7134163b5`.
The 20 Rust sources plus the feature have manifest hash
`69474fba80ad2acfe5f3f793c96fed3fc4dee149d81f9a7dd9eb3aec4af136e6`.
Definition: sorted workspace-relative paths, one standard `sha256sum` record
per file, then SHA256 of those newline-terminated records. Its exact path set
is this feature, the three target-root `.rs` files, and every `.rs` file under
`tests/refac020/`, `tests/refac020_selected_coverage/`,
`tests/refac020_scoped_generation/`, `tests/refac020_coverage_transactions/`.

Selected changed-source pins under `rust/fspec-core/tests/`:

| Source | SHA256 |
| --- | --- |
| refac020_selected_coverage/unlinking.rs | a74c00c7f090ef61fc9f7acd5cce5a63e0d3fe316ec34b4c2c94f210cd7b5681 |
| refac020_selected_coverage/ambiguity.rs | a6847d5f31ff757453596924c8bd96d174b09b4cf76d644648ea9ab94221e214 |
| refac020_scoped_generation/ambiguity.rs | 4cb63dcb2fbb5693445cd548fe01aab44045ad073065ef3e9a04aea9e73d868c |
| refac020_scoped_generation/blocked.rs | e7f392aa3016e38859e5ceff112252fec48d5c268b97564d2ac92c53ef2baf21 |
| refac020_scoped_generation/scope.rs | 20e019bc093f9577846d86cf645418b05f16cab8df764333d2ff136bc948843a |
| refac020_coverage_transactions/selected.rs | 8ec4e1fa6069a6e50b394e1727e6bb969ecd7e47ef3fc8ebaa2ab8f09fbcb7f5 |
| refac020_coverage_transactions/process.rs | 8a2a11c08ad9572545c28538823a1a9472b42bdf8aeafdd1a120c29a2c1496fb |
| refac020_coverage_transactions/legacy.rs | 5291cea1003f86864b623105fd9b21cc4665b666cb45066c79c24da7b7ec223a |
| refac020_coverage_transactions/generation.rs | cdd8080f3e1aad0600362105567e38cce66c41f01a7bc616816fdb556b8af85f |
