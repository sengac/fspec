# BUG-405 root cause

The NAPI front door drove `SessionManager::instance()`, so several lookups fall back to that singleton when no
manager was passed or registered. The NAPI crate is gone (there is no `rust/napi`), so the fallback now only hides a
missing registration: a front door that forgets to register its manager silently works against an empty singleton.
This breaks the no-fallbacks rule. BUG-398 removed the fallback in `session_tool_callbacks::session_by_id`; the
others remain (see ast-research.md). Fix: require the manager (constructor argument or registration that returns an
error when missing) and delete the `None => SessionManager::instance()` arms; decide whether
`SessionManager::instance()` itself can be deleted.
