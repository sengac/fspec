# JSONL framing and recovery policy

## Design

BUG-248 uses the existing stable message sidecar lock for all inspection, repair,
index publication and appends. No new locking hierarchy or schema is introduced.
A record is a complete deserializable StoredMessage on a physical JSONL line,
with a matching content hash. UUID-looking text is never sufficient for indexing.
Legacy records with an empty hash remain readable without hash verification.
Empty physical lines may be ignored, but malformed delimited lines are corruption.

A complete final JSON value without a newline is retained and delimited before
another append. This includes valid Unicode content. An incomplete UTF-8 sequence
is handled as bytes, not sliced as a Rust string or treated as an I/O panic.

An invalid unterminated final physical line is eligible for recovery only after
a strict prefix scan and examination of committed session manifests. Every
committed message reference must resolve in the valid prefix; malformed or
unreadable committed manifests and an absent sessions directory prevent
destructive recovery. An existing empty sessions directory is explicit evidence
that no committed session currently references the tail. A recognizable
referenced UUID in the torn tail is also grounds for refusal, including duplicate
metadata versions. Reading manifest evidence uses atomic committed files without
taking session leases or calling session-store/global APIs, avoiding lock inversion.

Tail identity observation parses the top-level JSON map rather than matching a
literal byte spelling. JSON whitespace and escaped key/value strings therefore
cannot hide a referenced metadata replacement. It stops at the first root `id`;
incomplete or malformed later fields do not erase the observed identity. This
observation can only refuse truncation, never validate a record or publish an
index entry. Strict complete-record parsing remains the authority for indexing.

Invalid unterminated tail bytes, including referenced corruption that refuses
truncation, are copied to a deterministic same-directory quarantine file.
The quarantine file and directory are synchronized before invalidating the old
derived index and truncating JSONL to the validated boundary. The repaired JSONL
and directory are synchronized before any append. An interrupted recovery can
reuse only a quarantine file with exactly matching evidence. Recovery never
tries to extract records from inside arbitrary malformed JSON strings.

The derived-index version is advanced so indexes previously built using only
UUID extraction cannot authorize incomplete records. File generation checks and
index invalidation prevent using offsets from a different JSONL generation.
Existing String-returning MessageStore APIs preserve compatibility while the
framing implementation classifies failures with a typed internal error.

## Evidence and limitations

Dedicated real-filesystem regressions cover partial JSON tails, complete
unterminated JSON, referenced UUID/UTF-8 corruption, interior malformed lines,
concurrent recovery/appends, interrupted quarantine boundaries and repeated full
index rebuilds. A property test enumerates every byte cut of generated serialized
Unicode records. Temporary directories come from codelet-test-helpers. No live
store is mutated, and no ordinary reopen is presented as proof of power-loss
survival. The BUG-247 durability contract specifies the synchronization limits.

Recovery preserves evidence but does not provide automatic repair of committed
corruption: operators must retain the original JSONL, quarantine files, manifests,
and any backups for an explicit forensic restore. It is safer to refuse an append
than to silently discard a potentially referenced original.
