# BUG-251: Faithful multimodal input archival and restoration

## Priority and evidence

**P2 — confirmed writer/converter contract mismatch.**
`rust/agent-loop/src/agent_loop.rs:392` persists text-only user input, while images separately
reach inference at line 1017. `rust/agent-loop/src/persist.rs:43` constructs only a text block.
Even if a writer saves image/document blocks, `rust/sessions/src/replay_content.rs:163`
rejects unsupported user block kinds. Provider-native snapshots are not faithful original user envelopes.

## Code and existing infrastructure

- User input ingestion and attachment routing in `rust/agent-loop/src/agent_loop.rs`.
- User-envelope creation in `rust/agent-loop/src/persist.rs`.
- Shared `UserContent`, image/document source types in `rust/core/src/persistence/message_envelope.rs`.
- Existing extraction/rehydration in `rust/core/src/persistence/blob_processing.rs`.
- Originals blob restoration in `rust/core/src/persistence/original_blobs.rs`.
- Model/display restoration in `rust/sessions/src/replay_content.rs`.

## Required repair

1. Identify supported user image/document input forms across TUI, embedded/WebSocket transport and
   provider adapters. Do not infer document-send support merely because a schema variant exists.
2. Persist the actual accepted structured user input, preserving block ordering, media type,
   source identity and original bytes where the input contract supplies bytes.
3. Use the existing content-addressed blob store for large payloads; retain explicit blob references
   and proper reachability/cleanup. Do not duplicate base64 into every checkpoint or diagnostic log.
4. Implement both archival and model replay for supported media. Transcript hydration must identify
   attachments faithfully; provider reconstruction must use the authentic supported representation.
5. Handle missing blobs, invalid media metadata, unsupported restored formats, and changed external
   URLs with typed, visible behavior. Do not silently replace an image with text or imply external URL
   content is immutable unless it was captured.
6. Maintain legacy text-only compatibility, provider image-count limits, and the distinction between
   archival retention and any configured limit on images sent in future model context.
7. Coordinate independent projections and paged delivery with BUG-243/244/245 and journal identities
   with BUG-249. No new parallel attachment store or business logic duplicated per transport.

## Regression scenarios

- Send text plus multiple images through the real session input path; verify stored original block
  ordering, media types and blob bytes rather than just a provider checkpoint's incidental image copy.
- Cold resume and capture the next physical provider request; supported image identities/content survive.
- Retrieve bounded originals with blob rehydration and verify exact original bytes/metadata.
- Exercise supported document formats through writer and reader together; unsupported formats return
  explicit compatibility errors rather than silently falling back.
- Remove a referenced blob in an isolated fixture: replay/originals surface a typed missing-source failure.
- Cleanup retains referenced media; unrelated garbage can be removed safely; repeat migration/reopen.
- Legacy text sessions, Unicode text, remote URL sources, oversized attachments and image-limit policy
  retain their agreed semantics through both transports.

## Prohibited shortcuts

- Do not fix only the writer or only the replay converter.
- Do not use an opaque provider checkpoint as the sole attachment archive.
- Do not loosen image limits, force media into every future request, or log binary/base64 payloads.

## Completion evidence

- [ ] Accepted multimodal input has one end-to-end archive/originals/replay contract.
- [ ] Real attachment-send and cold-request tests cover media fidelity, limits and unavailable blobs.
- [ ] Legacy compatibility, cleanup reachability, privacy and bounded storage are verified.
- [ ] Feature-first red/green tests and scoped quality/coverage checks pass.

Related: BUG-243, BUG-244, BUG-245, BUG-249. Investigation:
`docs/investigations/2026-10-23-compaction-and-resume.md`.
