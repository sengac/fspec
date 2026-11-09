macro_rules! compose_websocket_finish {
    ($($collected:tt)*) => {
#[async_trait]
impl FspecBackend for WebSocketFspecBackend {
    $($collected)*
}
    };
}
