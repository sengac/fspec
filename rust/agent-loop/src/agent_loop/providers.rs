use crate::{background_output::BackgroundOutput, run_with_provider};
use codelet_cli::{interactive::BridgeImage, session::Session};
use codelet_sessions::background_session::BackgroundSession;

pub(super) async fn run_selected_provider(
    inner_session: &mut Session,
    input: &str,
    bridge_images: Option<Vec<BridgeImage>>,
    session: &BackgroundSession,
    output: &BackgroundOutput,
    thinking_config_value: Option<serde_json::Value>,
    current_provider: &str,
    current_model: Option<String>,
    builder_retry: Result<bool, codelet_cli::compaction_operation::CompactionFailure>,
) -> anyhow::Result<()> {
    match current_provider {
        _ if builder_retry.is_err() => builder_retry.map(|_| ()).map_err(anyhow::Error::new),
        "claude" => run_with_provider!(
            inner_session,
            get_claude,
            input,
            bridge_images.clone(),
            session,
            output,
            thinking_config_value
        ),
        "openai" => {
            super::provider_openai::run_openai_turn(
                inner_session,
                input,
                bridge_images,
                session,
                output,
                thinking_config_value,
            )
            .await
        }
        "gemini" => run_with_provider!(
            inner_session,
            get_gemini,
            input,
            bridge_images.clone(),
            session,
            output,
            thinking_config_value
        ),
        "zai" => run_with_provider!(
            inner_session,
            get_zai,
            input,
            bridge_images,
            session,
            output,
            thinking_config_value
        ),
        "codex" => run_with_provider!(
            inner_session,
            get_codex,
            input,
            bridge_images.clone(),
            session,
            output,
            thinking_config_value
        ),
        "github-copilot" | "copilot" => run_with_provider!(
            inner_session,
            get_github_copilot,
            input,
            bridge_images.clone(),
            session,
            output,
            thinking_config_value
        ),
        #[cfg(feature = "test-support")]
        "stub" => {
            super::provider_custom::run_stub_turn(
                inner_session,
                input,
                bridge_images,
                session,
                output,
                thinking_config_value,
                current_provider,
            )
            .await
        }
        _ => {
            super::provider_custom::run_custom_turn(
                inner_session,
                input,
                bridge_images,
                session,
                output,
                thinking_config_value,
                current_provider,
                current_model,
            )
            .await
        }
    }
}
