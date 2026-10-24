use codelet_agent_loop::session_search_handler::create_handler;
use codelet_core::persistence::{
    append_message_with_metadata, create_session, reset_stores_for_tests, SessionManifest,
};
use codelet_tools::session_search::{
    set_session_search_handler, types::SessionSearchAction, SessionSearchHandler,
};
use serde_json::{json, Value};
use std::sync::{atomic::AtomicBool, Arc};
use uuid::Uuid;

pub struct Fixture {
    pub directory: tempfile::TempDir,
    pub session: SessionManifest,
    pub handler: SessionSearchHandler,
}

pub fn blob_envelope(hash: &str) -> Value {
    let mut original = envelope(&codelet_core::persistence::make_blob_reference(hash));
    original["_blobRefs"] = json!([{"key":"tool_result:0:call-1", "hash":hash}]);
    original
}

impl Fixture {
    pub fn new() -> Self {
        let directory =
            codelet_test_helpers::setup_temp_directory().expect("create data directory");
        codelet_common::set_data_directory(directory.path().to_path_buf())
            .expect("redirect persistence to test directory");
        reset_stores_for_tests();
        let session =
            create_session("original evidence", directory.path()).expect("create session");
        let handler = create_handler(
            directory.path().to_path_buf(),
            Arc::new(AtomicBool::new(true)),
        );
        set_session_search_handler(session.id, Some(handler.clone()));
        Self {
            directory,
            session,
            handler,
        }
    }

    pub fn append(&mut self, envelope: Value) -> Uuid {
        append_message_with_metadata(
            &mut self.session,
            "user",
            "searchable preview",
            serde_json::from_value(envelope).expect("envelope is an object"),
        )
        .expect("persist original metadata")
    }

    pub fn request(&self, overrides: Value) -> Value {
        let mut request = json!({"action_type":"originals", "session_id":self.session.id});
        for (key, value) in overrides
            .as_object()
            .expect("request overrides are an object")
        {
            request[key] = value.clone();
        }
        self.call(request)
    }

    pub fn call(&self, request: Value) -> Value {
        let action: SessionSearchAction = serde_json::from_value(request)
            .expect("SessionSearch must accept the explicit original retrieval action");
        serde_json::to_value((self.handler)(action, self.session.id))
            .expect("serialize tool result")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        set_session_search_handler(self.session.id, None);
        reset_stores_for_tests();
    }
}

pub fn envelope(text: &str) -> Value {
    json!({
        "uuid":Uuid::new_v4(), "parentUuid":Uuid::new_v4(),
        "timestamp":"2026-10-17T00:00:00Z", "type":"user", "provider":"tool",
        "message":{"role":"user", "content":[{
            "type":"tool_result", "tool_use_id":"call-1", "content":text,
            "provider_extension":{"nested":[null, 9, {"evidence":"unchanged"}]}
        }], "provider_message_field":{"retain":true}},
        "unknown_top_level":{"retain":[1,2,3]}
    })
}

pub fn assert_error(result: &Value, code: &str) {
    assert_eq!(result["result_type"], "originals_error", "{result}");
    assert_eq!(result["error"]["code"], code, "{result}");
    assert!(
        result.get("records").is_none(),
        "errors must not return partial evidence"
    );
}
