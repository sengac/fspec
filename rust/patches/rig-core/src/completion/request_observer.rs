use super::{CompletionError, Usage};
use futures::{Stream, StreamExt};
use std::sync::Arc;

pub type RequestUsageObserver = Arc<dyn Fn(&str, &Usage) -> Result<(), String> + Send + Sync>;

tokio::task_local! {
    static REQUEST_USAGE_OBSERVER: Option<RequestUsageObserver>;
}

pub fn report_request_usage(request_id: &str, usage: &Usage) -> Result<(), CompletionError> {
    REQUEST_USAGE_OBSERVER
        .try_with(|observer| match observer {
            Some(observer) => observer(request_id, usage).map_err(CompletionError::ResponseError),
            None => Ok(()),
        })
        .unwrap_or(Ok(()))
}

pub fn observe_request_usage<Source: Stream>(
    source: Source,
    observer: Option<RequestUsageObserver>,
) -> impl Stream<Item = Source::Item> {
    async_stream::stream! {
        futures::pin_mut!(source);
        while let Some(event) = REQUEST_USAGE_OBSERVER.scope(observer.clone(), source.next()).await {
            yield event;
        }
    }
}
