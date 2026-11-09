use super::*;

#[test]
fn args_parse_camel_case() {
    let a: DiscoverEventStormArgs = serde_json::from_str(r#"{"workUnitId":"AUTH-001"}"#).unwrap();
    assert_eq!(a.work_unit_id, "AUTH-001");
}

#[test]
fn wrap_in_system_reminder_wraps() {
    let r = wrap_in_system_reminder("BODY");
    assert!(r.starts_with("<system-reminder>\n"));
    assert!(r.ends_with("\n</system-reminder>"));
    assert!(r.contains("BODY"));
}

#[test]
fn guidance_is_non_empty() {
    assert!(event_storm_guidance().contains("Feature Event Storm"));
}

#[test]
#[serial_test::serial(capture_mode)]
fn event_storm_discovery_guidance_is_mode_aware() {
    // @step Given a work unit is in specifying status and FSPEC_CAPTURE_MODE is not set
    // @step When I dispatch discover-event-storm for it
    if crate::utils::mode::in_capture_mode() {
        return; // environment explicitly opts in; nothing to assert
    }
    let cli_guidance = event_storm_guidance();
    // @step Then the emitted guidance references `fspec astgrep` for the research-first step
    assert!(cli_guidance.contains("fspec astgrep --pattern"));
    assert!(!cli_guidance.contains("AstGrep(language="));

    // @step And when FSPEC_CAPTURE_MODE is set to "1" I dispatch discover-event-storm for it
    std::env::set_var("FSPEC_CAPTURE_MODE", "1");
    let capture_guidance = event_storm_guidance();
    std::env::remove_var("FSPEC_CAPTURE_MODE");
    // @step Then the emitted guidance references the `AstGrep` tool for the research-first step
    assert!(capture_guidance.contains("AstGrep(language="));
    assert!(!capture_guidance.contains("fspec astgrep"));
}
