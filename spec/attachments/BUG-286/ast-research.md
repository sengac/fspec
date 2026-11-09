# AST research: BUG-286
```
fspec-core/src/commands/validate_work_units.rs:239:9:validate_string_array(&mut errors, id, "rules", wu.get("rules"))
fspec-core/src/commands/validate_work_units.rs:240:9:validate_string_array(&mut errors, id, "examples", wu.get("examples"))
fspec-core/src/commands/validate_work_units.rs:242:9:validate_string_array(&mut errors, id, "assumptions", wu.get("assumptions"))
fspec-core/src/commands/validate_work_units.rs:248:9:validate_string_array(&mut errors, id, "blocks", wu.get("blocks"))
fspec-core/src/commands/validate_work_units.rs:249:9:validate_string_array(&mut errors, id, "blockedBy", wu.get("blockedBy"))
fspec-core/src/commands/validate_work_units.rs:250:9:validate_string_array(&mut errors, id, "dependsOn", wu.get("dependsOn"))
fspec-core/src/commands/validate_work_units.rs:251:9:validate_string_array(&mut errors, id, "relatesTo", wu.get("relatesTo"))

```
