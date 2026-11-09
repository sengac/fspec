# BUG-417 root cause

Missing data-dir redirection in tests that create sessions. Fix: redirect to a temp data dir (serialised) as rpc440/tool027 tests do, seeding the default model explicitly so the outcome is deterministic.
