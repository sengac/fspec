# MODEL-016 AST research (2026-11-05)

```
== ThinkingTarget::for_provider_manager($$$A)
rust/agent-loop/src/agent_loop/routing.rs:17:27:ThinkingTarget::for_provider_manager(inner.provider_manager())
== ThinkingTarget::new($$$A)
rust/agent-loop/tests/model010_thinking_levels_reach_each_provider.rs:69:5:ThinkingTarget::new(
rust/agent-loop/tests/model010_thinking_levels_reach_each_provider.rs:449:19:ThinkingTarget::new(
rust/agent-loop/tests/bug440_openai_compatible_thinking_level.rs:19:18:ThinkingTarget::new(
rust/agent-loop/tests/bug442_gemini_thinking_for_every_gemini_id.rs:24:18:ThinkingTarget::new(Ok(ThinkingWire::Gemini), &id, model, Ok(capability))
rust/agent-loop/tests/bug455_sessions_without_a_chosen_thinking_level.rs:18:5:ThinkingTarget::new(
rust/agent-loop/tests/bug441_codex_reasoning_for_every_codex_model.rs:17:18:ThinkingTarget::new(
rust/agent-loop/tests/model013_prompt_text_never_changes_the_thinking_level.rs:17:5:ThinkingTarget::new(
== $M.set_model_direct_with_profile($$$A)
rust/agent-loop/tests/bug215_effective_compaction_limits.rs:61:9:inner.provider_manager_mut().set_model_direct_with_profile(
rust/providers/src/codex/manager_admission_tests.rs:46:9:manager.set_model_direct_with_profile("codex", model, None, None, None, None)
rust/providers/src/codex/manager_admission_tests.rs:83:5:manager.set_model_direct_with_profile(
rust/providers/src/manager/tests_7.rs:14:5:manager
rust/providers/src/manager/tests_6.rs:112:5:manager
rust/providers/src/manager/tests_6.rs:147:5:manager
rust/providers/src/manager/impl_2.rs:143:9:self.set_model_direct_with_profile(
rust/providers/tests/prov161_scripted_dispatch.rs:70:9:manager.set_model_direct_with_profile("scripted", "smart", profile, None, None, None)
rust/providers/tests/prov161_runtime_resolution.rs:68:13:manager.set_model_direct_with_profile(
rust/providers/tests/prov161_runtime_resolution.rs:82:9:manager.set_model_direct_with_profile(
rust/providers/tests/prov161_runtime_resolution.rs:165:5:manager.set_model_direct_with_profile(
rust/providers/tests/prov161_runtime_resolution.rs:175:5:manager.set_model_direct_with_profile(
rust/providers/tests/prov161_runtime_resolution.rs:237:9:manager.set_model_direct_with_profile(
rust/providers/tests/prov161_runtime_resolution.rs:247:5:manager.set_model_direct_with_profile(
rust/providers/tests/prov161_runtime_resolution.rs:256:5:manager.set_model_direct_with_profile(
rust/providers/tests/prov161_runtime_resolution.rs:287:5:manager.set_model_direct_with_profile(
rust/sessions/src/profile_model_limits.rs:41:5:manager
rust/sessions/tests/bug168_model_vision_resolution.rs:87:5:pm.set_model_direct_with_profile(
rust/sessions/tests/bug168_model_vision_resolution.rs:115:5:pm.set_model_direct_with_profile("openai", "qwen-plain", Some("qwen-plain"), None, None, None)
rust/sessions/tests/prov145_loop_detection_resolution.rs:44:5:pm.set_model_direct_with_profile("openai", "qwen", Some("work-vllm"), None, None, None)
rust/sessions/tests/prov145_loop_detection_resolution.rs:66:5:pm.set_model_direct_with_profile("openai", "qwen", Some("legacy"), None, None, None)
rust/sessions/tests/prov145_loop_detection_resolution.rs:109:5:pm.set_model_direct_with_profile("openai", "model-a", Some("loose"), None, None, None)
rust/sessions/tests/prov145_loop_detection_resolution.rs:118:5:pm.set_model_direct_with_profile("openai", "model-b", Some("tight"), None, None, None)
rust/sessions/tests/prov144_max_images_resolution.rs:36:5:pm.set_model_direct_with_profile("openai", "qwen", Some("work-vllm"), None, None, None)
rust/sessions/tests/prov144_max_images_resolution.rs:59:5:pm.set_model_direct_with_profile("openai", "qwen", Some("no-vision"), None, None, None)
rust/sessions/tests/prov144_max_images_resolution.rs:82:5:pm.set_model_direct_with_profile("openai", "qwen", Some("legacy"), None, None, None)
rust/sessions/tests/prov144_max_images_resolution.rs:125:5:pm.set_model_direct_with_profile("openai", "model-a", Some("vision"), None, None, None)
rust/sessions/tests/prov144_max_images_resolution.rs:134:5:pm.set_model_direct_with_profile("openai", "model-b", Some("no-vision"), None, None, None)
== apply_profile_model($$$A)
== ReasoningCapability::resolve($$$A)
rust/agent-loop/src/thinking_config.rs:123:17:ReasoningCapability::resolve(&model, info.reasoning_options.as_deref())
rust/agent-loop/tests/bug442_gemini_thinking_for_every_gemini_id.rs:23:9:ReasoningCapability::resolve(&id, Some(options.as_slice()))
rust/providers/src/models/reasoning.rs:223:9:ReasoningCapability::resolve(
== effort_wire($$$A)
rust/agent-loop/src/thinking_config.rs:180:25:effort_wire(wire, "none")
rust/agent-loop/src/thinking_config.rs:237:21:effort_wire(wire, name.as_str())
== load_local_server_profiles()
rust/sessions/src/profile_sections.rs:142:20:load_local_server_profiles()
rust/sessions/tests/prov145_loop_detection_persistence.rs:264:20:load_local_server_profiles()
rust/sessions/tests/prov145_loop_detection_persistence.rs:313:20:load_local_server_profiles()
rust/sessions/tests/prov144_max_images_persistence.rs:183:20:load_local_server_profiles()
rust/sessions/tests/prov144_max_images_persistence.rs:210:20:load_local_server_profiles()
rust/sessions/tests/prov175_profile_numbers_are_validated.rs:30:18:load_local_server_profiles()
rust/sessions/tests/prov175_profile_numbers_are_validated.rs:45:18:load_local_server_profiles()
rust/sessions/tests/prov175_profile_numbers_are_validated.rs:58:20:load_local_server_profiles()
== resolve_turn_routing($$$A)
```

Extra (grep, the astgrep patterns above miss path-qualified calls):
- rust/sessions/src/model_resolution.rs:189 calls crate::profile_model_limits::apply_profile_model, the only path that selects a profile model (session create, resume and mid-session set_model all go through apply_model_selection).
- rust/agent-loop/src/agent_loop.rs:126 calls routing::resolve_turn_routing once per turn; it builds the turn's ThinkingTarget.

Conclusion: a profile model is selected in exactly one place, sessions profile_model_limits::apply_profile_model, which already reads the profile and its customModels entry. That is where the declaration is resolved into the model's ReasoningCapability and stored on the ProviderManager (next to the profile name that set_model_direct_with_profile records). ThinkingTarget::for_provider_manager, called once per turn from agent_loop/routing.rs, reads it; today it gives every profile model `Ok(None)` (unchecked). effort_wire's LocalOpenAi arm is the fixed thinking/enable_thinking form that the engine wire table replaces. ThinkingTarget::new is used by tests only (MODEL-010, BUG-440/441/442/455, MODEL-013); BUG-440's local-server scenario and MODEL-010's local scenario build a profile target with no capability and must move to a declared one. set_model_direct_with_profile is called from many tests with profile names, so the declaration is set through a separate setter rather than a new parameter.
