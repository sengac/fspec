use super::{assert_contract, Remote, TestResult};
use codelet_tools::tool_attribution::{configure_tool_server, SchemaMiddleware};
use rig::{
    completion::ToolDefinition,
    tool::{server::ToolServer, ToolDyn, ToolEmbeddingDyn, ToolError, ToolMiddleware, ToolSet},
    vector_store::{
        request::Filter, TopNResults, VectorSearchRequest, VectorStoreError, VectorStoreIndexDyn,
    },
    wasm_compat::WasmBoxedFuture,
};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

impl ToolEmbeddingDyn for Remote {
    fn context(&self) -> serde_json::Result<Value> {
        Ok(json!({}))
    }
    fn embedding_docs(&self) -> Vec<String> {
        vec!["remote tool".into()]
    }
}

struct Index;

impl VectorStoreIndexDyn for Index {
    fn top_n<'a>(
        &'a self,
        _request: VectorSearchRequest<Filter<Value>>,
    ) -> WasmBoxedFuture<'a, TopNResults> {
        Box::pin(async { Ok(Vec::new()) })
    }

    fn top_n_ids<'a>(
        &'a self,
        _request: VectorSearchRequest<Filter<Value>>,
    ) -> WasmBoxedFuture<'a, Result<Vec<(f64, String)>, VectorStoreError>> {
        Box::pin(async { Ok(vec![(1.0, "remote".into())]) })
    }
}

struct Probe {
    scope: String,
    calls: Arc<Mutex<Vec<String>>>,
}

impl ToolMiddleware for Probe {
    fn scope_id(&self) -> &str {
        &self.scope
    }

    fn decorate_definition(&self, definition: ToolDefinition) -> Result<ToolDefinition, ToolError> {
        SchemaMiddleware::new(&self.scope).decorate_definition(definition)
    }

    fn call<'a>(
        &'a self,
        tool: &'a dyn ToolDyn,
        args: String,
    ) -> WasmBoxedFuture<'a, Result<String, ToolError>> {
        Box::pin(async move {
            self.calls
                .lock()
                .map_err(|_| ToolError::ToolCallError("probe poisoned".into()))?
                .push(args.clone());
            let mut business: Value = serde_json::from_str(&args)?;
            if let Some(object) = business.as_object_mut() {
                object.remove("work_unit_id");
            }
            tool.call(business.to_string()).await
        })
    }
}

pub async fn exercise_lifecycle() -> TestResult {
    let original = json!({"type":"object","properties":{"business":{"type":"string"}}});
    let dynamic = ToolSet::builder()
        .dynamic_tool(Remote(original.clone()))
        .build();
    let actor = ToolServer::new().dynamic_tools(1, Index, dynamic).run();
    let calls = Arc::new(Mutex::new(Vec::new()));
    actor
        .configure_middleware(Arc::new(Probe {
            scope: "invoker".into(),
            calls: calls.clone(),
        }))
        .await?;
    assert!(configure_tool_server(&actor, "invoker").await.is_err());
    assert!(configure_tool_server(&actor, "foreign").await.is_err());
    let definitions = actor.get_tool_defs(Some("select dynamic".into())).await?;
    assert_eq!(definitions.len(), 1);
    assert_contract(&definitions[0]);
    let original_args = json!({"business":"unchanged","work_unit_id":"NONE"}).to_string();
    assert_eq!(
        actor.call_tool("remote", &original_args).await?,
        json!({"business":"unchanged"}).to_string()
    );
    assert_eq!(
        *calls.lock().map_err(|_| "calls poisoned")?,
        vec![original_args]
    );

    let mut additions = ToolSet::default();
    additions.add_tool(Remote(original.clone()));
    actor.append_toolset(additions).await?;
    assert_eq!(actor.get_tool_defs(None).await?.len(), 1);
    actor.add_tool(Remote(original.clone())).await?;
    assert_eq!(actor.get_tool_defs(None).await?.len(), 1);

    let collision = json!({"type":"object","properties":{"work_unit_id":{"type":"string"}}});
    assert!(actor.add_tool(Remote(collision.clone())).await.is_err());
    let before = actor.get_tool_defs(None).await?;
    assert_contract(&before[0]);
    let mut rejected = ToolSet::default();
    rejected.add_tool(Remote(collision));
    assert!(actor.append_toolset(rejected).await.is_err());
    assert_eq!(before, actor.get_tool_defs(None).await?);

    let mut foreign = ToolSet::default();
    foreign.add_tool(Remote(original.clone()));
    foreign
        .configure_middleware(Arc::new(SchemaMiddleware::new("foreign")))
        .await?;
    assert!(actor.append_toolset(foreign).await.is_err());
    assert_eq!(before, actor.get_tool_defs(None).await?);

    let mut direct = ToolSet::builder().dynamic_tool(Remote(original)).build();
    direct
        .configure_middleware(Arc::new(SchemaMiddleware::new("direct")))
        .await?;
    assert_contract(&direct.get_tool_definitions().await?[0]);
    assert_eq!(
        direct
            .call("remote", "{\"business\":\"same\"}".into())
            .await?,
        "{\"business\":\"same\"}"
    );
    Ok(())
}
