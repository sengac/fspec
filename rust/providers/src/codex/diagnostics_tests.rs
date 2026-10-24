use super::*;
use proptest::prelude::*;
use serde_json::json;

proptest! {
    #[test]
    fn only_the_declared_exact_code_classifies_context_rejection(code in ".{0,512}", message in ".{0,4096}") {
        let diagnostic = CodexRejection::backend(json!({"code":code,"message":message}), &Redactor::default());
        prop_assert_eq!(diagnostic.class == RejectionClass::ContextLengthExceeded, code == "context_length_exceeded");
        prop_assert!(diagnostic.body.len() <= MAX_DIAGNOSTIC_BYTES);
        prop_assert!(diagnostic.code.as_ref().is_none_or(|value| value.len() <= MAX_FIELD_BYTES));
        prop_assert!(diagnostic.message.as_ref().is_none_or(|value| value.len() <= MAX_MESSAGE_BYTES));
    }

    #[test]
    fn encoded_credentials_are_not_retained(secret in "[a-z]{12}\\\\[a-z]{12}") {
        let mut redactor = Redactor::default();
        redactor.add(&secret);
        let diagnostic = CodexRejection::backend(json!({"code":"bad_request","message":secret}), &redactor);
        let evidence = format!("{diagnostic} {diagnostic:?}");
        prop_assert!(!evidence.contains(&secret));
        prop_assert!(!diagnostic.body.contains(&secret.replace('\\', "\\\\")));
    }

    #[test]
    fn individual_cookie_values_are_redacted(secret in "[a-z]{16}") {
        let mut headers = http::HeaderMap::new();
        headers.insert("cookie", format!("first=public; session=\"{secret}\"").parse().expect("valid ASCII cookie header"));
        let redactor = Redactor::from_headers(&headers);
        prop_assert_eq!(redactor.clean(&secret, 8192), "[redacted]");
    }

    #[test]
    fn padded_authentication_schemes_do_not_hide_credentials(secret in "[a-z]{16}") {
        for name in ["authorization", "proxy-authorization"] {
            let mut headers = http::HeaderMap::new();
            headers.insert(name, format!(" \tBearer  {secret}\t ").parse().expect("valid padded authorization"));
            let redactor = Redactor::from_headers(&headers);
            prop_assert_eq!(redactor.clean(&secret, 8192), "[redacted]");
        }
    }

    #[test]
    fn unicode_escaped_echoes_are_redacted_before_bounding(secret in "[a-z]{16}", prefix in "[0-9]{0,16}") {
        let mut redactor = Redactor::default();
        redactor.add(&secret);
        let escaped: String = secret.chars().map(|character| format!("\\u{:04x}", u32::from(character))).collect();
        prop_assert_eq!(redactor.clean_body(&format!("{prefix}{escaped}"), 8192, false), format!("{prefix}[redacted]"));
        prop_assert_eq!(redactor.clean_fragment(&format!("{prefix}{}", &escaped[..escaped.len() - 6]), 8192), format!("{prefix}[redacted]"));
    }

    #[test]
    fn sensitive_header_values_are_redacted(secret in "[a-z]{16}") {
        let mut value = http::HeaderValue::from_str(&secret).expect("valid ASCII header");
        value.set_sensitive(true);
        let mut headers = http::HeaderMap::new();
        headers.append("x-private-token", value);
        let redactor = Redactor::from_headers(&headers);
        prop_assert_eq!(redactor.clean(&secret, 8192), "[redacted]");
    }
}

#[test]
fn opaque_header_values_are_redacted() {
    let mut headers = http::HeaderMap::new();
    headers.append(
        "authorization",
        http::HeaderValue::from_bytes("Bearer privé-token".as_bytes())
            .expect("valid opaque header"),
    );
    let redactor = Redactor::from_headers(&headers);
    assert_eq!(redactor.clean("privé-token", 8192), "[redacted]");
}

#[test]
fn proxy_credentials_are_redacted_without_the_authentication_scheme() {
    let mut headers = http::HeaderMap::new();
    headers.append(
        "proxy-authorization",
        "Basic private-proxy-token"
            .parse()
            .expect("valid proxy authorization"),
    );
    let redactor = Redactor::from_headers(&headers);
    assert_eq!(redactor.clean("private-proxy-token", 8192), "[redacted]");
}

#[test]
fn a_secret_starting_at_an_incomplete_escape_is_redacted() {
    let mut redactor = Redactor::default();
    redactor.add("😀");
    assert_eq!(
        redactor.clean_fragment(r"error: \uD83D\uDE0", 8192),
        "error: [redacted]"
    );
}

#[tokio::test]
async fn a_secret_cut_within_a_utf8_character_is_redacted() -> Result<(), Box<dyn std::error::Error>>
{
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let mut redactor = Redactor::default();
    redactor.add("token-é");
    let prefix = "x".repeat(MAX_DIAGNOSTIC_BYTES - 7);
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(400).set_body_string(format!("{prefix}token-é")))
        .expect(1)
        .mount(&server)
        .await;
    let response = reqwest::get(server.uri()).await?;
    let diagnostic = CodexRejection::http(response, &redactor, 1, Duration::from_secs(1)).await;
    assert_eq!(diagnostic.state, EvidenceState::Truncated);
    assert!(diagnostic.body.starts_with(&prefix));
    assert!(!diagnostic.body.contains("token-"));
    assert_eq!(diagnostic.body.len(), MAX_DIAGNOSTIC_BYTES);
    server.verify().await;
    Ok(())
}

#[test]
fn overlapping_credential_spans_are_redacted_together() {
    let mut redactor = Redactor::default();
    redactor.add("0123456789A");
    redactor.add("A987654321");
    assert_eq!(redactor.clean("0123456789A987654321", 8192), "[redacted]");
    assert_eq!(
        redactor.clean_body(r"0123456789A\u003987654321", 8192, false),
        "[redacted]"
    );
    redactor.add("ababa");
    assert_eq!(redactor.clean("abababa", 8192), "[redacted]");
}

#[test]
fn escaped_surrogate_pairs_and_literal_backslashes_are_redacted() {
    let mut redactor = Redactor::default();
    redactor.add("private-😸-credential");
    redactor.add(r"private-\u0061-credential");
    for body in [
        r"private-\uD83D\uDE38-credential",
        r"private-\\u0061-credential",
    ] {
        assert_eq!(redactor.clean_body(body, 8192, false), "[redacted]");
    }
    for body in [r"private-\uD83D", r"private-\uD83D\uDE"] {
        assert_eq!(redactor.clean_fragment(body, 8192), "[redacted]");
    }
    assert_eq!(
        redactor.clean_body(r"diagnostic \q \uZZZZ", 8192, false),
        r"diagnostic \q \uZZZZ"
    );
}

#[test]
fn a_secret_cut_at_the_evidence_boundary_is_redacted() {
    let mut redactor = Redactor::default();
    redactor.add("long-private-credential");
    assert_eq!(
        redactor.clean_fragment("prefix long-private", 8192),
        "prefix [redacted]"
    );
    assert_eq!(
        redactor.clean_fragment(r"prefix \u006cong-priva\u007", 8192),
        "prefix [redacted]"
    );
}
