# Recovery ownership completion

The existing `persistence-session-ownership.feature` and its two integration
scenarios preceded this repair. Executable RED is recorded in
`/tmp/bug246-session-ownership-red.log`: deleted-session admission incorrectly
succeeded, and unreadable committed ownership admitted a second original.
Both assertions fail against the original recovery implementation.

Recovery now uses core's existing `RecoveryLease`, not a duplicate file-lock
implementation. Open, admission, retention, counting and retry inspect committed
session ownership while holding that lease. Only an explicitly absent manifest
retires the session's durable lane and in-memory owner. Malformed or unreadable
ownership remains an error and preserves authentic originals for repair. The
redundant check-then-append manifest read is removed; configured transcript
append already acquires the canonical session writer transaction.

`/tmp/bug246-session-ownership-green.log` executes both scenarios successfully.
`/tmp/bug250-recovery-retirement-final-source-green.log` executes seven lifecycle
scenarios successfully, including real cold recovery, retained-memory retirement,
concurrent session ownership, and failures before and after deletion promotion.
Its ignored child fixture is explicitly executed by its parent tests.

These focused source integration runs compile the actual recovery module and
the actual complete core crate against existing matching dependencies. They are
supplemental runtime evidence, not substitutes for canonical Cargo validation.
The intermediate `/tmp/bug246-249-250-final-cargo.log` failed on an unrelated
sessions replay return-type error and is not a pass. The later canonical
`/tmp/bug246-250-final-integrated-cargo.log` (18:06) successfully executes both
ownership scenarios and the seven retirement parents against integrated source.
See `validation-evidence.md` for the remaining scoped quality gate.

Scenario mappings are unique and include the shared ownership implementation.
Unused cold-process helpers were separated from the shared fixture so the
ownership-only integration target does not carry dead helper functions.
No Cargo profile overrides, separate build directories, live-store mutations,
commits or fallback ownership paths are introduced.
