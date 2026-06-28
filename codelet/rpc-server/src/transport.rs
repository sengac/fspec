//! Internal Stream+Sink adapter that bridges the WebSocket pump to tarpc's
//! transport requirements.
//!
//! Each direction uses an unbounded mpsc channel of bincode-encoded
//! tarpc-protocol-message bytes; this adapter applies bincode codec on
//! either end so tarpc sees a normal `Stream<Item=Request> + Sink<Response>`
//! pair. The actual WebSocket pump (which wraps each byte-buffer in
//! [`crate::Envelope::Rpc`] before sending and unwraps on receive) is one
//! layer above this adapter — see [`crate::pump::run_envelope_pump`].

use futures::{Sink, Stream};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_stream::wrappers::UnboundedReceiverStream;

/// Stream of incoming tarpc-protocol messages (already bincode-encoded
/// in the inner Envelope::Rpc body) and a sink for outgoing ones.
pub struct ChannelTransport<Item, SinkItem> {
    incoming: UnboundedReceiverStream<Vec<u8>>,
    outgoing: UnboundedSender<Vec<u8>>,
    _item: std::marker::PhantomData<fn() -> Item>,
    _sink_item: std::marker::PhantomData<fn(SinkItem)>,
}

impl<Item, SinkItem> ChannelTransport<Item, SinkItem> {
    pub fn new(rx: UnboundedReceiver<Vec<u8>>, tx: UnboundedSender<Vec<u8>>) -> Self {
        Self {
            incoming: UnboundedReceiverStream::new(rx),
            outgoing: tx,
            _item: std::marker::PhantomData,
            _sink_item: std::marker::PhantomData,
        }
    }
}

impl<Item, SinkItem> Stream for ChannelTransport<Item, SinkItem>
where
    Item: serde::de::DeserializeOwned + Unpin,
{
    type Item = std::io::Result<Item>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.incoming).poll_next(cx) {
            Poll::Ready(Some(bytes)) => match bincode::deserialize::<Item>(&bytes) {
                Ok(item) => Poll::Ready(Some(Ok(item))),
                Err(e) => Poll::Ready(Some(Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    e,
                )))),
            },
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<Item, SinkItem> Sink<SinkItem> for ChannelTransport<Item, SinkItem>
where
    SinkItem: serde::Serialize + Unpin,
{
    type Error = std::io::Error;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: SinkItem) -> Result<(), Self::Error> {
        let bytes = bincode::serialize(&item)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        self.outgoing.send(bytes).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::BrokenPipe, "outgoing channel closed")
        })?;
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}
