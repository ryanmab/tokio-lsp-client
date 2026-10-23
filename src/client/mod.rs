use crate::client::connection::Connection;
use crate::client::incoming::{IncomingError, IncomingResult, Subscriber, Subscribers};
use crate::client::{self};
use crate::types;
use crate::types::{
    Cancel, CancelParams, Exit, Initialize, InitializeParams, InitializeResult, Initialized,
    InitializedParams, NumberOrString, Shutdown,
};
use std::process::Stdio;
use std::string::ToString;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{self};
use tokio::task::JoinHandle;
use tokio_stream::StreamExt;
use utils::cancellation::CancellationGuard;
use uuid::Uuid;

mod connection;
mod constant;
mod incoming;
mod outgoing;
mod utils;

#[allow(missing_docs)]
pub mod error;
pub use incoming::result;

pub use error::*;
#[doc(no_inline)]
pub use result::*;

pub use outgoing::{OutgoingNotification, OutgoingRequest};

#[derive(Debug)]
struct Listener(JoinHandle<()>);

impl Drop for Listener {
    fn drop(&mut self) {
        self.0.abort();
        log::debug!("Aborted listener thread as the client has been dropped");
    }
}

/// A connection to a Language Server Protocol (LSP) server.
///
/// Once connected, bidirectional communication between the client and server allows
/// for sending requests and notifications, and receiving responses and notifications.
///
/// Cloning a [`Client`] instance will create a new handle to the same underlying connection, allowing
/// for multiple threads to send requests and notifications concurrently, negotiated through [`tokio::sync::Mutex`]
/// locks and channels.
///
/// Protocol version v3.18.0 is fully supported.
///
/// <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/>
///
/// # Example
///
/// ```rust
/// use tokio_lsp_client::Client;
/// use tokio_lsp_client::types::{InitializeParams, InitializeResult};
/// use tokio::process::Command;
/// # use std::result::Result;
/// # use std::error::Error;
///
/// # #[tokio::main]
/// # pub async fn main() -> Result<(), Box<dyn Error>> {
/// let (mut client, _) = Client::from_command(&mut Command::new("rust-analyzer"))?;
///
/// let InitializeResult { server_info, .. } = client
///     .initialize(InitializeParams::default())
///     .await?;
///
/// client.initialized().await?;
///
/// let version = server_info
///     .map(|info| info.version.unwrap_or_else(|| "Unknown".into()))
///     .unwrap_or_else(|| "Unknown".into());
///
/// println!("Version: {version}");
///
/// client.shutdown().await?;
/// client.exit().await.map_err(|(_, err)| err)?;
///
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Client {
    /// Connection to the LSP server.
    connection: Arc<Connection>,

    /// Threads currently awaiting responses from the language server.
    subscribers: Subscribers,

    /// Handle to the task listening for incoming messages
    /// from the language server.
    #[allow(dead_code)]
    listener: Arc<Listener>,
}

