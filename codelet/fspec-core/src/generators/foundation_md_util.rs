//! Shared JavaScript-semantics + Mermaid pre-check helpers for the
//! `FOUNDATION.md` generator. Extracted from `foundation_md.rs` to keep each
//! module under the 300-line standard. These reproduce the exact JS coercion
//! and truthiness rules the TypeScript `generateFoundationMd` relies on.

use serde_json::Value;

/// JavaScript truthiness for an optional `serde_json::Value`. `undefined`
/// (absent key -> `None`) and `null`/`false`/`0`/`""` are falsy; everything
/// else (objects, arrays, non-empty strings, non-zero numbers, `true`) is
/// truthy.
pub fn truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().map(|f| f != 0.0).unwrap_or(true),
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Array(_)) | Some(Value::Object(_)) => true,
    }
}

/// JavaScript `String(value)` / template-literal coercion for the value
/// shapes that reach FOUNDATION.md text. Strings pass through verbatim;
/// numbers/booleans/null use their JS string forms (integral numbers print
/// without a fractional part, matching `JSON.stringify`/`String()`).
pub fn js_str(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::String(s) => s.clone(),
        Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                if f.is_finite() && f.fract() == 0.0 {
                    return (f as i64).to_string();
                }
            }
            n.to_string()
        }
        Value::Array(_) | Value::Object(_) => v.to_string(),
    }
}

/// JavaScript strict equality (`===`) for the scalar shapes used by the
/// bounded-context id comparison. Numbers compare by value (`5 === 5.0`),
/// strings/bools by content, `null === null` is true; mismatched or
/// composite types are never equal.
pub fn js_strict_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => match (x.as_f64(), y.as_f64()) {
            (Some(p), Some(q)) => p == q,
            _ => false,
        },
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Null, Value::Null) => true,
        _ => false,
    }
}

/// Returns the array slice only when the value is a non-empty JS array,
/// matching `arr && arr.length > 0`.
pub fn nonempty_array(v: Option<&Value>) -> Option<&Vec<Value>> {
    match v.and_then(Value::as_array) {
        Some(a) if !a.is_empty() => Some(a),
        _ => None,
    }
}

/// Mermaid validation for the FOUNDATION.md generator. Delegates to the
/// shared [`crate::utils::mermaid_validation::validate_mermaid_syntax`], which
/// runs the canonical pure-string pre-checks (quoted subgraph title / invalid
/// subgraph identifier) and then the real `merman` parser. Returns
/// `Err(message)` for an invalid diagram; `Ok(())` otherwise.
pub fn validate_mermaid(code: &str) -> Result<(), String> {
    crate::utils::mermaid_validation::validate_mermaid_syntax(code)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use serde_json::json;

    #[test]
    fn truthy_matches_javascript_falsy_set() {
        assert!(!truthy(None));
        assert!(!truthy(Some(&Value::Null)));
        assert!(!truthy(Some(&json!(false))));
        assert!(!truthy(Some(&json!(0))));
        assert!(!truthy(Some(&json!(""))));
        assert!(truthy(Some(&json!("x"))));
        assert!(truthy(Some(&json!([]))));
    }

    #[test]
    fn js_str_integral_number_has_no_fraction() {
        assert_eq!(js_str(&json!(5)), "5");
        assert_eq!(js_str(&json!(2.5)), "2.5");
        assert_eq!(js_str(&json!("hi")), "hi");
    }

    #[test]
    fn js_strict_eq_number_value_equality() {
        assert!(js_strict_eq(&json!(5), &json!(5.0)));
        assert!(!js_strict_eq(&json!(5), &json!("5")));
        assert!(js_strict_eq(&Value::Null, &Value::Null));
    }

    #[test]
    fn validate_mermaid_rejects_quoted_subgraph() {
        let err = validate_mermaid("flowchart TB\n  subgraph \"Quoted\"\n  end").unwrap_err();
        assert!(err.contains("Quoted subgraph titles are not supported"));
    }

    #[test]
    fn validate_mermaid_accepts_generated_event_flow() {
        let code = "flowchart TB\n  subgraph Commands[\"\u{26a1} Commands\"]\n    C3[X]\n  end";
        assert!(validate_mermaid(code).is_ok());
    }
}
