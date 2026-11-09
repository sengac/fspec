// BUG-523 (spec/features/commands-write-the-spec-directory-they-read.feature),
// included into common.rs.

/// BUG-523: the project root a spec command runs against, from the working
/// directory: the directory owning the spec/ the fspec-core loaders resolve
/// (walking up from a subdirectory such as rust/). Every bridge that calls
/// an fspec-core command uses this instead of the raw working directory,
/// because commands write `<root>/spec/...`; handed rust/, they read and
/// locked the repository's board and wrote a new rust/spec/work-units.json.
/// init is the exception and initialises the working directory itself.
///
/// WT-015 (product owner, 2026-11-08): the project's state root is resolved
/// here too, so a broken `.fspec/state-root` pointer fails every command,
/// readers included, with an error naming the pointer file.
pub fn spec_project_root() -> Result<PathBuf> {
    let cwd = std::env::current_dir().context("resolve current working directory")?;
    let root = codelet_fspec_core::io::project_root::resolve_project_root(&cwd);
    codelet_fspec_core::io::spec_paths::roots(&root)?;
    Ok(root)
}
