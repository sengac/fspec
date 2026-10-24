# CMPCT-071 status and ownership handoff

Status: **testing, incomplete**. No commits.

## Verified evidence

- Provider admission: four passing tests in
  `/tmp/cmpct071-serialized-complete.log` using
  `cargo test --profile release-slim -j 2 -p codelet-providers --test cmpct071_serialized_admission`.
- Initial provider red: missing admission module in
  `/tmp/cmpct071-serialized-red.log`, before production implementation.
- Runtime red: one passing fitting control and three failed rejection scenarios
  in `/tmp/cmpct071-cli-red.log` and `/tmp/cmpct071-route-red.log`, using
  `cargo test --profile release-slim -j 2 -p codelet-cli --test cmpct071_whole_request_fit`.
  Overflow and missing-limit rounds incorrectly return success.
- Feature validation, coverage mapping audit and focused rustfmt checks pass.
  Eight scenarios have test mappings; only four have implementation mappings.
- A fresh provider rerun is queued behind the shared Cargo artifact lock in
  `/tmp/cmpct071-provider-recheck.log`; no new result is claimed.

## Outstanding integration

Promotion must consume the actual selected provider request before mutation.
The existing pin signature receives no agent, tools, next prompt or authoritative
suffix. Provider serialization happens later. A synthetic message envelope is not
a fit certificate. The same preparation/admission policy must guard physical
sends and continuations. The supplied typed API and requirements are documented
in `admission-api-handoff.md`.

Owner f5546042 controls055/059 promotion; main controls052. Those files were not
edited here. Requests for the integration seam returned busy-not-queued. Do not
mark071 done based on the standalone provider tests.

## CMPCT-051 audit

CMPCT-051 is already done and documentation-only. Its scope does not include071
or other runtime repairs. Fresh audit finds63 canonical Markdown documents,
80 Mermaid fences and31 registered051 snapshots; all31 byte-equal their
canonical documents and all canonical documents remain below300 lines.
The existing validation ledger records attachment/Mermaid validation and
explicitly excludes runtime certification. Audit recorded via fspec architecture
note; no new feature, Rust test or lifecycle transition is needed for051.
