//! Feature: spec/features/codex-apply-patch.feature

use codelet_test_helpers::setup_temp_directory;
use codelet_tools::apply_patch::{ApplyPatchArgs, ApplyPatchTool};
use rig::tool::Tool;

#[tokio::test]
async fn update_a_file_using_bare_hunk_separators() -> Result<(), Box<dyn std::error::Error>> {
    let directory = setup_temp_directory()?;
    let path = directory.path().join("bare-hunks.txt");
    std::fs::write(&path, "header\nold first\nunchanged\nold second\nfooter\n")?;
    let patch = format!(
        "*** Begin Patch\n*** Update File: {}\n@@\n-old first\n+new first\n@@\n-old second\n+new second\n*** End Patch",
        path.display()
    );
    let result = ApplyPatchTool::new(uuid::Uuid::new_v4())
        .call(ApplyPatchArgs { patch })
        .await?;
    assert!(result.contains("Updated"));
    assert_eq!(
        std::fs::read_to_string(path)?,
        "header\nnew first\nunchanged\nnew second\nfooter\n"
    );
    Ok(())
}
