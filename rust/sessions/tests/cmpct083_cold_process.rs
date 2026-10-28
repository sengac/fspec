//! Feature: spec/features/cold-resume-compaction-acceptance-gate.feature

#[path = "cmpct083/application.rs"]
pub mod application;
#[path = "cmpct083/codex.rs"]
pub mod codex;
#[path = "cmpct083/custom.rs"]
pub mod custom;
#[path = "cmpct083/fixture.rs"]
pub mod fixture;
#[path = "cmpct083/lifecycle.rs"]
pub mod lifecycle;
#[path = "cmpct083/originals.rs"]
pub mod originals;
#[path = "cmpct083/recovery.rs"]
pub mod recovery;
#[path = "cmpct083/restoration.rs"]
pub mod restoration;
#[path = "cmpct083/runtime.rs"]
pub mod runtime;
#[path = "cmpct083/suffix.rs"]
pub mod suffix;
#[path = "cmpct083/transport.rs"]
pub mod transport;
#[path = "cmpct083/zai.rs"]
pub mod zai;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_repeated_committed_dag_cycles_anthropic() -> fixture::TestResult {
    // @step Given a live <provider> session with immutable originals and post-snapshot tool work
    // @step When two distinct DAG candidates are submitted through the live runtime and durably applied using the selected physical request preparer
    // @step And the selected session sends a physical provider request
    // @step And a fresh process restores the session and sends another physical provider request
    // @step Then both requests carry the intended committed DAG and complete ordered tool suffix
    // @step And no covered original is silently replayed in either physical request
    // @step And each candidate was submitted by the live stream and applied through the runtime commit owner
    // @step And the physical adapter is the selected provider rather than a copied generic request
    // @step And original payload retrieval returns the retained deep sentinel without another commit
    // @step And context events and durable generation agree
    // @step And genuine provider checkpoints in the captured prefix and retained suffix remain immutable but are not conversational content
    // @step And the committed checkpoint generation remains authoritative before the next live and cold physical requests
    // @step And repeated restore after a completed same-generation request preserves checkpoint and ordered suffix without duplicate billing
    // @step And an additional immediate post-commit fresh-process probe does not replace the live-request-before-resume acceptance sequence
    // @step And Z.AI rows resolve their actual selected model, credential endpoint and GLM request options rather than borrowing OpenAI coverage
    lifecycle::repeated(
        "anthropic",
        "scenario_repeated_committed_dag_cycles_anthropic",
    )
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_repeated_committed_dag_cycles_zai() -> fixture::TestResult {
    // @step Given a live <provider> session with immutable originals and post-snapshot tool work
    // @step When two distinct DAG candidates are submitted through the live runtime and durably applied using the selected physical request preparer
    // @step And the selected session sends a physical provider request
    // @step And a fresh process restores the session and sends another physical provider request
    // @step Then both requests carry the intended committed DAG and complete ordered tool suffix
    // @step And no covered original is silently replayed in either physical request
    // @step And each candidate was submitted by the live stream and applied through the runtime commit owner
    // @step And the physical adapter is the selected provider rather than a copied generic request
    // @step And original payload retrieval returns the retained deep sentinel without another commit
    // @step And context events and durable generation agree
    // @step And genuine provider checkpoints in the captured prefix and retained suffix remain immutable but are not conversational content
    // @step And the committed checkpoint generation remains authoritative before the next live and cold physical requests
    // @step And repeated restore after a completed same-generation request preserves checkpoint and ordered suffix without duplicate billing
    // @step And an additional immediate post-commit fresh-process probe does not replace the live-request-before-resume acceptance sequence
    // @step And Z.AI rows resolve their actual selected model, credential endpoint and GLM request options rather than borrowing OpenAI coverage
    lifecycle::repeated("zai", "scenario_repeated_committed_dag_cycles_zai").await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_repeated_committed_dag_cycles_zai_plan() -> fixture::TestResult {
    // @step Given a live <provider> session with immutable originals and post-snapshot tool work
    // @step When two distinct DAG candidates are submitted through the live runtime and durably applied using the selected physical request preparer
    // @step And the selected session sends a physical provider request
    // @step And a fresh process restores the session and sends another physical provider request
    // @step Then both requests carry the intended committed DAG and complete ordered tool suffix
    // @step And no covered original is silently replayed in either physical request
    // @step And each candidate was submitted by the live stream and applied through the runtime commit owner
    // @step And the physical adapter is the selected provider rather than a copied generic request
    // @step And original payload retrieval returns the retained deep sentinel without another commit
    // @step And context events and durable generation agree
    // @step And genuine provider checkpoints in the captured prefix and retained suffix remain immutable but are not conversational content
    // @step And the committed checkpoint generation remains authoritative before the next live and cold physical requests
    // @step And repeated restore after a completed same-generation request preserves checkpoint and ordered suffix without duplicate billing
    // @step And an additional immediate post-commit fresh-process probe does not replace the live-request-before-resume acceptance sequence
    // @step And Z.AI rows resolve their actual selected model, credential endpoint and GLM request options rather than borrowing OpenAI coverage
    lifecycle::repeated(
        "zai-plan",
        "scenario_repeated_committed_dag_cycles_zai_plan",
    )
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_repeated_committed_dag_cycles_openai() -> fixture::TestResult {
    // @step Given a live <provider> session with immutable originals and post-snapshot tool work
    // @step When two distinct DAG candidates are submitted through the live runtime and durably applied using the selected physical request preparer
    // @step And the selected session sends a physical provider request
    // @step And a fresh process restores the session and sends another physical provider request
    // @step Then both requests carry the intended committed DAG and complete ordered tool suffix
    // @step And no covered original is silently replayed in either physical request
    // @step And each candidate was submitted by the live stream and applied through the runtime commit owner
    // @step And the physical adapter is the selected provider rather than a copied generic request
    // @step And original payload retrieval returns the retained deep sentinel without another commit
    // @step And context events and durable generation agree
    // @step And genuine provider checkpoints in the captured prefix and retained suffix remain immutable but are not conversational content
    // @step And the committed checkpoint generation remains authoritative before the next live and cold physical requests
    // @step And repeated restore after a completed same-generation request preserves checkpoint and ordered suffix without duplicate billing
    // @step And an additional immediate post-commit fresh-process probe does not replace the live-request-before-resume acceptance sequence
    // @step And Z.AI rows resolve their actual selected model, credential endpoint and GLM request options rather than borrowing OpenAI coverage
    lifecycle::repeated("openai", "scenario_repeated_committed_dag_cycles_openai").await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_repeated_committed_dag_cycles_gemini() -> fixture::TestResult {
    // @step Given a live <provider> session with immutable originals and post-snapshot tool work
    // @step When two distinct DAG candidates are submitted through the live runtime and durably applied using the selected physical request preparer
    // @step And the selected session sends a physical provider request
    // @step And a fresh process restores the session and sends another physical provider request
    // @step Then both requests carry the intended committed DAG and complete ordered tool suffix
    // @step And no covered original is silently replayed in either physical request
    // @step And each candidate was submitted by the live stream and applied through the runtime commit owner
    // @step And the physical adapter is the selected provider rather than a copied generic request
    // @step And original payload retrieval returns the retained deep sentinel without another commit
    // @step And context events and durable generation agree
    // @step And genuine provider checkpoints in the captured prefix and retained suffix remain immutable but are not conversational content
    // @step And the committed checkpoint generation remains authoritative before the next live and cold physical requests
    // @step And repeated restore after a completed same-generation request preserves checkpoint and ordered suffix without duplicate billing
    // @step And an additional immediate post-commit fresh-process probe does not replace the live-request-before-resume acceptance sequence
    // @step And Z.AI rows resolve their actual selected model, credential endpoint and GLM request options rather than borrowing OpenAI coverage
    lifecycle::repeated("gemini", "scenario_repeated_committed_dag_cycles_gemini").await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_repeated_committed_dag_cycles_copilot() -> fixture::TestResult {
    // @step Given a live <provider> session with immutable originals and post-snapshot tool work
    // @step When two distinct DAG candidates are submitted through the live runtime and durably applied using the selected physical request preparer
    // @step And the selected session sends a physical provider request
    // @step And a fresh process restores the session and sends another physical provider request
    // @step Then both requests carry the intended committed DAG and complete ordered tool suffix
    // @step And no covered original is silently replayed in either physical request
    // @step And each candidate was submitted by the live stream and applied through the runtime commit owner
    // @step And the physical adapter is the selected provider rather than a copied generic request
    // @step And original payload retrieval returns the retained deep sentinel without another commit
    // @step And context events and durable generation agree
    // @step And genuine provider checkpoints in the captured prefix and retained suffix remain immutable but are not conversational content
    // @step And the committed checkpoint generation remains authoritative before the next live and cold physical requests
    // @step And repeated restore after a completed same-generation request preserves checkpoint and ordered suffix without duplicate billing
    // @step And an additional immediate post-commit fresh-process probe does not replace the live-request-before-resume acceptance sequence
    // @step And Z.AI rows resolve their actual selected model, credential endpoint and GLM request options rather than borrowing OpenAI coverage
    lifecycle::repeated("copilot", "scenario_repeated_committed_dag_cycles_copilot").await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_repeated_committed_dag_cycles_codex() -> fixture::TestResult {
    // @step Given a live <provider> session with immutable originals and post-snapshot tool work
    // @step When two distinct DAG candidates are submitted through the live runtime and durably applied using the selected physical request preparer
    // @step And the selected session sends a physical provider request
    // @step And a fresh process restores the session and sends another physical provider request
    // @step Then both requests carry the intended committed DAG and complete ordered tool suffix
    // @step And no covered original is silently replayed in either physical request
    // @step And each candidate was submitted by the live stream and applied through the runtime commit owner
    // @step And the physical adapter is the selected provider rather than a copied generic request
    // @step And original payload retrieval returns the retained deep sentinel without another commit
    // @step And context events and durable generation agree
    // @step And genuine provider checkpoints in the captured prefix and retained suffix remain immutable but are not conversational content
    // @step And the committed checkpoint generation remains authoritative before the next live and cold physical requests
    // @step And repeated restore after a completed same-generation request preserves checkpoint and ordered suffix without duplicate billing
    // @step And an additional immediate post-commit fresh-process probe does not replace the live-request-before-resume acceptance sequence
    // @step And Z.AI rows resolve their actual selected model, credential endpoint and GLM request options rather than borrowing OpenAI coverage
    lifecycle::repeated("codex", "scenario_repeated_committed_dag_cycles_codex").await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_repeated_committed_dag_cycles_custom() -> fixture::TestResult {
    // @step Given a live <provider> session with immutable originals and post-snapshot tool work
    // @step When two distinct DAG candidates are submitted through the live runtime and durably applied using the selected physical request preparer
    // @step And the selected session sends a physical provider request
    // @step And a fresh process restores the session and sends another physical provider request
    // @step Then both requests carry the intended committed DAG and complete ordered tool suffix
    // @step And no covered original is silently replayed in either physical request
    // @step And each candidate was submitted by the live stream and applied through the runtime commit owner
    // @step And the physical adapter is the selected provider rather than a copied generic request
    // @step And original payload retrieval returns the retained deep sentinel without another commit
    // @step And context events and durable generation agree
    // @step And genuine provider checkpoints in the captured prefix and retained suffix remain immutable but are not conversational content
    // @step And the committed checkpoint generation remains authoritative before the next live and cold physical requests
    // @step And repeated restore after a completed same-generation request preserves checkpoint and ordered suffix without duplicate billing
    // @step And an additional immediate post-commit fresh-process probe does not replace the live-request-before-resume acceptance sequence
    // @step And Z.AI rows resolve their actual selected model, credential endpoint and GLM request options rather than borrowing OpenAI coverage
    lifecycle::repeated("custom", "scenario_repeated_committed_dag_cycles_custom").await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_repeated_committed_dag_cycles_stub() -> fixture::TestResult {
    // @step Given a live <provider> session with immutable originals and post-snapshot tool work
    // @step When two distinct DAG candidates are submitted through the live runtime and durably applied using the selected physical request preparer
    // @step And the selected session sends a physical provider request
    // @step And a fresh process restores the session and sends another physical provider request
    // @step Then both requests carry the intended committed DAG and complete ordered tool suffix
    // @step And no covered original is silently replayed in either physical request
    // @step And each candidate was submitted by the live stream and applied through the runtime commit owner
    // @step And the physical adapter is the selected provider rather than a copied generic request
    // @step And original payload retrieval returns the retained deep sentinel without another commit
    // @step And context events and durable generation agree
    // @step And genuine provider checkpoints in the captured prefix and retained suffix remain immutable but are not conversational content
    // @step And the committed checkpoint generation remains authoritative before the next live and cold physical requests
    // @step And repeated restore after a completed same-generation request preserves checkpoint and ordered suffix without duplicate billing
    // @step And an additional immediate post-commit fresh-process probe does not replace the live-request-before-resume acceptance sequence
    // @step And Z.AI rows resolve their actual selected model, credential endpoint and GLM request options rather than borrowing OpenAI coverage
    lifecycle::repeated("stub", "scenario_repeated_committed_dag_cycles_stub").await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_cancellation_before_promotion_preserves_the_original_generation_on_cold_resume(
) -> fixture::TestResult {
    // @step Given a committed session with an uncovered tool suffix
    // @step When candidate preparation is cancelled before durable promotion
    // @step And a fresh process restores the session
    // @step Then the original generation and complete suffix survive without a completion notice
    recovery::cancel(
        "scenario_cancellation_before_promotion_preserves_the_original_generation_on_cold_resume",
    )
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_notification_loss_after_promotion_reconciles_the_new_generation_without_another_commit(
) -> fixture::TestResult {
    // @step Given durable promotion succeeds but completion notification fails
    // @step When a fresh process restores and explicitly reconciles the session
    // @step Then the new generation and complete suffix survive without rollback or duplicate promotion
    // @step And ordinary inference makes no physical request before exact identity reconciliation
    // @step And the header refresh does not claim that the lost notification was delivered
    recovery::notification("scenario_notification_loss_after_promotion_reconciles_the_new_generation_without_another_commit").await
}
