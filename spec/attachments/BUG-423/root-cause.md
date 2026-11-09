# BUG-423 root cause

PROV-141 profile-model session reports a provider other than the profile name

rust/sessions/tests/prov141_session_creation_without_global_credentials.rs profile_model_session_creation_succeeds_without_global_credentials fails deterministically at 62fe1f807 (HEAD), three runs out of three: creation of default model openai:spark/qwen3.8-27b succeeds, but get_session_model(&sid).provider_id is not "spark" (assertion at line 155: provider must be the profile name). Either the session stores the base provider instead of the profile-qualified one, or the test's expectation is stale; find which and fix it.

Found while running the BUG-399 regression targets (2026-11-03); filed to the backlog untouched per the user's instruction.
