# BUG-419 root cause

The per-turn wiring substitutes literal values for absent profile settings at the call site. PROV-145's scenario
'A wiring built from absent values keeps the detector enabled' specifies the outcome, but the defaults live as
unwrap_or literals in the agent loop rather than one named, tested resolution (the no-silent-defaults rule).
Fix: resolve_profile_loop_detection returns concrete values with the documented defaults as named constants
(or an explicit Default for the resolved type), and the agent loop uses them without unwrap_or.
