use crate::ProviderError;

const UPDATE_GUIDANCE: &str = "Install a current Claude Code release or run `claude update`. If FSPEC_CLAUDE_CODE_VERSION is configured, update or remove that authoritative deployment override. No bundled version fallback is used.";

pub fn user_agent() -> Result<String, ProviderError> {
    let version = match std::env::var("FSPEC_CLAUDE_CODE_VERSION") {
        Ok(version) => version,
        Err(std::env::VarError::NotPresent) => installed_version()?,
        Err(std::env::VarError::NotUnicode(_)) => {
            return Err(configuration_error("Deployment version is not valid UTF-8"));
        }
    };
    let mut components = version.split('.');
    let release = [
        component(components.next()),
        component(components.next()),
        component(components.next()),
    ];
    let [Some(major), Some(minor), Some(patch)] = release else {
        return Err(configuration_error(
            "Expected a major.minor.patch release version",
        ));
    };
    if components.next().is_some() {
        return Err(configuration_error(
            "Expected exactly three release components",
        ));
    }
    if [major, minor, patch] < [2, 1, 280] {
        return Err(configuration_error("Claude Code must be at least 2.1.280"));
    }
    Ok(format!("claude-cli/{version} (external, cli)"))
}

fn component(value: Option<&str>) -> Option<u64> {
    let value = value?;
    if value.is_empty()
        || !value.bytes().all(|byte| byte.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return None;
    }
    value.parse().ok()
}

fn installed_version() -> Result<String, ProviderError> {
    let output = std::process::Command::new("claude")
        .arg("--version")
        .output()
        .map_err(|_| configuration_error("Cannot execute `claude --version` on PATH"))?;
    if !output.status.success() {
        return Err(configuration_error(
            "`claude --version` exited unsuccessfully",
        ));
    }
    let output = String::from_utf8(output.stdout)
        .map_err(|_| configuration_error("Claude Code version output is not valid UTF-8"))?;
    output
        .trim()
        .strip_suffix(" (Claude Code)")
        .map(str::to_owned)
        .ok_or_else(|| configuration_error("Unrecognized `claude --version` output"))
}

fn configuration_error(reason: &str) -> ProviderError {
    ProviderError::config("claude", format!("{reason}. {UPDATE_GUIDANCE}"))
}

pub(crate) fn annotate_version_error(error: rig::http_client::Error) -> rig::http_client::Error {
    match error {
        rig::http_client::Error::InvalidStatusCodeWithMessage(status, body) => {
            let rejection = serde_json::from_str::<serde_json::Value>(&body).ok();
            if rejection
                .as_ref()
                .and_then(|value| value.pointer("/error/details/error_code"))
                .and_then(serde_json::Value::as_str)
                == Some("claude_code_version_too_old")
            {
                rig::http_client::Error::InvalidStatusCodeWithMessage(
                    status,
                    format!("{body}\n{UPDATE_GUIDANCE}"),
                )
            } else {
                rig::http_client::Error::InvalidStatusCodeWithMessage(status, body)
            }
        }
        other => other,
    }
}