impl Client {
    /// Create a new [`Client`] and connect via Stdio to a local process ([`tokio::process::Command`]).
    ///
    /// Returns a tuple containing the [`Client`] and a [`client::ServerMessageStream`] of messages
    /// from the server.
    ///
    /// [`client::ServerMessageStream`] allows for the language server to deliver [`client::ServerMessage`]s to the client outside of
    /// the traditional request-response lifecycle that is typical of client-sourced messages.
    ///
    /// <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#implementationConsiderations>
    ///
    /// # Example
    ///
    /// ```rust
    /// use tokio::process::Command;
    /// use tokio_lsp_client::Client;
    /// use tokio_lsp_client::types::{InitializeParams, InitializeResult, WorkspaceSymbolParams, WorkspaceSymbolRequest};
    /// use tokio_stream::StreamExt;
    /// # use std::result::Result;
    /// # use std::error::Error;
    ///
    /// # #[tokio::main]
    /// # pub async fn main() -> Result<(), Box<dyn Error>> {
    /// let (mut client, mut server_message_stream) =
    ///     Client::from_command(&mut Command::new("rust-analyzer"))?;
    ///
    /// tokio::spawn(async move {
    ///     while let Some(message) = server_message_stream.next().await {
    ///         // ...
    ///     }
    /// });
    ///
    /// let _ = client
    ///     .initialize(InitializeParams::default())
    ///     .await?;
    ///
    /// client.initialized().await?;
    ///
    /// // ...
    ///
    /// client.shutdown().await?;
    /// client.exit().await.map_err(|(_, err)| err)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    ///
    /// # Errors
    ///
    /// Returns an error if the process could not be spawned, or if the Stdio streams could not be accessed.
    pub fn from_command(
        command: &mut Command,
    ) -> ConnectionResult<(Self, result::ServerMessageStream)> {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            // Kill the process when the client (which owns the [`Connection`] and
            // process) is dropped, the accompanying listener task will be aborted
            // when the last client is dropped
            .kill_on_drop(true)
            .spawn()
            .map_err(client::ConnectionError::CouldNotSpawn)?;

        let stdin = child
            .stdin
            .take()
            .map(|stdin| Arc::new(Mutex::new(stdin)))
            .ok_or(client::ConnectionError::InvalidConnection)?;
        let stdout = child
            .stdout
            .take()
            .map(|stdout| Arc::new(Mutex::new(stdout)))
            .ok_or(client::ConnectionError::InvalidConnection)?;
        let stderr = child
            .stderr
            .take()
            .map(|stderr| Arc::new(Mutex::new(stderr)));

        let connection = Arc::new(Connection::StdIo {
            process: child,
            stdin,
            stdout,
            stderr,
        });

        let subscribers: Subscribers = Arc::default();

        let (tx, rx) = mpsc::channel(constant::SERVER_MESSAGE_CHANNEL_CAPACITY);

        let listener = Listener({
            let subscribers = Arc::clone(&subscribers);
            let connection = Arc::clone(&connection);

            tokio::spawn(async move {
                incoming::listen_for_messages(
                    Arc::clone(&connection.output()),
                    connection.error().as_ref().map(Arc::clone),
                    subscribers,
                    tx,
                )
                .await;
            })
        });

        Ok((
            Self {
                connection,
                subscribers,
                listener: Arc::new(listener),
            },
            result::ServerMessageStream::new(rx),
        ))
    }

    /// Send an `initialize` request to the LSP server.
    ///
    /// The request should always be the first one sent from the client to the server.
    ///
    /// Until the server has responded to the request with an [`crate::types::InitializeResult`], the
    /// client _must not_ send any additional requests or notifications to the server.
    ///
    /// In addition, the server _is not_ allowed to send any requests or notifications
    /// to the client until it has responded with an [`crate::types::InitializeResult`], with
    /// the exception that during the initialize request the server is allowed to send
    /// [`client::ServerMessage`]s containing:
    ///
    /// 1. [`client::result::NotificationParameters::ShowMessage`],
    /// 2. [`client::result::NotificationParameters::LogMessage`],
    /// 3. [`client::result::NotificationParameters::TelemetryEvent`]
    /// 4. [`client::result::RequestParameters::ShowMessageRequest`].
    ///
    /// In case the client sets up a progress token in the initialize parameters (e.g. [`crate::types::InitializeParams::work_done_progress_params`]) the
    /// server is also allowed to use that token (and only that token) using the [`client::result::NotificationParameters::Progress`] notification sent
    /// from the server to the client.
    ///
    /// The initialize request may only be sent once.
    ///
    /// <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#initialize>
    ///
    /// # Errors
    ///
    /// Returns an error if the request could not be sent ([`client::WriteError`]), or if the server did not respond
    /// with a valid [`crate::types::InitializeResult`] ([`client::ReadError`]).
    pub async fn initialize(
        &mut self,
        params: InitializeParams,
    ) -> ProtocolResult<InitializeResult> {
        let result = self
            .send_request::<Initialize>(params)
            .await?
            .next()
            .await
            .ok_or_else(|| {
                client::ProtocolError::Read(client::ReadError::CouldNotRead(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Initialize did not return a result".to_string(),
                )))
            })?;

