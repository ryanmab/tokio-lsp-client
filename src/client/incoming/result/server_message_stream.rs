use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::mpsc::Receiver;

use crate::client::result::ServerMessage;

#[doc(hidden)]
type Inner = Receiver<super::ServerMessage>;

/// A stream of [`crate::client::ServerMessage`] sent from the server to the client.
///
/// This stream allows for out-of-band messages to be sent from the server - in order to notify the
/// client of a particular event, or request the client to perform a specific action.
///
/// The exact number of server messages to expect, as well as the contents and order of each message
/// cannot be known upfront, and vary between language servers.
///
/// It is not required that the client implement behaviours which react to specific messages, however
/// for the best experience, it is recommended that the stream be handled appropriately.
#[derive(Debug)]
pub struct ServerMessageStream {
    inner: Inner,
}

impl ServerMessageStream {
    #[must_use]
    pub(crate) const fn new(recv: Inner) -> Self {
        Self { inner: recv }
    }

    /// Get the inner receiving channel from the [`ServerMessageStream`].
    #[must_use]
    pub fn into_inner(self) -> Inner {
        self.inner
    }

    /// Close the stream early.
    pub fn close(&mut self) {
        self.inner.close();
    }
}

impl tokio_stream::Stream for ServerMessageStream {
    type Item = ServerMessage;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.poll_recv(cx)
    }
}

impl AsRef<Inner> for ServerMessageStream {
    fn as_ref(&self) -> &Inner {
        &self.inner
    }
}

impl AsMut<Inner> for ServerMessageStream {
    fn as_mut(&mut self) -> &mut Inner {
        &mut self.inner
    }
}

impl From<Inner> for ServerMessageStream {
    fn from(recv: Inner) -> Self {
        Self::new(recv)
    }
}
