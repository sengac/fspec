use std::collections::BTreeMap;
use std::fmt::{self, Write};
use std::sync::OnceLock;

use regex::Regex;
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::field::RecordFields;
use tracing_subscriber::fmt::format::{FormatEvent, FormatFields, Writer};
use tracing_subscriber::fmt::FmtContext;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{layer::Context, Layer};

#[path = "privacy_json.rs"]
mod json;

#[derive(Clone, Copy, Default)]
pub struct PrivacyFields {
    pub payloads: bool,
}

struct LocalSpan(DiagnosticFields);

pub struct PrivacySpanLayer(pub PrivacyFields);

impl<S> Layer<S> for PrivacySpanLayer
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        id: &tracing::Id,
        context: Context<'_, S>,
    ) {
        if let Some(span) = context.span(id) {
            let mut fields = DiagnosticFields::new(self.0);
            fields.update(attrs);
            span.extensions_mut().insert(LocalSpan(fields));
        }
    }

    fn on_record(
        &self,
        id: &tracing::Id,
        values: &tracing::span::Record<'_>,
        context: Context<'_, S>,
    ) {
        if let Some(span) = context.span(id) {
            if let Some(fields) = span.extensions_mut().get_mut::<LocalSpan>() {
                fields.0.update(values);
            }
        }
    }
}

impl PrivacyFields {
    pub fn local() -> Self {
        Self {
            payloads: std::env::var("FSPEC_LOG_PAYLOADS").is_ok_and(|value| value == "1"),
        }
    }
}

fn credential(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    [
        "authorization",
        "api_key",
        "apikey",
        "api-key",
        "password",
        "secret",
        "access_token",
        "refresh_token",
        "cookie",
    ]
    .iter()
    .any(|key| name.contains(key))
        || name == "token"
}

fn payload(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    if name.ends_with("_tokens") || name.starts_with("gen_ai.usage.") {
        return false;
    }
    [
        "system_instructions",
        "prompt",
        "completion",
        "messages",
        "arguments",
        "content",
    ]
    .iter()
    .any(|key| name.contains(key))
        || [
            "args",
            "result",
            "body",
            "data",
            "input",
            "output",
            "request",
            "response",
            "text",
            "part",
            "raw",
            "system",
            "instructions",
        ]
        .iter()
        .any(|key| {
            name == *key || name.ends_with(&format!(".{key}")) || name.ends_with(&format!("_{key}"))
        })
}

pub fn redact(value: &str) -> String {
    static HEADERS: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
    static BEARER: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
    static ASSIGNMENT: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
    let Ok(bearer) = BEARER.get_or_init(|| Regex::new(r"(?i)(bearer\s+)[A-Za-z0-9._~+/=-]+"))
    else {
        return "[REDACTED]".to_string();
    };
    let Ok(assignment) = ASSIGNMENT.get_or_init(|| Regex::new(
        r#"(?i)((?:\\?"|'|\b)(?:api[_-]?key|authorization|access[_-]?token|refresh[_-]?token|password|secret|cookie)(?:\\?"|')?\s*[:=]\s*)(?:\\?"[^"\r\n]*\\?"|'[^'\r\n]*'|[^\s,;}]+)"#
    )) else { return "[REDACTED]".to_string(); };
    let Ok(headers) = HEADERS.get_or_init(|| Regex::new(
        r#"(?i)((?:\\?["'])?(?:proxy-authorization|authorization|set-cookie|cookie)(?:\\?["'])?\s*[:=]\s*)(?:\\?"[^"\r\n]*\\?"|'[^'\r\n]*'|[^\r\n]+)"#
    )) else { return "[REDACTED]".to_string(); };
    let value = headers.replace_all(value, "${1}[REDACTED]");
    let value = bearer.replace_all(&value, "${1}[REDACTED]");
    assignment
        .replace_all(&value, "${1}[REDACTED]")
        .into_owned()
}

#[derive(Default)]
pub struct DiagnosticFields {
    values: BTreeMap<String, String>,
    policy: PrivacyFields,
    payload_message: bool,
}

impl DiagnosticFields {
    pub fn new(policy: PrivacyFields) -> Self {
        Self {
            policy,
            ..Self::default()
        }
    }

    fn record(&mut self, name: &str, value: &str) {
        if credential(name) {
            self.values
                .insert(name.to_string(), "[REDACTED]".to_string());
            return;
        }
        let sensitive = payload(name) || (name == "message" && self.payload_message);
        if sensitive && !self.policy.payloads {
            return;
        }
        let value = json::sanitize(value, self.policy.payloads);
        let value = if sensitive && value.chars().count() > 4096 {
            format!(
                "{}[TRUNCATED]",
                value.chars().take(4096).collect::<String>()
            )
        } else {
            value
        };
        self.values.insert(
            name.to_string(),
            value.replace('\n', "\\n").replace('\r', "\\r"),
        );
    }

    pub fn update(&mut self, fields: impl RecordFields) {
        fields.record(self);
    }

    pub fn render(&self) -> String {
        let mut output = self.values.get("message").cloned().unwrap_or_default();
        for (name, value) in &self.values {
            if name == "message" {
                continue;
            }
            if !output.is_empty() {
                output.push(' ');
            }
            let _ = write!(output, "{name}={value}");
        }
        output
    }

    fn render_and_clear_payloads(&mut self) -> String {
        let output = self.render();
        self.values.retain(|name, _| !payload(name));
        output
    }
}

impl Visit for DiagnosticFields {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.record(field.name(), value);
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if !self.policy.payloads
            && (payload(field.name()) || (field.name() == "message" && self.payload_message))
        {
            return;
        }
        if credential(field.name()) {
            self.record(field.name(), "");
            return;
        }
        let value = format!("{value:?}");
        let value = serde_json::from_str::<String>(&value).unwrap_or(value);
        self.record(field.name(), &value);
    }
}

pub fn event_fields(event: &Event<'_>, policy: PrivacyFields) -> DiagnosticFields {
    let metadata = event.metadata();
    let mut fields = DiagnosticFields {
        policy,
        payload_message: metadata.target().starts_with("rig")
            && matches!(
                *metadata.level(),
                tracing::Level::DEBUG | tracing::Level::TRACE
            ),
        ..DiagnosticFields::default()
    };
    event.record(&mut fields);
    fields
}

impl<'writer> FormatFields<'writer> for PrivacyFields {
    fn format_fields<R: RecordFields>(
        &self,
        mut writer: Writer<'writer>,
        fields: R,
    ) -> fmt::Result {
        let mut visitor = DiagnosticFields::default();
        visitor.update(fields);
        writer.write_str(&visitor.render())
    }
}

impl<S> FormatEvent<S, PrivacyFields> for PrivacyFields
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn format_event(
        &self,
        context: &FmtContext<'_, S, PrivacyFields>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        write!(
            writer,
            "{} {} {} ",
            chrono::Utc::now().to_rfc3339(),
            event.metadata().level(),
            event.metadata().target()
        )?;
        if let Some(scope) = context.event_scope() {
            for span in scope.from_root() {
                write!(writer, "{}{{", span.name())?;
                if let Some(fields) = span.extensions_mut().get_mut::<LocalSpan>() {
                    writer.write_str(&fields.0.render_and_clear_payloads())?;
                }
                writer.write_str("}: ")?;
            }
        }
        writeln!(writer, "{}", event_fields(event, *self).render())
    }
}