        result.map_err(|err| client::ProtocolError::Read(client::ReadError::InvalidResponse(err)))
    }

    /// Send the `initialized` notification to the LSP server.
    ///
    /// The notification is sent, at most once, after the client receives the result of the initialize request but
    /// before the client begins to send any other request or notification to the server.
    ///
    /// The server can use the notification, for example, to dynamically register capabilities.
    ///
    /// <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#initialized>
    ///
    /// # Errors
    ///
    /// Returns an error if the notification could not be sent ([`client::WriteError`]).
    pub async fn initialized(&mut self) -> ProtocolResult<(), client::WriteError> {
        self.send_notification::<Initialized>(InitializedParams {})
            .await?;

        ProtocolResult::Ok(())
    }

    /// Send a request to the LSP server and wait for a response.
    ///
    /// # Streamed Results
    ///
    /// The server may optionally stream back partial results to the client, which can be accessed through the
    /// [`PartialResult`] stream. However, this is predicated on the client providing a partial result token
    /// as a parameter on eligible requests.
    ///
    /// If the request is not streamable, or the server decides not to stream the results, the [`PartialResult`]
    /// stream will only contain a single item, which is the final result of the request.
    ///
    /// <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#languageFeatures>
    /// <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#partialResultParams>
    ///
    /// # Cancellation
    ///
    /// Dropping an in-flight request will automatically cause a cancellation notification to be sent to the server
    /// ([`crate::types::Cancel`]), however support for actual cancellation depends on the language
    /// server.
    ///
    /// <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#cancelRequest>
    ///
    /// # Errors
    ///
    /// Returns an error if the request could not be sent ([`client::WriteError`]), or if the server did not respond with a valid result ([`client::ReadError`]).
    pub async fn send_request<OR>(
        &self,
        params: OR::Params,
    ) -> ProtocolResult<result::PartialResult<OR>>
    where
        OR: OutgoingRequest,
    {
        let request_id = NumberOrString::String(Uuid::new_v4().into());
        let cancellation_id = request_id.clone();
        let connection = Arc::clone(&self.connection);

        let request_tokens = {
            outgoing::send_request::<OR, _>(
                &request_id,
                &params,
                &mut *self.connection.input().lock().await,
            )
            .await
        }
        .map_err(client::ProtocolError::Write)?;

        CancellationGuard::with_guard(
            async || {
                let (tx, rx) = mpsc::channel::<ProtocolResult<IncomingResult, IncomingError>>(constant::RESULT_CHANNEL_BUFFER_CAPACITY);

                let mut guard = self.subscribers.lock().await;

                // If the request provided a token for the language server to stream back partial
                // results using, also add the channel as a subscriber
                if let Some(partial_result_token) = request_tokens.get_partial_result_token() {
                    guard.insert(
                        partial_result_token.clone(),
                        Subscriber::new(tx.clone(), [
                            Some(&request_id),
                            request_tokens.get_work_done_token()
                        ])
                    );
                }

                // Add the channel as a subscriber for responses to the main request
                guard.insert(request_id, Subscriber::new(tx.clone(), request_tokens.iter()));

                drop(guard);

                Ok(PartialResult::<OR>::new(rx))
            },
            || {
                Box::pin(async move {
                    let input = connection.input();
                    let mut input = input.lock().await;

                    if let Err(err) = outgoing::send_notification::<Cancel, _>(
                        &CancelParams { id: cancellation_id.clone() },
                        &mut *input,
                    )
                    .await
                    {
                        log::error!(
                            "Unable to send cancellation notification for request {cancellation_id:?}: {err}",
                        );
                    }
                })
            },
        )
        .await
    }

    /// Send a result to the LSP server in response to a previous [`client::ServerMessage`].
    ///
    /// # Errors
    ///
    /// Returns an error if the response could not be sent ([`client::WriteError`]).
    pub async fn send_result<OR>(
        &self,
        id: NumberOrString,
        result: OR::Result,
    ) -> ProtocolResult<()>
    where
        OR: OutgoingRequest,
    {
        let input = self.connection.input();

        outgoing::send_result::<OR, _>(id, &result, &mut *input.lock().await)
            .await
            .map_err(client::ProtocolError::Write)
    }

    /// Send an error to the LSP server in response to a previous [`client::ServerMessage`].
    ///
    /// # Errors
    ///
    /// Returns an error if the error response could not be sent ([`client::WriteError`]).
    pub async fn send_error(&self, id: NumberOrString, error: types::Error) -> ProtocolResult<()> {
        let input = self.connection.input();

        outgoing::send_error(id, &error, &mut *input.lock().await)
            .await
            .map_err(client::ProtocolError::Write)
    }

    /// Send a notification to the LSP server.
    ///
    /// # Errors
    ///
    /// Returns an error if the notification could not be sent ([`client::WriteError`]).
    pub async fn send_notification<ON>(
        &self,
        params: ON::Params,
    ) -> ProtocolResult<(), client::WriteError>
    where
        ON: OutgoingNotification,
    {
        let input = self.connection.input();

        outgoing::send_notification::<ON, _>(&params, &mut *input.lock().await).await
    }

    /// Send a `shutdown` request to the LSP server.
    ///
    /// The shutdown request is sent from the client to the server to ask the server to shut down, but to not
    /// exit (terminate) the process.
    ///
    /// Once the server is shut down, the client can optionally start the server again by sending an `initialize`
    /// request.
    ///
    /// <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#shutdown>
    ///
    /// ## Errors
    ///
    /// Returns an error if the request could not be sent ([`client::WriteError`]), or if the server did not respond with a valid result
    /// ([`client::ReadError`]).
    pub async fn shutdown(&mut self) -> ProtocolResult<()> {
        let outcome = self
            .send_request::<Shutdown>(())
            .await?
            .next()
            .await
            .ok_or_else(|| {
                client::ProtocolError::Read(client::ReadError::CouldNotRead(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Shutdown did not return a response".to_string(),
                )))
            })?;

        outcome.map_err(|err| client::ProtocolError::Read(client::ReadError::InvalidResponse(err)))
    }

    /// Send an `exit` notification to the LSP server.
    ///
    /// The exit notification is sent from the client to the server to ask the server to exit (terminate) the process.
    ///
    /// <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#exit>
    ///
    /// # Errors
    ///
    /// Returns an error if the language server did not successfully terminate ([`client::WriteError`]). The current client
    /// is also returned again, to allow for the exit to be retried.
    pub async fn exit(self) -> ProtocolResult<(), (Self, client::WriteError)> {
        self.send_notification::<Exit>(())
            .await
            .map_err(|err| (self, err))?;

        ProtocolResult::Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc;

    use super::types::{InitializeParams, WorkspaceSymbolParams, WorkspaceSymbolRequest};
    use super::*;
    use tokio::process::Command;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[test_log::test]
    pub async fn test_concurrent_cloned_clients() {
        let (mut client, mut server_message_stream) =
            Client::from_command(&mut Command::new("rust-analyzer"))
                .expect("Should never fail to create client");

        assert!(client.initialize(InitializeParams::default()).await.is_ok());
        client
            .initialized()
            .await
            .expect("Should never fail to send initialized notification");

        // Wait for the first server message to come back (usually a diagnostic refresh)
        // to ensure we're not sending requests before the server is ready
        server_message_stream.next().await;

        let (tx, mut rx) = mpsc::channel::<()>(2);

        let client_one = client.clone();
        let tx_one = tx.clone();

        let client_two = client.clone();
        let tx_two = tx.clone();

        let handle_one = tokio::spawn(async move {
            let _ = client_one
                .send_request::<WorkspaceSymbolRequest>(WorkspaceSymbolParams {
                    query: "listen_for_messages".to_string(),
                    ..Default::default()
                })
                .await
                .expect("Should never fail to request workspace symbols")
                .collect::<Vec<_>>()
                .await;

            tx_one
                .send(())
                .await
                .expect("Should never fail to send completion message");
        });

        let handle_two = tokio::spawn(async move {
            let _ = client_two
                .send_request::<WorkspaceSymbolRequest>(WorkspaceSymbolParams {
                    query: "Client".to_string(),
                    ..Default::default()
                })
                .await
                .expect("Should never fail to request workspace symbols")
                .collect::<Vec<_>>()
                .await;

            tx_two
                .send(())
                .await
                .expect("Should never fail to send completion message");
        });

        let (result_1, result_2) = tokio::join!(handle_one, handle_two);

        assert!(result_1.is_ok());
        assert!(result_2.is_ok());

        let mut buf = Vec::with_capacity(2);
        assert_eq!(rx.recv_many(&mut buf, 2).await, 2);
        assert_eq!(buf, &[(), ()]);

        assert!(client.shutdown().await.is_ok());
        assert!(client.exit().await.is_ok());
    }
}
