# AUDIT-004 AST research (2026-11-07)

## `increment_step($$$A)`
```
rust/fspec-core/src/commands/reverse.rs:294:19:increment_step(session)
```

## `validate_completion($$$A)`
```
rust/fspec-core/src/commands/reverse.rs:266:9:validate_completion(&session)
```

## `set_strategy($$$A)`
```
rust/fspec-core/src/commands/reverse.rs:339:19:set_strategy(session, strategy, strategy_name, total_steps)
```

## `session_path($$$A)`
```
rust/fspec-core/src/types/reverse_session.rs:135:21:session_path(project_root)
rust/fspec-core/src/types/reverse_session.rs:147:18:session_path(project_root)
rust/fspec-core/src/types/reverse_session.rs:170:16:session_path(project_root)
rust/fspec-core/src/types/reverse_session.rs:180:34:session_path(project_root)
```

## `legacy_session_path($$$A)`
```
rust/fspec-core/src/types/reverse_session.rs:136:18:legacy_session_path(project_root)
rust/fspec-core/src/types/reverse_session.rs:146:18:legacy_session_path(project_root)
```

## `save_session($$$A)`
```
rust/fspec-core/src/commands/reverse.rs:295:5:save_session(project_root, &updated)
rust/fspec-core/src/commands/reverse.rs:340:5:save_session(project_root, &updated)
rust/fspec-core/src/commands/reverse.rs:426:5:save_session(project_root, &session)
```

## `load_session($$$A)`
```
rust/fspec-core/src/commands/reverse.rs:244:11:load_session(project_root)
rust/fspec-core/src/commands/reverse.rs:257:25:load_session(project_root)
rust/fspec-core/src/commands/reverse.rs:285:25:load_session(project_root)
rust/fspec-core/src/commands/reverse.rs:327:25:load_session(project_root)
rust/fspec-core/src/commands/reverse.rs:366:8:load_session(project_root)
```

## `relinked($$$A)`
```
rust/fspec-core/src/commands/audit/coverage.rs:180:31:relinked(&scenario.name, found)
rust/fspec-core/src/commands/audit/coverage.rs:220:23:relinked(&drift.scenario, found)
```

## `Relink::NotOnce($$$A)`
```
rust/fspec-core/src/commands/audit/relink.rs:112:19:Relink::NotOnce(
rust/fspec-core/src/commands/audit/relink.rs:136:19:Relink::NotOnce(block_count_fact(&candidates))
rust/fspec-core/src/commands/audit/relink.rs:146:19:Relink::NotOnce(format!(
```

## `FixKind::$V`
```
rust/fspec-core/src/commands/audit.rs:143:23:FixKind::Command
rust/fspec-core/src/commands/audit.rs:153:19:FixKind::Mechanical
```

