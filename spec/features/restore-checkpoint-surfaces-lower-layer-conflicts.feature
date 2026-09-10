@WT-010
@git
@checkpoint-management
@bug-fix
Feature: Restore Checkpoint Surfaces Lower-Layer Conflicts
  """
  fspec-core restore-checkpoint (restore_checkpoint.rs::restore_util) delegates the actual tree restoration to codelet_git::ghost_commit::restore_ghost_commit. WT-010 makes that lower layer return Err(GitError::RestoreConflict { files }) when force=false and the workdir diverges from the checkpoint. restore_util must map that specific variant to a conflict result (success:false, conflicts_detected:true, conflicted_files=files, systemReminder=CHECKPOINT RESTORATION CONFLICT DETECTED reminder) instead of falling into the Err(_) => not_found_result arm. Only a genuinely missing ref (GitError::Other 'Checkpoint not found ...') keeps the not-found sentinel.
  """

  Background: User Story
    As a developer using fspec checkpoints
    I want the restore-checkpoint command to report a lower-layer conflict as a conflict result instead of a false 'checkpoint not found'
    So that conflict messages reach the user with the right message even when the git working tree is clean

  Scenario: The restore-checkpoint command surfaces a lower-layer conflict as a conflict result, not a not-found sentinel
    Given a git repository where test.txt was committed as 'v1' and a ghost commit checkpoint was taken, then test.txt was changed to 'v2' and committed so the working tree reports clean
    When the dispatcher receives restore-checkpoint for that checkpoint with force false and no userChoice and the dirty check is reported clean
    Then the result reports success false, conflictsDetected true, conflictedFiles containing test.txt, and a CHECKPOINT RESTORATION CONFLICT DETECTED system reminder
