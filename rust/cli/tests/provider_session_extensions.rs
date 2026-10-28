use codelet_cli::session::Session;
use codelet_providers::session::{ExtensionError, ProviderSessionExtension, StateWriter};
use codelet_providers::{ProviderManager, ProviderType};
use serde_json::{json, Value};
use std::any::Any;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct IndependentExtension {
    events: Mutex<Vec<String>>,
    writer: Mutex<Option<StateWriter>>,
}

impl ProviderSessionExtension for IndependentExtension {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn restore(&self, envelopes: &[String]) -> Result<(), ExtensionError> {
        let state: Value = serde_json::from_str(&envelopes[0])?;
        assert_eq!(state["independentContinuation"], json!({"saved": true}));
        self.events
            .lock()
            .map_err(|error| ExtensionError::Persistence(error.to_string()))?
            .push("restore".into());
        Ok(())
    }
    fn set_writer(&self, writer: StateWriter) -> Result<(), ExtensionError> {
        *self
            .writer
            .lock()
            .map_err(|error| ExtensionError::Persistence(error.to_string()))? = Some(writer);
        Ok(())
    }
    fn begin_turn(&self) {
        if let Ok(mut events) = self.events.lock() {
            events.push("begin".into());
        }
    }
    fn reset(&self) -> Result<(), ExtensionError> {
        self.events
            .lock()
            .map_err(|error| ExtensionError::Persistence(error.to_string()))?
            .push("reset".into());
        Ok(())
    }
}

#[test]
#[serial_test::serial]
fn independent_provider_extensions_receive_session_lifecycle_events(
) -> Result<(), Box<dyn std::error::Error>> {
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    codelet_core::persistence::reset_stores_for_tests();
    let mut session = Session::from_provider_manager(ProviderManager::for_testing(
        ProviderType::OpenAI,
        Some(1000),
        Some(100),
    ));
    let extension = Arc::new(IndependentExtension::default());
    session
        .provider_session
        .register("independent", extension.clone());
    let manifest = codelet_core::persistence::create_session_with_provider(
        "independent",
        directory.path(),
        "independent",
    )?;
    let session_id = manifest.id;
    session
        .restore_provider_state(&[json!({"independentContinuation":{"saved":true}}).to_string()])?;
    session.bind_provider_persistence(session_id)?;
    session.begin_provider_turn();
    session.reset_provider_state()?;
    assert_eq!(
        *extension.events.lock().map_err(|error| error.to_string())?,
        ["restore", "begin", "reset"]
    );
    let writer = extension
        .writer
        .lock()
        .map_err(|error| error.to_string())?
        .clone()
        .ok_or("writer missing")?;
    writer(
        [("independentContinuation".into(), Value::Null)]
            .into_iter()
            .collect(),
    )?;
    let envelopes = codelet_core::persistence::get_session_message_envelopes(session_id)?;
    let saved: Value = serde_json::from_str(&envelopes[0])?;
    assert_eq!(saved["provider"], "independent");
    assert_eq!(saved.get("independentContinuation"), Some(&Value::Null));
    let other = Session::from_provider_manager(ProviderManager::for_testing(
        ProviderType::OpenAI,
        Some(1000),
        Some(100),
    ));
    assert!(other
        .provider_session
        .extension::<IndependentExtension>("independent")
        .is_none());
    Ok(())
}
