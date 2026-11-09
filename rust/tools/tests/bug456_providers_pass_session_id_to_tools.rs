//! BUG-456 — every provider passes the session id to the tools it builds.
//!
//! Feature: spec/features/require-session-id-for-all-tools-to-support-worktree-isolation.feature

use std::path::Path;

type TestResult = Result<(), String>;

/// A provider's source as its module tree (the file and its submodules).
/// BUG-481: every test reads its provider through here, so the user
/// directory is isolated first (the BUG-347 rule for tests that name the
/// file tools).
fn provider_source(relative: &str) -> Result<String, String> {
    codelet_test_helpers::isolated_user_directory()?;
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../providers/src")
        .join(relative);
    codelet_test_helpers::source_composition::read_tree_or_file(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))
}

/// Every `<name>::new(` call in `source` passes `session_id` as its last
/// argument (parentheses balanced), and there is at least one.
fn constructed_with_session_id(source: &str, name: &str) -> TestResult {
    let opener = format!("{name}::new(");
    let mut calls = Vec::new();
    for (start, _) in source.match_indices(&opener) {
        let args_start = start + opener.len();
        let mut depth = 1usize;
        let mut end = None;
        for (offset, ch) in source[args_start..].char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(args_start + offset);
                        break;
                    }
                }
                _ => {}
            }
        }
        let end = end.ok_or(format!("unbalanced {name}::new( call"))?;
        calls.push(source[args_start..end].trim().trim_end_matches(',').trim());
    }
    if calls.is_empty() {
        return Err(format!("{name} is never constructed"));
    }
    match calls.iter().find(|args| !args.ends_with("session_id")) {
        Some(args) => Err(format!("{name} is constructed without session_id: {args}")),
        None => Ok(()),
    }
}

#[test]
fn claude_provider_must_pass_session_id_to_all_tools() -> TestResult {
    // @step Given a Claude provider creating an agent with session_id
    let source = provider_source("claude.rs")?;
    // @step When tools are instantiated for the agent
    // @step Then ReadTool MUST be constructed with ReadTool::new(session_id)
    constructed_with_session_id(&source, "ReadTool")?;
    // @step And WriteTool MUST be constructed with WriteTool::new(session_id)
    constructed_with_session_id(&source, "WriteTool")?;
    // @step And EditTool MUST be constructed with EditTool::new(session_id)
    constructed_with_session_id(&source, "EditTool")?;
    // @step And GrepTool MUST be constructed with GrepTool::new(session_id)
    constructed_with_session_id(&source, "GrepTool")?;
    // @step And GlobTool MUST be constructed with GlobTool::new(session_id)
    constructed_with_session_id(&source, "GlobTool")?;
    // @step And LsTool MUST be constructed with LsTool::new(session_id)
    constructed_with_session_id(&source, "LsTool")?;
    // @step And AstGrepTool MUST be constructed with AstGrepTool::new(session_id)
    constructed_with_session_id(&source, "AstGrepTool")?;
    // @step And AstGrepRefactorTool MUST be constructed with AstGrepRefactorTool::new(session_id)
    constructed_with_session_id(&source, "AstGrepRefactorTool")?;
    Ok(())
}

#[test]
fn openai_provider_must_pass_session_id_to_all_tools() -> TestResult {
    // @step Given an OpenAI provider creating an agent with session_id
    let source = provider_source("openai.rs")?;
    // @step When tools are instantiated for the agent
    // @step Then ReadTool MUST be constructed with ReadTool::new(session_id)
    constructed_with_session_id(&source, "ReadTool")?;
    // @step And WriteTool MUST be constructed with WriteTool::new(session_id)
    constructed_with_session_id(&source, "WriteTool")?;
    // @step And EditTool MUST be constructed with EditTool::new(session_id)
    constructed_with_session_id(&source, "EditTool")?;
    // @step And GrepTool MUST be constructed with GrepTool::new(session_id)
    constructed_with_session_id(&source, "GrepTool")?;
    // @step And GlobTool MUST be constructed with GlobTool::new(session_id)
    constructed_with_session_id(&source, "GlobTool")?;
    // @step And LsTool MUST be constructed with LsTool::new(session_id)
    constructed_with_session_id(&source, "LsTool")?;
    // @step And AstGrepTool MUST be constructed with AstGrepTool::new(session_id)
    constructed_with_session_id(&source, "AstGrepTool")?;
    // @step And AstGrepRefactorTool MUST be constructed with AstGrepRefactorTool::new(session_id)
    constructed_with_session_id(&source, "AstGrepRefactorTool")?;
    Ok(())
}

