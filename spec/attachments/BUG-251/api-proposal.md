# BUG-251 media persistence integration proposal

Discovery confirms `agent_loop.rs` persists only text before separately forwarding
`BridgeImageData {data, media_type}` to inference. The accepted input contract is
text followed by ordered base64 images. There is no document-send UI contract.
Stored envelope schemas do accept base64 and URL images/documents.

Proposed bounded changes:

- New `agent-loop/user_persistence.rs` owns `persist_user_input(&Uuid, &str,
  &[BridgeImageData])` and structured `persist_user_content` construction.
- One import/call replacement in `agent_loop.rs`; BUG-246 retains admission,
  error handling and retry ownership. BUG-251 does not edit `persist.rs` or
  `background_output.rs`.
- BUG-246 should expose its shared recoverable envelope submission function
  to the new helper, accepting an immutable `MessageEnvelope` and display text.
  Stable UUID retries, ordering and bounded admission must remain shared.
- New `sessions/replay_media.rs` converts stored image/PDF base64/URL sources
  into rig content with explicit metadata/unsupported-source errors. It does
  not invent document UI input or flatten media into text.
- Existing blob extraction/rehydration and bounded originals remain the only
  media storage. Provider-native snapshots are not archival evidence.
- Replay stays extensible for BUG-250's independently coordinated event decoder.

Tests will exercise accepted real input, bounded originals, cleanup reachability,
fresh-process restore and the physical next request. Legacy Unicode, remote URL
identity, unsupported formats, missing blobs and image-context limits are explicit
cases. Remote URLs are references, not immutable captured content.

Coordination messages to parent and BUG-246 were rejected as busy, not queued;
this file is the durable early proposal. Production changes await executable RED
and the exact shared retry API. Available disk measured 23 GiB at start.

## Resolved integration contract

BUG-246's `coordination-contract.md` proposes
`persist::persist_user_content(&Uuid, Vec<core::UserContent>) -> Result<(), String>`.
BUG-251 accepts that API: the new input helper will only build ordered blocks and
delegate. BUG-246 must preserve existing `process_envelope_for_blob_storage` and
`_blobRefs` metadata on its durable envelope commit path; originals and cleanup
already consume this format. The recovery owner must retain authentic content
even if blob extraction itself fails. No alternate append or media spool is added.

Actual `maxImages` limits apply to the Read tool's result, not historical/accepted
input images. Tests exercise that existing budget separately; replay must not
invent input clipping. Anthropic supports stored base64 PDF, but its current rig
adapter does not support URL documents. The physical URL/PDF regression therefore
uses the OpenAI Responses adapter which actually supports those sources; no rig
provider adapter expansion is proposed.
