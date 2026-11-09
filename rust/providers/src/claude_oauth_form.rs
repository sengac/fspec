pub(super) fn build_form_html(auth_url: &str) -> String {
    format!(
        r#"<!doctype html>
<html>
  <head>
    <title>fspec - Claude OAuth Login</title>
    <style>
      body {{ font-family: system-ui, -apple-system, sans-serif; display: flex; justify-content: center; align-items: center; height: 100vh; margin: 0; background: #131010; color: #f1ecec; }}
      .container {{ text-align: center; padding: 2rem; max-width: 600px; }}
      h1 {{ color: #f1ecec; margin-bottom: 1rem; }}
      p {{ color: #b7b1b1; margin-bottom: 1.5rem; }}
      a {{ color: #7ab5ff; text-decoration: none; word-break: break-all; }}
      a:hover {{ text-decoration: underline; }}
      .form-group {{ margin: 1.5rem 0; }}
      input[type="text"] {{ width: 100%; padding: 0.75rem; border: 1px solid #444; border-radius: 0.5rem; background: #1a1717; color: #f1ecec; font-size: 1rem; box-sizing: border-box; font-family: monospace; }}
      input[type="text"]:focus {{ outline: none; border-color: #7ab5ff; }}
      button {{ padding: 0.75rem 2rem; border: none; border-radius: 0.5rem; background: #2563eb; color: white; font-size: 1rem; cursor: pointer; margin-top: 0.5rem; }}
      button:hover {{ background: #1d4ed8; }}
      .cancel {{ margin-top: 1rem; }}
      .cancel a {{ color: #888; font-size: 0.9rem; }}
    </style>
  </head>
  <body>
    <div class="container">
      <h1>Claude OAuth Login</h1>
      <p>1. Click the link below to authorize with Claude:</p>
      <p><a href="{auth_url}" target="_blank">{auth_url}</a></p>
      <p>2. After authorizing, copy the code from the callback page and paste it below:</p>
      <form method="POST" action="/submit">
        <div class="form-group">
          <input type="text" name="code" placeholder="Paste authorization code here (code#state)" autofocus />
        </div>
        <button type="submit">Submit Code</button>
      </form>
      <div class="cancel">
        <a href="/cancel">Cancel login</a>
      </div>
    </div>
  </body>
</html>"#
    )
}

pub(super) const HTML_CAPTURED: &str = r#"<!doctype html>
<html><head><title>fspec - Code Received</title></head>
<body><h1>Authorization Code Received</h1>
<p>Please return to fspec for the login result. Token exchange is not yet complete.</p>
</body></html>"#;

pub(super) const HTML_SUCCESS_CLAUDE: &str = r#"<!doctype html>
<html>
  <head>
    <title>fspec - Authorization Successful</title>
    <style>
      body { font-family: system-ui, -apple-system, sans-serif; display: flex; justify-content: center; align-items: center; height: 100vh; margin: 0; background: #131010; color: #f1ecec; }
      .container { text-align: center; padding: 2rem; }
      h1 { color: #f1ecec; margin-bottom: 1rem; }
      p { color: #b7b1b1; }
    </style>
  </head>
  <body>
    <div class="container">
      <h1>Authorization Successful</h1>
      <p>You can close this window and return to fspec.</p>
    </div>
    <script>setTimeout(() => window.close(), 2000)</script>
  </body>
</html>"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_form_html_contains_auth_url() {
        let html = build_form_html("https://claude.ai/oauth/authorize?test=1");
        assert!(html.contains("https://claude.ai/oauth/authorize?test=1"));
        assert!(html.contains("href="));
        assert!(html.contains("<form"));
    }

    #[test]
    fn test_build_form_html_uses_relative_urls() {
        let html = build_form_html("https://example.com");
        assert!(html.contains(r#"action="/submit""#));
        assert!(html.contains(r#"href="/cancel""#));
        assert!(!html.contains("localhost"));
    }
}
