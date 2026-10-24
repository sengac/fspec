use codelet_providers::{custom::CustomProvider, custom_provider_registered, LlmProvider};
use codelet_tools::{
    AstGrepTool, BashTool, GlobTool, GrepTool, LsTool, ReadTool, SessionSearchTool,
};
use rig::client::CompletionClient;
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};
use tracing::Instrument;
use uuid::Uuid;

use crate::compaction_evidence::CompactionEvidence;
use crate::deep_search_provider_config::request_config_for_provider;

pub const COMPACTION_SUB_AGENT_MAX_DEPTH: usize = 20;

struct SearchRegistration(Uuid);

impl Drop for SearchRegistration {
    fn drop(&mut self) {
        codelet_tools::set_session_search_handler(self.0, None);
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn execute_compaction_subagent(
    project_path: &Path,
    target_session_id: Uuid,
    _existing_dag: Option<(String, usize)>,
    compaction_in_progress: &Arc<AtomicBool>,
    provider_name: &str,
    model_id: Option<&str>,
    context_window: Option<usize>,
    max_output_tokens: Option<usize>,
) -> Result<String, String> {
    let manifest = codelet_core::persistence::load_session(target_session_id)?;
    let source = codelet_core::persistence::compaction_boundary::capture_compaction_source(
        &manifest,
        model_id.unwrap_or("default"),
        "full-current-dag",
    )
    .map_err(|error| error.to_string())?;
    execute_captured_compaction_subagent(
        project_path,
        CompactionEvidence::capture(source)?,
        None,
        compaction_in_progress,
        provider_name,
        model_id,
        context_window,
        max_output_tokens,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn execute_captured_compaction_subagent(
    project_path: &Path,
    evidence: CompactionEvidence,
    owner: Option<Uuid>,
    _compaction_in_progress: &Arc<AtomicBool>,
    provider_name: &str,
    model_id: Option<&str>,
    context_window: Option<usize>,
    max_output_tokens: Option<usize>,
) -> Result<String, String> {
    let target_session_id = evidence.source().session_id;
    let deadline = codelet_cli::compaction_timing::deadline().unwrap_or_else(|| {
        tokio::time::Instant::now() + codelet_cli::interactive::deep_search_wall_clock_timeout()
    });
    let ephemeral = Uuid::new_v4();
    let _admission = match owner {
        Some(owner) => codelet_tools::tool_admission::register_auxiliary_for_owner(
            ephemeral,
            project_path.to_path_buf(),
            owner,
        ),
        None => {
            codelet_tools::tool_admission::register_auxiliary(ephemeral, project_path.to_path_buf())
        }
    }
    .map_err(|error| error.to_string())?;
    let _processes = codelet_tools::unified_exec::ProcessOwnerScope::new(ephemeral);
    codelet_tools::set_session_search_handler(
        ephemeral,
        Some(evidence.search_handler(project_path.to_path_buf())),
    );
    let _registration = SearchRegistration(ephemeral);
    let mut prompt = codelet_cli::compaction_dag::build_generate_compaction_prompt(
        target_session_id,
        evidence.existing_dag(),
    );
    prompt.push_str(&format!(
        "\n\nThis operation covers exactly {} captured persisted originals, starting at turn 0. \
         SessionSearch is bounded to this prefix, including continuation pages. \
         Do not include subsequently appended records in the DAG.",
        evidence.source().prefix.len(),
    ));
    codelet_cli::compaction_timing::scope(
        ephemeral,
        deadline,
        codelet_cli::interactive::deep_search_wall_clock_timeout(),
        tokio::time::timeout_at(
            deadline,
            codelet_cli::compaction_operation::build_validated_dag(
                &prompt,
                &codelet_core::compaction::DagValidationContext {
                    persisted_turn_count: evidence.source().prefix.len(),
                    required_coverage_end: evidence.source().prefix.len().checked_sub(1),
                    existing_dag: evidence.existing_dag().map(|(content, _)| content),
                    ..Default::default()
                },
                |task| async move {
                    Box::pin(run(
                        ephemeral,
                        project_path,
                        &task,
                        target_session_id,
                        provider_name,
                        model_id,
                        context_window,
                        max_output_tokens,
                    ))
                    .await
                },
            ),
        ),
    )
    .instrument(
        tracing::info_span!("compaction_builder", session_id = %target_session_id,
        provider = provider_name, model = model_id.unwrap_or("default")),
    )
    .await
    .map_err(|_| "compaction builder deadline exceeded".to_string())?
}

#[allow(clippy::too_many_arguments)]
async fn run(
    ephemeral: Uuid,
    project_path: &Path,
    prompt: &str,
    target: Uuid,
    provider_name: &str,
    model_id: Option<&str>,
    context_window: Option<usize>,
    max_output_tokens: Option<usize>,
) -> Result<String, String> {
    const _: () = assert!(codelet_tools::SUB_AGENT_TOOL_COUNT == 7);
    let task = format!("Target session: {target}\n\n{prompt}");
    let limits = codelet_providers::request_admission::AdmissionLimits {
        context_capacity: context_window.map(|capacity| capacity as u64),
        output_reservation: max_output_tokens.map(|tokens| tokens as u64),
    };
    if custom_provider_registered(provider_name) {
        let handle = CustomProvider::create_rig_agent(
            project_path,
            provider_name,
            model_id.unwrap_or("default"),
            ephemeral,
            Some(prompt),
            None,
            true,
        )
        .map_err(|error| format!("Failed to build custom-provider compactor sub-agent: {error}"))?;
        return Box::pin(crate::compaction_execution::run(
            handle.into_inner(),
            ephemeral,
            &task,
            provider_name,
            COMPACTION_SUB_AGENT_MAX_DEPTH,
            limits,
        ))
        .await;
    }
    let manager = codelet_providers::ProviderManager::with_provider_and_model(
        provider_name,
        model_id,
        context_window,
        max_output_tokens,
    )
    .map_err(|error| format!("Failed to create ProviderManager: {error}"))?;
    macro_rules! build_and_run {
        ($provider:expr, $oauth:expr $(, $builder_kind:ident)?) => {{
            let provider = $provider.map_err(|error| error.to_string())?;
            let config = request_config_for_provider(
                provider_name,
                provider.model(),
                prompt,
                $oauth(&provider),
            )
            .map_err(|error| error.to_string())?;
            let mut builder = crate::auxiliary_agent_builder::auxiliary_agent_builder!(
                provider $(, $builder_kind)?
            )
            .preamble(&config.preamble);
            if let Some(maximum) = config.max_tokens {
                builder = builder.max_tokens(limits.output_reservation.map_or(maximum, |reservation| maximum.min(reservation)));
            }
            if let Some(parameters) = config.additional_params {
                builder = builder.additional_params(parameters);
            }
            let agent = builder
                .tool(ReadTool::new(ephemeral))
                .tool(GrepTool::new(ephemeral))
                .tool(AstGrepTool::new(ephemeral))
                .tool(GlobTool::new(ephemeral))
                .tool(LsTool::new(ephemeral))
                .tool(BashTool::new(ephemeral))
                .tool(SessionSearchTool::new(ephemeral))
                .build();
            Box::pin(crate::compaction_execution::run(
                agent, ephemeral, &task, provider_name, COMPACTION_SUB_AGENT_MAX_DEPTH, limits,
            )).await
        }};
    }
    match provider_name {
        "claude" => build_and_run!(manager.get_claude(), |provider: &codelet_providers::ClaudeProvider| provider.is_oauth_mode()),
        "openai" => build_and_run!(manager.get_openai(ephemeral), |_| false),
        "gemini" => build_and_run!(manager.get_gemini(), |_| false),
        "codex" => build_and_run!(manager.get_codex(), |_| false, codex),
        "zai" => build_and_run!(manager.get_zai(), |_| false),
        "github-copilot" | "copilot" => Err("github-copilot compactor sub-agent builder pending: CopilotProvider::client accessor not yet implemented".into()),
        _ => Err(format!("Unsupported provider for compactor sub-agent: {provider_name}")),
    }
}
