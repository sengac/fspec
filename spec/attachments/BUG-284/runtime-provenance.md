# Running Fspec build provenance, 2026-10-28

## Findings

The current daemon is not the earlier explicitly unpatched PID 366915. Read-only
verification of PID 922084 resolves its executable to
`/home/rquast/.local/bin/fspec`. Its bytes exactly match both the installed path
and the retained `rust/target/release-slim/fspec` build artifact:

`d2c910442adbe130e69e921b151c4613b6fb445f87e05ecd3857f83c7064eb4a`

The retained build's dependency fingerprints connect that binary to compiled
agent-loop and provider artifacts containing the BUG-282 and BUG-278 changes.
This is substantially stronger than inferring patch presence from the version or
current source. It supports treating a missing BUG-282 implementation in this
retained build as an unlikely explanation for the later failures.

Qualification: the final executable is stripped and there is no source-revision
attestation. The evidence below establishes binary identity, matching Cargo build
records, and actual compiled dependency code. It does not independently identify
every instruction in the stripped executable or prove its complete source tree
was exactly a particular commit. Do not turn this into a live-fix claim.

## Runtime and binary identity

- Host: `gx10-d917`.
- Daemon registration: `/run/user/1000/fspec/daemon.json`.
- PID: `922084`; port: `36373`; reported version: `0.10.9`.
- Registered start: `2026-10-28T12:07:57Z`.
- Workspace: `/home/rquast/projects/fspec`.
- ELF architecture: AArch64; PIE; stripped.
- ELF build ID: `7c55e2fe77fcff2023c744d8556699919769c6c4`.
- ELF compiler comment: `rustc version 1.95.0 (59807616e 2026-04-14)`.
- Retained executable modification time: `2026-10-28T11:25:58.124318Z`.
- Installed executable modification time: `2026-10-28T11:25:58.998315Z`.

The process executable hash was read directly through `/proc/922084/exe`, not
merely inferred from the installed filename. No process memory, environment,
credentials, or sensitive session contents were read for this sub-investigation.

## Cargo artifact lineage

The following values are the little-endian numerical forms of Cargo's retained
fingerprint bytes. The parent artifact's JSON dependency record matches each
child fingerprint exactly:

| Artifact | Fingerprint | Relevant dependencies |
| --- | --- | --- |
| `codelet-fspec-c26558c99d34eb71` | `652665305998001474` | agent-loop `3146038877406298734` |
| `codelet-agent-loop-e2e18c20ab833020` | `3146038877406298734` | CLI `3339670947066748426`, providers `6900445911529857783` |
| `codelet-cli-4e40e334ae271608` | `3339670947066748426` | providers `6900445911529857783` |
| `codelet-providers-e631600c23a2c6b1` | `6900445911529857783` | — |

These records are under `rust/target/release-slim/.fingerprint/`.
The final `deps/fspec-c26558c99d34eb71` and `release-slim/fspec` have the same
reported size, timestamp, and build artifact identity. Cargo records are local
build evidence, not signed attestations; their source dep-info uses
`checksum: false` and cannot independently certify all original source bytes.

Compiled archive hashes:

| Archive under `rust/target/release-slim/deps/` | SHA256 |
| --- | --- |
| `libcodelet_agent_loop-e2e18c20ab833020.rlib` | `6c4f096cf09755c532bedc196e74900413fb7027235cfd8f915e989eb98be94e` |
| `libcodelet_cli-4e40e334ae271608.rlib` | `e36117ba4acc1c505e4e61fb41fd2c97439d82370ef75f213a1cd972fb47f144` |
| `libcodelet_providers-e631600c23a2c6b1.rlib` | `8d186a6ce2346679688725655f310a608bf9e03dca50cbf8efe0f00d168eab81` |

## BUG-282: actual compiled tool-result handler

The release profile uses ThinLTO, so its `.rlib` archive members contain LLVM
bitcode. GNU `nm` cannot read these members, and the installed LLVM 21 utility
rejects LLVM 22's attribute 105. No component was installed or rebuilt to work
around that format difference. The already installed Rust 1.95 LLVM 22 shared
library was used read-only via its C API to parse the archive member in memory.

