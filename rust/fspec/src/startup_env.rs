//! PROV-097: dotenv-at-startup seam for the `fspec` binary.
//!
//! Feature: spec/features/fspec-binary-loads-dotenv-at-startup.feature
//!
//! The pure-Rust `fspec` binary loads `<cwd>/.env` into the process
//! environment once at startup — before `combined::run` / `daemon::run`
//! reach `ProviderCredentials::detect()` (which reads `std::env::var`).
//!
//! CWD-ONLY (TS parity): the seam uses `dotenvy::from_path(<cwd>/.env)`, NOT
//! `dotenvy::dotenv()` which walks UP parent dirs and would bleed an
//! ancestor `.env` (e.g. a dev repo-root file of real provider keys) into a
//! process whose own CWD has none — breaking spawn-based parity tests.
//! `from_path` reads strictly the current dir's `.env`, matching the TS
//! `join(process.cwd(), '.env')` and the [`load_dotenv_from`] test helper.
//!
//! The load is isolated here so an inline `#[cfg(test)]` test can exercise
//! the path-injectable variant directly: `codelet-fspec` is `[[bin]]`-only
//! (no `[lib]`), so integration tests cannot import crate internals. The
//! loaders are non-overriding (a shell-exported key wins) and
//! missing-file-tolerant (a missing `.env` returns `Err`, ignored).

/// Load `<cwd>/.env` at startup — CWD-only via `from_path` (no ancestor
/// walk; TS parity). Non-overriding and missing-file-tolerant: the `Err`
/// for a missing `.env` (or an already-exported key) is ignored so startup
/// never aborts.
pub fn load_startup_env() {
    let _ = dotenvy::from_path(std::path::Path::new(".env"));
}

