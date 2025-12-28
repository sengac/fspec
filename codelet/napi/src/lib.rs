//! Codelet NAPI-RS Native Module Bindings
//!
//! This module exposes codelet's Rust AI agent functionality to Node.js via NAPI-RS.
//! It enables fspec's Ink/React TUI to serve as the frontend for codelet.
//!
//! Uses the same streaming infrastructure as codelet-cli but with callbacks
//! instead of stdout printing.

#[cfg_attr(not(feature = "noop"), macro_use)]
extern crate napi_derive;

// These modules use ThreadsafeFunction and must be excluded in noop mode
#[cfg(not(feature = "noop"))]
mod models;
#[cfg(not(feature = "noop"))]
mod output;
#[cfg(not(feature = "noop"))]
mod session;
#[cfg(not(feature = "noop"))]
mod thinking_config;
#[cfg(not(feature = "noop"))]
mod types;

// Persistence module works in both modes (pure Rust with optional NAPI bindings)
pub mod persistence;

pub use persistence::*;

#[cfg(not(feature = "noop"))]
pub use models::*;
#[cfg(not(feature = "noop"))]
pub use session::CodeletSession;
#[cfg(not(feature = "noop"))]
pub use thinking_config::{
    extract_thinking_text, get_thinking_config, is_thinking_content, JsThinkingLevel,
};
#[cfg(not(feature = "noop"))]
pub use types::*;

// Logging callback infrastructure for routing Rust tracing logs to TypeScript
#[cfg(not(feature = "noop"))]
mod logging {
    use napi::threadsafe_function::{
        ThreadsafeFunction, ThreadsafeFunctionCallMode, UnknownReturnValue,
    };
    use napi::Status;
    use std::sync::Mutex;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    /// Type alias for log callback - matches NAPI v3 signature
    type LogCallback = ThreadsafeFunction<String, UnknownReturnValue, String, Status, false>;

    lazy_static::lazy_static! {
        static ref LOG_CALLBACK: Mutex<Option<LogCallback>> = Mutex::new(None);
        static ref SUBSCRIBER_INITIALIZED: Mutex<bool> = Mutex::new(false);
    }

    /// Custom tracing layer that sends logs to TypeScript
    struct TypeScriptLayer;

    impl<S> tracing_subscriber::Layer<S> for TypeScriptLayer
    where
        S: tracing::Subscriber,
    {
        fn on_event(
            &self,
            event: &tracing::Event<'_>,
            _ctx: tracing_subscriber::layer::Context<'_, S>,
        ) {
            // Extract the message from the event
            let mut visitor = MessageVisitor::default();
            event.record(&mut visitor);

            let level = match *event.metadata().level() {
                tracing::Level::ERROR => "ERROR",
                tracing::Level::WARN => "WARN",
                tracing::Level::INFO => "INFO",
                tracing::Level::DEBUG => "DEBUG",
                tracing::Level::TRACE => "TRACE",
            };

            let message = visitor.message.unwrap_or_default();
            let target = event.metadata().target();

            if let Ok(guard) = LOG_CALLBACK.lock() {
                if let Some(ref callback) = *guard {
                    let log_msg = format!("[RUST:{}] [{}] {}", level, target, message);
                    let _ = callback.call(log_msg, ThreadsafeFunctionCallMode::NonBlocking);
                }
            }
        }
    }

    #[derive(Default)]
    struct MessageVisitor {
        message: Option<String>,
    }

    impl tracing::field::Visit for MessageVisitor {
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            if field.name() == "message" {
                self.message = Some(format!("{:?}", value));
            } else if self.message.is_none() {
                // Capture first field as message if no explicit message field
                self.message = Some(format!("{:?}", value));
            }
        }

        fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
            if field.name() == "message" || self.message.is_none() {
                self.message = Some(value.to_string());
            }
        }
    }

    /// Set the logging callback from TypeScript and initialize the tracing subscriber
    #[napi]
    pub fn set_rust_log_callback(callback: LogCallback) {
        // Store the callback
        if let Ok(mut guard) = LOG_CALLBACK.lock() {
            *guard = Some(callback);
        }

        // Initialize tracing subscriber only once
        if let Ok(mut initialized) = SUBSCRIBER_INITIALIZED.lock() {
            if !*initialized {
                let _ = tracing_subscriber::registry()
                    .with(TypeScriptLayer)
                    .try_init();
                *initialized = true;
            }
        }
    }
}

#[cfg(not(feature = "noop"))]
pub use logging::set_rust_log_callback;
