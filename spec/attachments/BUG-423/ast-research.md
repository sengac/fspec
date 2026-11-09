# BUG-423 AST research

- rust/sessions/tests/prov141_session_creation_without_global_credentials.rs:127-165 (the profile scenario; assertion at 155).
- Session model resolution: SessionManagerHandle::get_session_model in rust/sessions/src/handle_impl/; profile-qualified provider handling in rust/sessions/src/handle_creation.rs and the providers profile resolution.