/// Path-injectable variant of [`load_startup_env`]: load `<dir>/.env`.
/// Returns `Err` when the file is missing (callers may ignore). Compiled
/// only under `cfg(test)` since the binary calls `load_startup_env`.
#[cfg(test)]
pub fn load_dotenv_from(dir: &std::path::Path) -> Result<(), dotenvy::Error> {
    dotenvy::from_path(dir.join(".env"))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    use tempfile::TempDir;

    /// Env vars are process-global; serialize every env-mutating test.
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        ENV_MUTEX
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// RAII guard that sets/unsets an env var and restores its prior
    /// value on drop, so a test never leaks state into a sibling.
    struct EnvGuard {
        key: String,
        prev: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &str, val: &str) -> Self {
            let prev = std::env::var(key).ok();
            std::env::set_var(key, val);
            Self {
                key: key.to_string(),
                prev,
            }
        }

        fn unset(key: &str) -> Self {
            let prev = std::env::var(key).ok();
            std::env::remove_var(key);
            Self {
                key: key.to_string(),
                prev,
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.prev {
                Some(v) => std::env::set_var(&self.key, v),
                None => std::env::remove_var(&self.key),
            }
        }
    }

    /// RAII guard that switches the process CWD and restores it on drop.
    struct CwdGuard {
        prev: PathBuf,
    }

    impl CwdGuard {
        fn enter(dir: &Path) -> Self {
            let prev = std::env::current_dir().expect("read current dir");
            std::env::set_current_dir(dir).expect("set current dir");
            Self { prev }
        }
    }

    impl Drop for CwdGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.prev);
        }
    }

    // Scenario: A key present only in .env becomes visible to env detection
    #[test]
    fn scenario_a_key_present_only_in_env_becomes_visible_to_env_detection() {
        let _lock = env_lock();

        // @step Given a working directory containing a .env file with "ANTHROPIC_API_KEY=sk-ant-xyz"
        let dir = TempDir::new().expect("tempdir");
        std::fs::write(dir.path().join(".env"), "ANTHROPIC_API_KEY=sk-ant-xyz\n")
            .expect("write .env");

        // @step And the variable "ANTHROPIC_API_KEY" is not exported in the shell
        let _key = EnvGuard::unset("ANTHROPIC_API_KEY");

        // @step When the fspec startup dotenv-load runs in that working directory
        load_dotenv_from(dir.path()).expect("load .env");

        // @step Then "std::env::var" for "ANTHROPIC_API_KEY" returns "sk-ant-xyz"
        assert_eq!(
            std::env::var("ANTHROPIC_API_KEY").ok().as_deref(),
            Some("sk-ant-xyz")
        );
    }

    // Scenario: A shell-exported variable takes precedence over .env
    #[test]
    fn scenario_a_shell_exported_variable_takes_precedence_over_env() {
        let _lock = env_lock();

        // @step Given the variable "ANTHROPIC_API_KEY" is exported as "key-from-shell"
        let _key = EnvGuard::set("ANTHROPIC_API_KEY", "key-from-shell");

        // @step And a working directory containing a .env file with "ANTHROPIC_API_KEY=key-from-env-file"
        let dir = TempDir::new().expect("tempdir");
        std::fs::write(
            dir.path().join(".env"),
            "ANTHROPIC_API_KEY=key-from-env-file\n",
        )
        .expect("write .env");

        // @step When the fspec startup dotenv-load runs in that working directory
        let _ = load_dotenv_from(dir.path());

        // @step Then "std::env::var" for "ANTHROPIC_API_KEY" returns "key-from-shell"
        assert_eq!(
            std::env::var("ANTHROPIC_API_KEY").ok().as_deref(),
            Some("key-from-shell")
        );
    }

    // Scenario: A missing .env file is not an error
    #[test]
    fn scenario_a_missing_env_file_is_not_an_error() {
        let _lock = env_lock();

        // @step Given a working directory containing no .env file
        let dir = TempDir::new().expect("tempdir");

        // @step And the variable "ANTHROPIC_API_KEY" is exported as "exported-key"
        let _key = EnvGuard::set("ANTHROPIC_API_KEY", "exported-key");

        // @step When the fspec startup dotenv-load runs in that working directory
        let result = load_dotenv_from(dir.path());

        // @step Then the load completes without aborting startup
        assert!(
            result.is_err(),
            "a missing .env yields Err, which startup tolerates without aborting"
        );

        // @step And "std::env::var" for "ANTHROPIC_API_KEY" returns "exported-key"
        assert_eq!(
            std::env::var("ANTHROPIC_API_KEY").ok().as_deref(),
            Some("exported-key")
        );
    }

    // Scenario: A provider with a key only in .env is reported as configured
    #[test]
    fn scenario_a_provider_with_a_key_only_in_env_is_reported_as_configured() {
        let _lock = env_lock();

        // @step Given a working directory containing a .env file with "COHERE_API_KEY=co-test-key"
        let dir = TempDir::new().expect("tempdir");
        std::fs::write(dir.path().join(".env"), "COHERE_API_KEY=co-test-key\n")
            .expect("write .env");
        // Isolate custom-provider discovery onto the temp dir so the query
        // never reads the real home directory (offline + deterministic).
        let _home = EnvGuard::set("FSPEC_HOME", &dir.path().display().to_string());
        let _cwd = CwdGuard::enter(dir.path());

        // @step And the variable "COHERE_API_KEY" is not exported in the shell
        let _key = EnvGuard::unset("COHERE_API_KEY");

        // @step When the fspec startup dotenv-load runs in that working directory
        load_dotenv_from(dir.path()).expect("load .env");

        // @step And list_provider_credentials is queried
        let providers = codelet_providers::custom::management::list_providers_info()
            .expect("list providers info");

        // @step Then the "cohere" provider row reports configured as true
        let cohere = providers
            .iter()
            .find(|p| p.name == "cohere")
            .expect("cohere provider row present");
        assert!(
            cohere.available,
            "cohere must report configured (available) after .env load"
        );
    }

    fn crate_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    // Scenario: The fspec startup seam calls dotenvy from_path in startup_env and main.rs invokes it before clap parse
    #[test]
    fn scenario_the_fspec_startup_seam_calls_from_path_and_main_invokes_before_clap_parse() {
        // @step Given the file "rust/fspec/src/startup_env.rs"
        let startup_env = crate_dir().join("src").join("startup_env.rs");
        assert!(
            startup_env.is_file(),
            "rust/fspec/src/startup_env.rs must exist (missing: {})",
            startup_env.display()
        );
        let startup_body = std::fs::read_to_string(&startup_env).expect("read startup_env.rs");

        // @step When the source is scanned for "dotenvy::from_path"
        // Needle built dynamically (no self-match); `//` comments stripped (skip @step lines).
        let needle = format!("dotenvy::{}", "from_path");
        let code: String = startup_body
            .lines()
            .map(|l| l.split("//").next().unwrap_or(l))
            .collect::<Vec<_>>()
            .join("\n");
        let call_count = code.matches(&needle).count();

        // @step Then it contains a call to "dotenvy::from_path"
        assert!(
            call_count >= 1,
            "startup_env.rs must contain at least one `{needle}` call (cwd-only seam); found {call_count}"
        );

        // @step And the file "rust/fspec/src/main.rs" invokes "startup_env::load_startup_env()" before the clap parse of the Cli struct
        let main_rs = crate_dir().join("src").join("main.rs");
        let main_body = std::fs::read_to_string(&main_rs).expect("read main.rs");
        let invoke_idx = main_body
            .find("startup_env::load_startup_env()")
            .expect("main.rs must invoke startup_env::load_startup_env()");
        let parse_idx = main_body
            .find("Cli::try_parse(")
            .expect("main.rs must contain the Cli::try_parse( call");
        assert!(
            invoke_idx < parse_idx,
            "load_startup_env() must be invoked before Cli::try_parse() \
             (invoke at {invoke_idx}, parse at {parse_idx})"
        );
    }

    // Scenario: The fspec crate declares the dotenvy dependency
    #[test]
    fn scenario_the_fspec_crate_declares_the_dotenvy_dependency() {
        // @step Given the file "rust/fspec/Cargo.toml"
        let cargo = crate_dir().join("Cargo.toml");
        assert!(cargo.is_file(), "rust/fspec/Cargo.toml must exist");
        let body = std::fs::read_to_string(&cargo).expect("read fspec/Cargo.toml");

        // @step When the "[dependencies]" table is parsed
        // Scope to the [dependencies] body so the [dev-dependencies] dotenvy entry is excluded.
        let after = body
            .split_once("[dependencies]")
            .map(|(_, rest)| rest)
            .expect("[dependencies] section must exist");
        let deps = after
            .split_once("\n[")
            .map(|(head, _)| head)
            .unwrap_or(after);

        // @step Then it contains a key "dotenvy"
        let found = deps.lines().any(|l| {
            let t = l.trim();
            t.starts_with("dotenvy ") || t.starts_with("dotenvy.") || t.starts_with("dotenvy=")
        });
        assert!(found, "[dependencies] must declare a `dotenvy` key");
    }
}
