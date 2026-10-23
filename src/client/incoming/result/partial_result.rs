use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio_stream::Stream;

use crate::client::incoming::{IncomingError, IncomingResult};
use crate::client::{self, OutgoingRequest};

#[doc(hidden)]
type Receiver = tokio::sync::mpsc::Receiver<Result<IncomingResult, IncomingError>>;

/// A stream of results, in response to a [`crate::client::OutgoingRequest`], received from a language server.
///
/// This allows for a server to deliver faster feedback to an [`crate::client::OutgoingRequest`] by streaming results back
/// in batches, rather than buffering everything for a single result.
///
/// The exact number of results in a batch, the exact number of batches to expect, and even whether results will be streamed
/// at all cannot be known to the client upfront. However, the stream will be closed, that is [`Option::None`] returned, when
/// all results have been returned.
///
/// Partial result streaming is opt-in from both the client and language server, and may not be implemented by every language server. In
/// order to opt-in as a client, requests must include a partial result token in the parameters, and the server will then choose whether
/// to make use of this.
///
/// If this token is not provided by the client, a single batch containing all the results will be returned, and the stream will be
/// immediately closed.
///
/// <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#partialResults>
#[derive(Debug)]
pub struct PartialResult<R>
where
    R: OutgoingRequest,
{
    inner: Receiver,
    _result: PhantomData<R::Result>,
}

impl<R> PartialResult<R>
where
    R: OutgoingRequest,
{
    #[must_use]
    pub(crate) const fn new(recv: Receiver) -> Self {
        Self {
            inner: recv,
            _result: PhantomData,
        }
    }

    /// Get the inner receiving channel from the [`PartialResult`] stream.
    #[must_use]
    pub fn into_inner(self) -> Receiver {
        self.inner
    }

    /// Close the stream early.
    pub fn close(&mut self) {
        self.inner.close();
    }
}

impl<R> Stream for PartialResult<R>
where
    R: OutgoingRequest,
{
    type Item = client::LspResult<R::Result>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.poll_recv(cx).map(|item| {
            item.map(|incoming_result| match incoming_result {
                Ok(incoming_result) => client::LspResult::Ok(
                    serde_json::from_value::<R::Result>(incoming_result.result).map_err(
                        // TODO(RM): Consider if there is a better representation for errors like this which
                        // sit somewhere between a protocol error, and an LSP error. In this case the LSP server
                        // has given us back _something_ that it expects to be a result, but we've been unable
                        // to parse it. So, it's not quite an LSP error (since the server didn't return it!), its
                        // closer to a protocol error, but to return that we'd have to wrap the return type in
                        // two levels of Result types, which is inconventient from an API point
                        // of view.
                        Into::<crate::types::Error>::into,
                    )?,
                ),
                Err(incoming_error) => client::LspResult::Err(incoming_error.error),
            })
        })
    }
}

impl<R> AsRef<Receiver> for PartialResult<R>
where
    R: OutgoingRequest,
{
    fn as_ref(&self) -> &Receiver {
        &self.inner
    }
}

impl<R> AsMut<Receiver> for PartialResult<R>
where
    R: OutgoingRequest,
{
    fn as_mut(&mut self) -> &mut Receiver {
        &mut self.inner
    }
}

impl<R> From<Receiver> for PartialResult<R>
where
    R: OutgoingRequest,
{
    fn from(recv: Receiver) -> Self {
        Self::new(recv)
    }
}

impl<R: OutgoingRequest> Unpin for PartialResult<R> {}
