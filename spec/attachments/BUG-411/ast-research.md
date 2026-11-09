# BUG-411 AST research

manager_impl_3.rs: if let Err(e) = crate::credentials::resolve_and_set_env_var(...) { tracing::error!(...) }; manager_impl_1.rs:139 if let Err(e) = save_default_model(model) { tracing::warn!(...) }; callers of set_default_model: grep -rn 'set_default_model(' rust --include=*.rs.
