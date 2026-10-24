use super::{CodexProvider, CodexResponsesModel, CODEX_BASE_INSTRUCTIONS};

impl CodexProvider {
    pub fn with_recovery_handle(mut self, recovery: super::reconnect::CodexRecoveryHandle) -> Self {
        self.completion_model = self.completion_model.with_recovery_handle(recovery);
        self
    }

    pub fn completion_model(&self) -> CodexResponsesModel {
        self.completion_model.clone()
    }

    pub fn with_request_admission(
        mut self,
        limits: crate::request_admission::AdmissionLimits,
    ) -> Self {
        self.completion_model = self.completion_model.with_request_admission(limits);
        self
    }

    pub fn with_custom_tools(mut self, tools: super::custom_tools::CustomToolRegistry) -> Self {
        self.completion_model = self.completion_model.with_custom_tools(tools);
        self
    }

    pub fn with_metadata_handle(mut self, metadata: super::metadata::CodexMetadataHandle) -> Self {
        self.completion_model = self.completion_model.with_metadata_handle(metadata);
        self
    }

    pub fn with_routing_handle(mut self, routing: super::routing::CodexRoutingHandle) -> Self {
        self.completion_model = self.completion_model.with_routing_handle(routing);
        self
    }

    pub fn with_history(mut self, history: super::history::CodexHistory) -> Self {
        self.completion_model = self.completion_model.with_history(history);
        self
    }
    pub fn create_rig_agent(
        &self,
        session_id: uuid::Uuid,
        preamble: Option<&str>,
        thinking_config: Option<serde_json::Value>,
    ) -> rig::agent::Agent<CodexResponsesModel> {
        use codelet_tools::facade::{
            codex_bridge_tool, codex_fspec_tool, BashToolFacadeWrapper, CodexExecCommandFacade,
            CodexGrepFilesFacade, CodexListDirFacade, CodexReadFileFacade,
            CodexRequestUserInputFacade, CodexShellCommandFacade, CodexShellFacade,
            CodexViewImageFacade, CodexWriteStdinFacade, ExecToolFacadeWrapper,
            FileToolFacadeWrapper, HitlToolFacadeWrapper, LsToolFacadeWrapper,
            SearchToolFacadeWrapper,
        };
        use codelet_tools::{
            AgentManagerTool, ApplyPatchTool, AstGrepRefactorTool, AstGrepTool, ConnectMcpTool,
            DecisionTool, DeepSearchTool, GenerateCompactionTool, GraphSearchTool,
            InjectSummaryTool, ScheduleTool, SessionSearchTool, WebSearchTool,
        };
        use std::sync::Arc;

        let shell_command =
            BashToolFacadeWrapper::new(Arc::new(CodexShellCommandFacade), session_id);
        let read_file = FileToolFacadeWrapper::new(Arc::new(CodexReadFileFacade), session_id);
        let view_image = FileToolFacadeWrapper::new(Arc::new(CodexViewImageFacade), session_id);
        let list_dir = LsToolFacadeWrapper::new(Arc::new(CodexListDirFacade), session_id);
        let grep_files = SearchToolFacadeWrapper::new(Arc::new(CodexGrepFilesFacade), session_id);
        let shell = ExecToolFacadeWrapper::new(Arc::new(CodexShellFacade), session_id);
        let exec_command = ExecToolFacadeWrapper::new(Arc::new(CodexExecCommandFacade), session_id);
        let write_stdin = ExecToolFacadeWrapper::new(Arc::new(CodexWriteStdinFacade), session_id);
        let request_user_input =
            HitlToolFacadeWrapper::new(Arc::new(CodexRequestUserInputFacade), session_id);

        let mut agent_builder = rig::agent::AgentBuilder::new(self.completion_model.clone())
            .tool(shell_command)
            .tool(read_file)
            .tool(view_image)
            .tool(list_dir)
            .tool(grep_files)
            .tool(shell)
            .tool(exec_command)
            .tool(write_stdin)
            .tool(ApplyPatchTool::new(session_id))
            .tool(AstGrepTool::new(session_id))
            .tool(AstGrepRefactorTool::new(session_id))
            .tool(WebSearchTool::new(session_id))
            .tool(codex_fspec_tool(session_id))
            .tool(codex_bridge_tool(session_id))
            .tool(ConnectMcpTool::new(session_id))
            .tool(SessionSearchTool::new(session_id))
            .tool(GraphSearchTool::new(session_id))
            .tool(InjectSummaryTool::new(session_id))
            .tool(DeepSearchTool::new(session_id))
            .tool(DecisionTool::new(session_id))
            .tool(GenerateCompactionTool::new(session_id))
            .tool(AgentManagerTool::new(session_id))
            .tool(request_user_input)
            .tool(ScheduleTool::new(session_id));

        if codelet_tools::is_continue_armed(session_id) {
            agent_builder = agent_builder.tool(codelet_tools::DoneTool::new(session_id));
        }

        let effective_preamble = match preamble {
            Some(role) if !role.trim().is_empty() => format!("{CODEX_BASE_INSTRUCTIONS}\n\n{role}"),
            _ => CODEX_BASE_INSTRUCTIONS.to_string(),
        };
        agent_builder = agent_builder.preamble(&effective_preamble);
        if let Some(additional) = thinking_config {
            agent_builder = agent_builder.additional_params(additional);
        }
        agent_builder.build()
    }
}