The agent-loop archive's member
`codelet_agent_loop-e2e18c20ab833020.codelet_agent_loop.8e219d1296143338-cgu.10.rcgu.o`
contains the instantiated handler:

`_ZN11codelet_cli11interactive15stream_handlers18handle_tool_result17hf2f55c9e2e16c06cE`

Its printed LLVM IR has 1,234 lines. It contains two `join_generic_copy` calls
(the emitted text and separate error-detection input) and no `replace` reference.
More decisively, the emitted content has this direct value flow:

1. The first `join_generic_copy` returns a 24-byte String into `%25`.
2. IR line index 439 copies those 24 bytes into `%55`.
3. The only non-cleanup uses of `%55` are read-only debug-preview reads and the
   copy at line index 1172 into `%524`, the event's content field.
4. That event is copied into `%29`, then passed to
   `ProgressiveOutput<Output>::StreamOutput::emit` at line index 1186.

There is no intermediate quote stripping or backslash-unescaping rewrite of the
emitted String. This is the post-BUG-282 behavior, inspected in compiled code,
not just the current Rust source.

- Archive member SHA256:
  `89647f333e579b068db3e6ed7dc9a014a87288005ecb79dd0c81e498b7868123`.
- Function printed-IR SHA256:
  `5f99778621074704db222f62736703c616766a13efdd87442a8433577e537d0d`.
- Archive modification time: `2026-10-28T11:19:18.412606Z`.

The current source, `rust/cli/src/interactive/stream_handlers.rs:201`, has the
same canonical join behavior. Its SHA256 is
`2bacd680ef7d83b6a6f934c47956833ffb3c3effe52a55550f1023e4d46b9060`;
its modification time is `2026-10-28T09:15:10.151233Z`.

## BUG-278: compiled generation binding

The provider archive contains actual definitions for:

- `codelet_providers::codex::session::compaction::prepare`;
- `codelet_providers::codex::session::compaction::committed_checkpoint`;
- the serialized `compaction::Binding` type;
- `CodexSession::prepare_compaction_generation`.

The `prepare` function calls `CodexHistory::prepare_generation` with its computed
provider generation. The module and binding were introduced in commit
`042d3974` for BUG-278. The retained provider member
`codelet_providers-e631600c23a2c6b1.codelet_providers.fea258119d903a58-cgu.11.rcgu.o`
has SHA256 `8c79abf23d43fb149078f61037386665a869dc1e81a89ced36ec79454600a0c7`.
Its `prepare` function's printed IR has SHA256
`eb0706fabaec284981f5a2498192094ecae0e3b29222140a8d86e92cff9bf23b`.

The final executable itself contains these distinctive added strings at the
listed file offsets (not process addresses):

| String | Offset |
| --- | --- |
| `_codexCompaction` | `0x9d32fa0` |
| `durable generation exceeds provider lineage` | `0x9f118d9` |
| `runtime checkpoint differs from committed replay` | `0x9f1191d` |
| `missing committed generation binding` | `0x9f11868` |
| `missing selected history validator` | `0x9dfa8a7` |

Strings alone would be insufficient proof of executed behavior; here they
corroborate the matching compiled provider dependency and Cargo lineage.

## Source revision limits

Current HEAD is `626eee1ec96a8e3ec616b5e6411a5c1596815709`, committed at
`2026-10-28T11:51:15Z`, with subject “fix: isolate session creation intent and
preserve canonical tool outcomes”. BUG-278 is in its ancestor `042d3974`.

The three inspected current source files (`stream_handlers.rs`,
`codex/compaction.rs`, `session/selected_compaction.rs`) match HEAD. Their file
modification times precede the retained build. However, the binary was built
before the HEAD commit was created. It can contain then-uncommitted changes; it
must not be labelled a clean build of HEAD solely because those times align.

Remaining provenance gap: a build-time source manifest, commit-plus-dirty-diff
receipt, symbol map, or independent reproducible-build comparison is needed to
certify the entire exact source revision. No build or test was run in this
sub-investigation. No production repair or live compaction success is claimed.

