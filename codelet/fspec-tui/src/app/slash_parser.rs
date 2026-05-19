//! Slash command parser invoked by `handle_input_submitted` BEFORE
//! the text is forwarded to `backend.send_input`. Extracted from
//! `dispatch_rpc022.rs` to keep that file under the 300-LoC ceiling
//! after RPC-027 added `Action::SetThinkingLevelDefault` routing.

/// Outcome of parsing a single submitted input line. The
/// `handle_input_submitted` arm in `dispatch_rpc020.rs` branches over
/// this enum BEFORE forwarding plain text to `backend.send_input`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlashCommandParse {
    /// `/model` — open the ModelSelectorDialog.
    OpenModelDialog,
    /// `/thinking` — open the ThinkingLevelDialog.
    OpenThinkingDialog,
    /// `/role` or `/role clear` — clear the session role.
    ClearRole,
    /// `/role <text>` — set the session role to `text`.
    SetRole(String),
    /// Anything else — forward to `backend.send_input` as before.
    NotASlashCommand,
}

/// Inspect the submitted input text and return the slash command it
/// represents, if any. Public so unit tests can exercise the parser
/// without spinning up an App.
///
/// Trimming rules mirror the TS `/role` slash handler:
///   - "/model" → `OpenModelDialog`
///   - "/thinking" → `OpenThinkingDialog`
///   - "/role" (bare) → `ClearRole`
///   - "/role clear" (any case after trimming) → `ClearRole`
///   - "/role <text>" → `SetRole(text.trim())`
///   - "/role " (trailing space, empty arg) → `ClearRole`
///   - everything else → `NotASlashCommand`
pub fn parse_slash_command(text: &str) -> SlashCommandParse {
    let trimmed = text.trim();
    if trimmed == "/model" {
        return SlashCommandParse::OpenModelDialog;
    }
    if trimmed == "/thinking" {
        return SlashCommandParse::OpenThinkingDialog;
    }
    if trimmed == "/role" {
        return SlashCommandParse::ClearRole;
    }
    if let Some(rest) = trimmed.strip_prefix("/role ") {
        let arg = rest.trim();
        if arg.is_empty() || arg.eq_ignore_ascii_case("clear") {
            return SlashCommandParse::ClearRole;
        }
        return SlashCommandParse::SetRole(arg.to_string());
    }
    SlashCommandParse::NotASlashCommand
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn parse_slash_command_recognises_model_thinking_and_role_variants() {
        assert_eq!(parse_slash_command("/model"), SlashCommandParse::OpenModelDialog);
        assert_eq!(
            parse_slash_command("/thinking"),
            SlashCommandParse::OpenThinkingDialog
        );
        assert_eq!(parse_slash_command("/role"), SlashCommandParse::ClearRole);
        assert_eq!(
            parse_slash_command("/role clear"),
            SlashCommandParse::ClearRole
        );
        assert_eq!(
            parse_slash_command("/role CLEAR"),
            SlashCommandParse::ClearRole
        );
        assert_eq!(
            parse_slash_command("/role You are a security reviewer"),
            SlashCommandParse::SetRole("You are a security reviewer".to_string())
        );
        assert_eq!(
            parse_slash_command("/role  leading space ok"),
            SlashCommandParse::SetRole("leading space ok".to_string())
        );
        assert_eq!(
            parse_slash_command("hello world"),
            SlashCommandParse::NotASlashCommand
        );
        assert_eq!(
            parse_slash_command("/unknown anything"),
            SlashCommandParse::NotASlashCommand
        );
    }
}
