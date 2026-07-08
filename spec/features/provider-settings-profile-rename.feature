@done
@profiles
@rust
@tui
@provider-settings
@PROV-136
Feature: Cannot edit the name of an existing custom OpenAI profile
  """
  Rename is implemented as delete-old-key + write-new-key in the profile persistence layer (read-modify-write of fspec-config.json), preserving customModels and sibling profiles. Collision check happens before the write: if the new name already exists as a different profile, reject.
  This deliberately DIVERGES from the TS reference which locks the name in edit mode. The name-editing gate flags (is_editing_name/is_new) and the 'name fixed' comments in profile_form.rs and mode.rs must be updated. EditProfile mode must track the original profile_name so rename can be detected on save.
  Rename backend is exposed as a new rpc/backend method (e.g. rename_profile(provider_id, old_name, new_name, definition)) OR by threading old_name through the SaveProfile action so handle_save_profile deletes the old key when it differs. The persistence primitives save_profile_at and delete_profile_at already exist in codelet/sessions/src/profile_persistence.rs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. In edit mode the user can move the cursor into the name field and edit the profile name
  #   2. Saving an edited profile with an unchanged name overwrites the same profile (no duplicate)
  #   3. Saving an edited profile with a changed name writes the new name and removes the old name
  #   4. A rename must not create a duplicate profile
  #   5. A rename preserves the profile connection fields (baseUrl, apiKey, contextWindow, maxOutputTokens, compaction threshold) and sibling keys like customModels
  #   6. An empty or whitespace-only name cannot be saved
  #   7. Renaming a profile onto an existing profile name is rejected (the existing different profile is never silently overwritten)
  #   8. In edit mode, Up arrow from the first connection field re-enters the editable name field (symmetric with create mode)
  #
  # EXAMPLES:
  #   1. Edit profile work-vllm, change name to work-vllm-2, save, config has work-vllm-2 with the same fields and no work-vllm
  #   2. Edit profile work-vllm, leave name unchanged, change apiKey, save, still one profile work-vllm with the updated apiKey
  #   3. Edit profile work-vllm, clear the name, save, the save is rejected because the name is required
  #   4. Rename work-vllm to fast where fast already exists, the rename is rejected and both profiles remain unchanged
  #   5. Rename a profile that has a customModels array, after rename the new-named profile still has its customModels
  #   6. In edit mode, press Up from the Base URL field to move into the name field, type to append to the name
  #
  # ========================================
  Background: User Story
    As a provider settings user editing an existing OpenAI profile
    I want to change the profile's name and save the rename
    So that I can correct or update a profile name without deleting and recreating it

  Scenario: Up arrow re-enters the name field in edit mode
    Given the edit profile form is open for the profile "work-vllm"
    When I press the Up arrow key
    Then the name field becomes editable
    Given the cursor is focused on the Base URL field
    When I type the character "2"
    Then the profile name becomes "work-vllm2"

  Scenario: An empty name cannot be saved
    Given the edit profile form is open for the profile "work-vllm"
    When the name is cleared and the form is submitted
    Then no save is performed
    Then the form stays open