#[test]
fn codex_provider_must_pass_session_id_to_all_tools() -> TestResult {
    // @step Given a Codex provider creating an agent with session_id
    let source = provider_source("codex/agent.rs")?;
    // @step When tools are instantiated for the agent
    // @step Then every Codex tool facade wrapper MUST be constructed with session_id
    for wrapper in [
        "FileToolFacadeWrapper",
        "SearchToolFacadeWrapper",
        "LsToolFacadeWrapper",
        "BashToolFacadeWrapper",
        "ExecToolFacadeWrapper",
        "HitlToolFacadeWrapper",
    ] {
        constructed_with_session_id(&source, wrapper)?;
    }
    // @step And ApplyPatchTool MUST be constructed with ApplyPatchTool::new(session_id)
    constructed_with_session_id(&source, "ApplyPatchTool")?;
    Ok(())
}

#[test]
fn gemini_provider_must_pass_session_id_to_all_tools() -> TestResult {
    // @step Given a Gemini provider creating an agent with session_id
    let source = provider_source("gemini.rs")?;
    // @step When tools are instantiated for the agent
    // @step Then SearchToolFacadeWrapper MUST be constructed with SearchToolFacadeWrapper::new(session_id)
    constructed_with_session_id(&source, "SearchToolFacadeWrapper")?;
    // @step And LsToolFacadeWrapper MUST be constructed with LsToolFacadeWrapper::new(session_id)
    constructed_with_session_id(&source, "LsToolFacadeWrapper")?;
    // @step And AstGrepTool MUST be constructed with AstGrepTool::new(session_id)
    constructed_with_session_id(&source, "AstGrepTool")?;
    // @step And AstGrepRefactorTool MUST be constructed with AstGrepRefactorTool::new(session_id)
    constructed_with_session_id(&source, "AstGrepRefactorTool")?;
    // @step And all file operation wrappers MUST receive session_id
    for wrapper in [
        "FileToolFacadeWrapper",
        "SearchToolFacadeWrapper",
        "LsToolFacadeWrapper",
        "BashToolFacadeWrapper",
        "ExecToolFacadeWrapper",
        "HitlToolFacadeWrapper",
    ] {
        if source.contains(&format!("{wrapper}::new(")) {
            constructed_with_session_id(&source, wrapper)?;
        }
    }
    Ok(())
}

#[test]
fn zai_provider_must_pass_session_id_to_all_tools() -> TestResult {
    // @step Given a ZAI provider creating an agent with session_id
    let source = provider_source("zai.rs")?;
    // @step When tools are instantiated for the agent
    // @step Then SearchToolFacadeWrapper MUST be constructed with SearchToolFacadeWrapper::new(session_id)
    constructed_with_session_id(&source, "SearchToolFacadeWrapper")?;
    // @step And LsToolFacadeWrapper MUST be constructed with LsToolFacadeWrapper::new(session_id)
    constructed_with_session_id(&source, "LsToolFacadeWrapper")?;
    // @step And AstGrepTool MUST be constructed with AstGrepTool::new(session_id)
    constructed_with_session_id(&source, "AstGrepTool")?;
    // @step And AstGrepRefactorTool MUST be constructed with AstGrepRefactorTool::new(session_id)
    constructed_with_session_id(&source, "AstGrepRefactorTool")?;
    // @step And all file operation wrappers MUST receive session_id
    for wrapper in [
        "FileToolFacadeWrapper",
        "SearchToolFacadeWrapper",
        "LsToolFacadeWrapper",
        "BashToolFacadeWrapper",
        "ExecToolFacadeWrapper",
        "HitlToolFacadeWrapper",
    ] {
        if source.contains(&format!("{wrapper}::new(")) {
            constructed_with_session_id(&source, wrapper)?;
        }
    }
    Ok(())
}
