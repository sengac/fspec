//! Request-only connection: no unsolicited workspace/session/log snapshots.
use crate::transport::ChannelTransport;
use crate::ServerStats;
use crate::{
    envelope::Envelope,
    pump::{run_envelope_pump, ServerInbound},
};
use codelet_rpc::{FspecService, FspecServiceImpl, SharedFspecService};
use futures::StreamExt;
use std::sync::Arc;
use tarpc::server::{BaseChannel, Channel};
use tokio_tungstenite::WebSocketStream;

pub(crate) async fn run<S>(
    ws: WebSocketStream<S>,
    service: Arc<SharedFspecService>,
    stats: ServerStats,
) -> anyhow::Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let (rpc_tx, rpc_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
    let (out_tx, out_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
    let (_push_tx, push_rx) = tokio::sync::mpsc::unbounded_channel::<Envelope>();
    let transport = ChannelTransport::new(rpc_rx, out_tx);
    let server = BaseChannel::with_defaults(transport);
    let serving = server
        .execute(FspecServiceImpl::for_authenticated_connection(service).serve())
        .for_each(|response| async move {
            tokio::spawn(response);
        });
    let (sink, stream) = ws.split();
    let pumping = run_envelope_pump(
        sink,
        stream,
        rpc_tx,
        out_rx,
        push_rx,
        ServerInbound {
            stats: stats.clone(),
        },
        Some(Arc::clone(&stats.shutdown_signal)),
        Some(Arc::clone(&stats.shutdown_flag)),
    );
    tokio::select! { _ = serving => Ok(()), result = pumping => result }
}
