pub fn is_content<Response>(
    event: &Result<
        rig::streaming::StreamedAssistantContent<Response>,
        rig::completion::CompletionError,
    >,
) -> futures::future::Ready<bool> {
    futures::future::ready(!matches!(
        event,
        Ok(
            rig::streaming::StreamedAssistantContent::ResponseAttemptStarted
                | rig::streaming::StreamedAssistantContent::ResponseAttemptAborted
        )
    ))
}
