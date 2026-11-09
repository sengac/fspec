//! BUG-449 — stage write permissions are enforced by every file-writing tool.
//!
//! Feature: spec/features/stage-write-permissions-are-enforced.feature

#[path = "bug231_support/mod.rs"]
#[allow(dead_code)]
mod fixture;

use codelet_tools::edit::EditArgs;
use codelet_tools::write::WriteArgs;
use codelet_tools::{EditTool, WriteTool};
use fixture::{Fixture, Result};
use rig::tool::Tool;

/// The fixture's first session, registered with the shared tool callbacks,
/// with its work unit in the specifying stage. RLCD is disabled so only the
/// stage decides (BUG-347 isolated user directory).
async fn specifying_session() -> Result<(Fixture, uuid::Uuid)> {
    codelet_test_helpers::isolated_user_directory()?;
    let fixture = Fixture::new().await?;
    codelet_sessions::session_tool_callbacks::register_manager(fixture.manager.clone());
    codelet_tools::facade::set_get_work_unit_stage_callback(
        codelet_sessions::session_tool_callbacks::work_unit_stage,
    );
    codelet_tools::facade::set_get_effective_cwd_callback(
        codelet_sessions::session_tool_callbacks::isolation_context,
    );
    let session = fixture.manager.get_session(&fixture.first.value)?;
    session.set_work_unit_context(
        Some("AUTH-001".into()),
        Some("bound".into()),
        Some("specifying".into()),
    )?;
    Ok((fixture, session.id))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn writing_and_editing_an_implementation_file_is_refused_while_the_card_is_specifying(
) -> Result {
    // @step Given a session whose work unit is in the specifying stage
    let (fixture, session) = specifying_session().await?;
    let file = fixture.project.path().join("src/production.rs");
    std::fs::create_dir_all(file.parent().ok_or("parent")?)?;
    std::fs::write(&file, "original")?;
    // @step When the Write and Edit tools target src/production.rs in the session's project
    let write = WriteTool::new(session)
        .call(WriteArgs {
            file_path: file.display().to_string(),
            content: "forbidden".into(),
        })
        .await;
    let edit = EditTool::new(session)
        .call(EditArgs {
            file_path: file.display().to_string(),
            old_string: "original".into(),
            new_string: "forbidden".into(),
        })
        .await;
    // @step Then both are refused naming the specifying stage and the file is unchanged
    for outcome in [write.map(|_| ()), edit.map(|_| ())] {
        let error = outcome.err().ok_or("the write was allowed")?.to_string();
        assert!(error.contains("specifying"), "{error}");
    }
    assert_eq!(std::fs::read_to_string(&file)?, "original");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn a_file_the_stage_allows_is_written() -> Result {
    // @step Given a session whose work unit is in the specifying stage
    let (fixture, session) = specifying_session().await?;
    let file = fixture.project.path().join("spec/features/login.feature");
    // @step When the Write tool targets spec/features/login.feature
    WriteTool::new(session)
        .call(WriteArgs {
            file_path: file.display().to_string(),
            content: "Feature: Login\n".into(),
        })
        .await?;
    // @step Then the file is written
    assert_eq!(std::fs::read_to_string(&file)?, "Feature: Login\n");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn a_failed_stage_lookup_refuses_the_write() -> Result {
    // @step Given a session the registered manager does not know
    let (fixture, _) = specifying_session().await?;
    let unknown = uuid::Uuid::new_v4();
    let file = fixture.project.path().join("src/production.rs");
    // @step When the Write tool targets src/production.rs
    let write = WriteTool::new(unknown)
        .call(WriteArgs {
            file_path: file.display().to_string(),
            content: "forbidden".into(),
        })
        .await;
    // @step Then the write is refused because the session's stage cannot be looked up
    assert!(write.is_err(), "{write:?}");
    assert!(!file.exists());
    Ok(())
}
