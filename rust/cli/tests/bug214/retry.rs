use super::{set_compactor_sub_agent_handler, Arc, DropSignal, Fixture, Mutex, TestResult, DAG};
use codelet_cli::compaction_operation::CompactionFailure;

impl Fixture {
    pub async fn retry(
        &mut self,
    ) -> Result<Result<bool, CompactionFailure>, Box<dyn std::error::Error>> {
        let agent = self.agent.as_ref().ok_or("fixture agent")?;
        let preparer = agent
            .selected_request_preparer(
                "resume original task",
                codelet_providers::request_admission::AdmissionLimits {
                    context_capacity: Some(16_000),
                    output_reservation: Some(4096),
                },
            )
            .await?;
        self.session.install_selected_preparer(preparer);
        Ok(
            codelet_cli::compactor_sub_agent::retry_failed_compactor_for_user_input(
                &mut self.session,
                self.manifest.id,
                &self.busy,
                &self.interrupted,
                &self.notify,
                &self.output,
            )
            .await,
        )
    }

    pub async fn fail_builder(&mut self) -> TestResult {
        set_compactor_sub_agent_handler(
            self.manifest.id,
            Some(Arc::new(|_, _| {
                Box::pin(async { Err("temporary builder failure".into()) })
            })),
        );
        let result = codelet_cli::compactor_sub_agent::run_compactor_sub_agent_round(
            &mut self.session,
            self.manifest.id,
            &Arc::new(Mutex::new(codelet_core::TokenState::default())),
            &self.busy,
            &self.output,
        )
        .await;
        assert!(matches!(result, Err(CompactionFailure::Builder(_))));
        let dropped = self.dropped.clone();
        set_compactor_sub_agent_handler(
            self.manifest.id,
            Some(Arc::new(move |_, _| {
                let dropped = dropped.clone();
                Box::pin(async move {
                    let _guard = DropSignal(dropped);
                    Ok(DAG.into())
                })
            })),
        );
        Ok(())
    }
}
