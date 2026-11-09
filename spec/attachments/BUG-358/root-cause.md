# BUG-358 root cause (draft)

`fspec validate-tags` checks every feature file. At 120952de, 539 files fail:
- 424 are missing a feature-group tag.
- 311 are missing a component tag.
- Some carry work-unit tags at scenario level (@HOOK-015 x19, @BOARD-023 x19, @RIG-014 x18, @HOOK-017 x18, @EXT-005 x14, ...).
- Some use unregistered tags (@list-mode, @keyboard, @core, ...).

Nothing runs validate-tags project-wide, so the violations piled up. Cards ran validate on single files only. The list of files is in violating-files.txt.
