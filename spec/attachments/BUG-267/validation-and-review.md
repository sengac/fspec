# BUG-267 validation and direct review

## Scope approval and ACDD

Native question `bug267_explicit_facade_scope` received **Approve bounded fix**
from the supervising caller under the user's explicit grant, not fabricated
human speech. Repair only implicit no-override Custom dispatch. Preserve native
routes, explicit facade routing (including incompatible native getter rejection),
shadowing disablement, credentials and admission. No broader MODEL-004 transport
policy, security policy or credential fallback is introduced.

Three rules, four examples and five Gherkin scenarios precede the tests and
production. `initial-red.log` is a compiler/fixture error (broadcast receives
session/chunk tuples), not causal red. After fixing the fixture,
`shared-loop-red.log` shows two genuine failures and three controls: selected
Custom claude and Custom copilot produce native-getter configuration errors and
zero requests. Noncolliding scripts, native/disabled-shadowing selections and
explicit incompatible override controls pass before production changes.

## Minimal production repair

The real locked shared dispatch reads the already selected ProviderType and
explicit facade state from the inner manager. A no-override Custom selection
routes directly to the existing custom builder with its exact slug. It is not
reclassified from a name or rediscovered from global config. Existing built-in
branches and explicit-override paths remain unchanged. Builder/compaction errors
still short-circuit before provider creation. The feature-gated stub route remains
special and is exercised with a real SessionManager round trip.

No alternate agent builder, new dependency, public API, endpoint, model policy,
tool surface or permission path is introduced. The one touched production file
is 123 lines. New tests and support files are all below 300 lines.

## Final-source evidence

`shared-loop-green.log`: all five acceptance tests pass, covering fifteen
selection cases: six colliding built-in names, four aliases, one noncolliding
script, three native/escape-hatch/compatible-explicit selections and one
incompatible explicit override. Fourteen successful cases each send exactly one
numeric-loopback inference request; the incompatible case sends none and retains
the exact native-getter rejection. The alias red first reaches copilot after
the other three alias controls already succeeded; the built-in-name red first
fails at claude. No claim is made that all later names separately ran red.

Tests run the actual background/shared loop after real selection resolution and
normal disposable admission. Captured request paths, script dialect marker,
resolved upstream model header, native dialect, mandatory attribution schema,
pre-request Done arming, accepted native Done, persisted billing (12 input / 3
output) and zero nudges are asserted. Config, HOME/credential directories and CWD
are temporary and restored. No external or real/billable provider request occurs.
The fixture's original script and native getter rejection are not mocked away.

`final-controls-quality.log`: five existing shadowing tests, ten existing script
tool-facade tests, all seven BUG-265 recovery tests, and the selected real
test-support stub round trip pass. Together with acceptance this is 28 distinct
tests passing. The stub process uses temporary HOME/FSPEC_HOME/FSPEC_USER_DIR/
CODEX_HOME, while Cargo/rustup use the already installed local caches; there is
no SDK/download. The metadata/dependency scenarios from that target are excluded;
only the exact stub runtime test is claimed. Default affected-library Cargo check
and scoped final Clippy pass. No full workspace-green claim is made.

Direct review covers typed selection ownership under the inner lock, builder
error order, explicit-facade precedence, feature-gated stub behavior, custom model
alias resolution, cancellation/fixture cleanup and real native Done admission.
Scoped formatting, whitespace, native feature/tag validation, coverage audit and
native review pass (`final-native-review.log`). All five scenarios map to their
real tests and changed shared dispatch. Native coverage was generated in a
disposable project containing only the exact feature to avoid unrelated global
sidecar mutations, then linked normally; no coverage JSON or status was edited.

No sessions/subagents, models, compaction actions, credentials, user config,
network/security settings, commits or publication are changed.
