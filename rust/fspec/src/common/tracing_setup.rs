// Tracing subscribers (combined / daemon / client), the panic hook and
// the appender guard registry (REFAC-024: moved out of common.rs).

/// LOG-005: per-target directives applied on top of the base level.
///
/// `tarpc::client` and `tarpc::server` emit 6–8 `INFO` span events
/// per RPC round-trip (`SendRequest`, `ReceiveRequest`, `BeginRequest`,
/// `CompleteRequest`, `BufferResponse`, `SendResponse`,
/// `ReceiveResponse`). In a single TUI session this floods
/// `~/.fspec/logs/fspec-combined.log` — ~98% of lines were tarpc
/// ceremony — drowning out real diagnostic events. Pin those two
/// targets to `warn` by default. The user can still override via
/// `RUST_LOG=tarpc=info` if they want the full RPC trace.
const TARPC_QUIET_DIRECTIVES: &[&str] = &["tarpc::client=warn", "tarpc::server=warn"];

fn env_filter() -> EnvFilter {
    // If RUST_LOG is set, honour the user's directives verbatim — they
    // know what they're asking for. Otherwise fall back to
    // `info,tarpc::client=warn,tarpc::server=warn` so the file
    // appender isn't dominated by tarpc INFO span events.
    EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        let mut filter = EnvFilter::new("info");
        for directive in TARPC_QUIET_DIRECTIVES {
            filter = filter.add_directive(
                directive
                    .parse()
                    .expect("hardcoded tarpc directive must parse"),
            );
        }
        filter
    })
}

/// Combined mode: tracing → private rolling file under ~/.fspec/logs/private/ AND the
/// LogEvent broadcast inside SharedFspecService. NO stderr fmt subscriber
/// (stderr is reserved for the PORT banner + panic backtraces).
pub fn init_tracing_combined(service: &Arc<SharedFspecService>) -> Result<()> {
    let logs_dir = home_fspec_dir()?.join("logs/private");
    let appender = private_files::appender(&logs_dir, "fspec-combined.log")?;
    let (nb, guard) = tracing_appender::non_blocking(appender);
    keep_guard(guard);
    let file_layer = tracing_subscriber::fmt::layer()
        .fmt_fields(PrivacyFields::local())
        .event_format(PrivacyFields::local())
        .with_writer(nb)
        .with_ansi(false);
    tracing_subscriber::registry()
        .with(PrivacySpanLayer(PrivacyFields::local()))
        .with(BroadcastLogLayer)
        .with(file_layer)
        .with(env_filter())
        .init();
    let _ = register_log_layer(Arc::clone(service));
    Ok(())
}

/// Daemon mode: tracing → stderr fmt (RPC-005 pattern) AND the LogEvent
/// broadcast. Registry built FIRST, register_log_layer called SECOND so
/// only the sender-push side-effect of register_log_layer lands.
pub fn init_tracing_daemon(service: &Arc<SharedFspecService>) -> Result<()> {
    let fmt_layer = tracing_subscriber::fmt::layer()
        .fmt_fields(PrivacyFields::default())
        .event_format(PrivacyFields::default())
        .with_writer(std::io::stderr);
    tracing_subscriber::registry()
        .with(PrivacySpanLayer(PrivacyFields::default()))
        .with(BroadcastLogLayer)
        .with(fmt_layer)
        .with(env_filter())
        .init();
    let _ = register_log_layer(Arc::clone(service));
    Ok(())
}

/// Client mode: tracing → rolling file under ~/.fspec/logs/private/client.log; no
/// SharedFspecService exists here (the daemon owns it).
pub fn init_tracing_client() -> Result<()> {
    let dir = home_fspec_dir()?.join("logs/private");
    let appender = private_files::appender(&dir, "client.log")?;
    let (nb, guard) = tracing_appender::non_blocking(appender);
    keep_guard(guard);
    let file_layer = tracing_subscriber::fmt::layer()
        .fmt_fields(PrivacyFields::local())
        .event_format(PrivacyFields::local())
        .with_writer(nb)
        .with_ansi(false);
    tracing_subscriber::registry()
        .with(file_layer)
        .with(PrivacySpanLayer(PrivacyFields::local()))
        .with(env_filter())
        .init();
    Ok(())
}

/// Idempotent panic hook that logs the panic (BUG-610) and restores the
/// terminal BEFORE the default hook prints the backtrace, so a panic
/// during TUI mode doesn't leave the alt-screen wedged and is still in the
/// log once the terminal is gone.
pub fn install_panic_hook() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let prev = std::panic::take_hook();
        std::panic::set_hook(codelet_common::panic_log::wrap(prev, || {
            ratatui::restore();
            let _ = crossterm::execute!(
                std::io::stdout(),
                crossterm::event::DisableMouseCapture,
                crossterm::event::DisableBracketedPaste,
            );
        }));
    });
}

// Keep the non-blocking tracing-appender WorkerGuard alive for the whole
// process (otherwise the worker thread is dropped immediately and log
// writes get lost). Stored in a Once-initialised static.
fn keep_guard(guard: tracing_appender::non_blocking::WorkerGuard) {
    use std::sync::Mutex;
    static GUARDS: Mutex<Vec<tracing_appender::non_blocking::WorkerGuard>> = Mutex::new(Vec::new());
    if let Ok(mut g) = GUARDS.lock() {
        g.push(guard);
    }
}
