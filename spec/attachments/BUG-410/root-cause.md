# BUG-410 root cause

PROV-101 chose an empty id to mean 'declined, pick a model'. It is an in-band sentinel: every caller must know it, and the RPC create_session/create_session_checked return it as success. Fix: return a typed decline (an error the TUI recognises, e.g. a distinct message or an enum result) and update the TUI routing and the PROV-101/118/119 tests.