## Reproduction of these read-only checks

Use `readlink /proc/922084/exe`, `sha256sum` on that path and the two retained
executable paths, `readelf -n` and `readelf -p .comment`. Decode the retained
Cargo fingerprint files as little-endian bytes and compare their values with
the parent JSON `deps` entries. Use `ar p` to read the named archive members;
parse their bytes with the installed LLVM 22 `LLVMParseBitcode2`, enumerate
functions and print the two named definitions with `LLVMPrintValueToString`.
All LLVM parsing performed here was in memory; no source, daemon or session
state was changed.

## Follow-up: persistence precedes cancelled-prompt admission

The current `rust/agent-loop/src/agent_loop/prompt.rs` matches HEAD and has
SHA256 `5b3e7546a128e6e684191eff384e8795065699be69f68dd6506b8cd1e6d0b07e`.
Its last commit is `042d3974`. Source lines 126–132 persist the pending input via
an awaited `spawn_blocking` operation; lines 149–154 then call
`finish_cancelled_prompt` and return false if the input was cancelled. This is
the ordering under investigation, not a newly proposed fix.

The retained release agent-loop archive confirms that ordering in compiled IR.
Member `...-cgu.00.rcgu.o` contains the async `admit_prompt` closure ending in
symbol hash `hf1d64665e386c13cE` (4,688 printed-IR lines):

| Operation | Printed-IR line index | Basic block |
| --- | --- | --- |
| Invoke `spawn_blocking` (`hb4045cdc099b54faE`) | 3749 | `1783` |
| Poll its `JoinHandle` | 3814 | `1814` |
| Invoke `finish_cancelled_prompt` | 4095 | `1914` |

This is confirmed control-flow order, not merely textual instruction order.
The successful path is:
`1783 → 1794 → 1814 → 1818 → 1822 → 1825 → 1879 → 1882 → 1914`.
Block `1818` tests whether the join is still pending; blocks `1825` and `1879`
test the joined result and its inner persistence result. Only after completion
and success does this path reach the cancelled-prompt check. Resumption enters
the poll block rather than repeating the spawn.

The same retained archive includes a specialized blocking-task poll function
(`h626ddb5c35b5c755E`, member `cgu.03`) that invokes `persist_user_input`
(`ha2646350297e6e74E`), consistent with the source closure at lines 129–130.
The source helper persists real user content through `persist_user_content`;
it does not itself apply the later cancelled-prompt admission check.

Member `...-cgu.07.rcgu.o` contains the actual `finish_cancelled_prompt` definition
(`h373af1afea8fa743E`, 58 printed-IR lines). It returns false only when neither
`cancelled_at_submission` nor `compaction_cancelled(epoch)` applies. On
cancellation it emits the interrupted result, sets Idle, emits Done, and returns
true. In `admit_prompt`, block `2057` branches on that result: true exits
admission; false proceeds to block `2058`, setting Running. Thus the compiled
path can successfully persist the input and subsequently refuse inference for
that cancelled input.

Verification hashes:

| Item | SHA256 |
| --- | --- |
| Agent-loop `cgu.00` archive member | `f47a5922de660971ee54cbc0595e01461ee149bb521b9459c3537c5fd26fa641` |
| `admit_prompt` printed IR | `7232ccaa3fb8aaca8038e6eadaa4932996a2bfc820273d3f445ddcdf3e53eca0` |
| Agent-loop `cgu.07` archive member | `e402632a5b460eb5fa4212b12ee814d6ad734b7e701df408edfb2390458dc085` |
| `finish_cancelled_prompt` printed IR | `2f9c632ab4dd8d394f683e772a7f4eede3d39af84336a87dfe43756879fef142` |

The previously documented exact executable hash and Cargo dependency chain
associate these compiled objects with the retained runtime build. The stripped
binary/source-attestation qualification still applies. This static evidence
confirms the relevant compiled ordering exists; it does not alone prove which
inputs traversed it in a particular live session. That requires the separate
failed-session receipts and real-evidence replay owned by the investigation.
No sessions were mutated and no code was compiled or executed as a diagnostic
test in this follow-up.
