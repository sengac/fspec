use codelet_cli::session::Session;
use codelet_providers::{ProviderManager, ProviderType};
use std::io::Write;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<u8>>>);

impl Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .map_err(|_| std::io::Error::other("log lock"))?
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub fn logged(
    operation: impl FnOnce() -> u64,
) -> Result<(u64, String), Box<dyn std::error::Error>> {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(move || writer.clone())
        .finish();
    let basis = tracing::subscriber::with_default(subscriber, operation);
    let bytes = capture.0.lock().map_err(|_| "log lock")?.clone();
    Ok((basis, String::from_utf8(bytes)?))
}

pub fn session() -> Session {
    Session::from_provider_manager(ProviderManager::for_testing(
        ProviderType::OpenAI,
        Some(200_000),
        Some(4096),
    ))
}
