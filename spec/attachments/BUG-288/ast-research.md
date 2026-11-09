# AST research: BUG-288
```
core/src/persistence/manifest_commit.rs:88:9:self.leases.insert(session_id, lease.clone())

sessions/src/session_manager/manager_impl_6.rs:7:5:pub fn destroy_session(&self, id: &str) -> Result<(), String> {

```
