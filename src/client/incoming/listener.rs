use std::sync::Arc;

use crate::client::incoming::{self, IncomingError, IncomingMessage, IncomingResult, Subscriber};
use crate::client::result::ServerMessage;
use crate::types::NumberOrString;
use tokio::io::AsyncRead;
use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::error::TrySendError;

use serde_json::Value::Array;
use serde_json::json;

use crate::client::Subscribers;
use crate::client::connection::{ErrorOut, Out};

/// A listener which orchestrates incoming messages from the output channel and error channels,
/// and delivers them to the appropriate destination.
///
/// This is either (1) the server message stream in case of an out-of-band notification
/// or request from the server, (2) any subscribed streams waiting for a response from a previous
/// message.
pub async fn listen_for_messages(
    output: Out<impl AsyncRead + Unpin + Send + Sync>,
    error: Option<ErrorOut<impl AsyncRead + Unpin + Send + Sync>>,
    subscribers: Subscribers,
    tx: Sender<ServerMessage>,
) {
    loop {
        let message = if let Some(error) = error.as_ref() {
            tokio::select! {
                message = incoming::get_message(Arc::clone(&output)) => message,
                () = incoming::handle_error(Arc::clone(error)) => continue
            }
        } else {
            incoming::get_message(Arc::clone(&output)).await
        };

        if let Err(error) = message {
            log::warn!(
                "An error occurred while reading a message from the language server: {error:?}"
            );
            continue;
        }

        let Ok(Some(message)) = message else {
            // No more messages to receive, we can do a safe shutdown
            log::trace!("Output closed, shutting down listener.");
            return;
        };

        match message {
            IncomingMessage::Result(result) => {
                broadcast_to_subscriber(
                    Arc::clone(&subscribers),
                    result.id.clone(),
                    Ok(result),
                    true,
                )
                .await;
            }
            IncomingMessage::Error(error) => {
                broadcast_to_subscriber(
                    Arc::clone(&subscribers),
                    error.id.clone(),
                    Err(error),
                    true,
                )
                .await;
            }
            IncomingMessage::Request {
                id,
                method,
                parameters,
            } => {
                let map = json!({
                    "id": &id,
                    "method": method,
                    "params": parameters
                });

                match serde_json::from_value::<ServerMessage>(map) {
                    Ok(message) => broadcast_to_caller(&tx, message),
                    Err(err) => log::warn!(
                        "Invalid request ({method}) received from language server: {err}. ID: {id:?}. Parameters: {parameters:?}"
                    ),
                }
            }
            IncomingMessage::Notification {
                method,
                token,
                parameters,
                value,
            } => {
                if method == "$/progress"
                    && let Some(token) = token
                    && let Some(Array(value)) = value
                    && subscribers.lock().await.contains_key(&token)
                {
                    // We've got a partial result (a progressive stream of chucks of a large result
                    // using generic notifications)
                    //
                    // As per the LSP spec, the last partial response will always be empty, to
                    // signify the end of the stream.
                    //
                    // https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#partialResults
                    let is_last_partial = value.is_empty();

                    broadcast_to_subscriber(
                        Arc::clone(&subscribers),
                        token.clone(),
                        Ok(IncomingResult {
                            id: token,
                            result: Array(value),
                        }),
                        is_last_partial,
                    )
                    .await;

                    break;
                }

                let map = json!({
                    "method": method,
                    "params": parameters
                });

                let server_message = match serde_json::from_value::<ServerMessage>(map) {
                    Ok(message) => message,
                    Err(err) => {
                        log::warn!(
                            "Invalid notification ({method}) received from language server: {err}. Parameters: {parameters:?}"
                        );
                        continue;
                    }
                };

                broadcast_to_caller(&tx, server_message);
            }
        }
    }
}

/// Broadcast an out-of-band message to the parent caller.
///
/// This is typically a server wishing to deliver notifications, or other telemetry
/// to the parent, but could be a request also.
///
/// Theres no obligation that the client handle these messages, and so it's possible
/// (and common) that the channel be closed.
pub fn broadcast_to_caller(tx: &Sender<ServerMessage>, message: ServerMessage) {
    match tx.try_send(message) {
        Ok(()) => {
            log::trace!("Informed parent of message from language server.");
        }
        Err(TrySendError::Closed(_)) => {
            // The channel for exchanging server messages with the parent is closed and
            // so there's nothing more we need to do.
            log::debug!("Did not broadcast message from server as the parent is not listening");
        }
        Err(err) => log::warn!(
            "Failed to send server's notification to caller: {:?}",
            err.to_string()
        ),
    }
}

/// Broadcast a received message back to its subscribing channel.
///
/// This is typically a result being delivered to an awaiting task the parent
/// caller initiated.
///
/// It's also possible that a subscriber expect _multiple_ messages back in the
/// case of a partial result. But, as documented in the spec a partial result
/// will be denoted as complete by an empty final message.
///
/// <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#partialResults>
pub async fn broadcast_to_subscriber(
    subscribers: Subscribers,
    id: NumberOrString,
    message: Result<IncomingResult, IncomingError>,
    is_final_message: bool,
) {
    let guard = subscribers.lock().await;

    if let Some(Subscriber { tx, .. }) = guard.get(&id) {
        match tx.try_send(message) {
            Ok(()) => log::trace!("Informed subscriber of message for request {id:?}"),
            Err(TrySendError::Closed(_)) => {
                // The channel in the map is closed and so there's nothing more
                // we need to do with this response.
                //
                // This usually happens when the original request's future was
                // canceled (dropped) (see [`CancellationToken`]) while awaiting
                // a response from the server. However, as is enforced in the LSP
                // specification, the server _MUST_ return some kind of response, even
                // for a canceled request.
                log::debug!(
                    "Did not broadcast message for request {id:?} as the subscriber is no longer awaiting broadcasts",
                );

                drop(guard);

                delete_subscriber_and_siblings_by_id(subscribers, &id).await;

                return;
            }
            Err(err) => log::warn!(
                "Failed to broadcast message for request {id:?}: {:?}",
                err.to_string()
            ),
        }
    }

    drop(guard);

    if is_final_message {
        delete_subscriber_and_siblings_by_id(subscribers, &id).await;
    }
}

/// Delete a subscriber and any of its siblings using the subscribers ID.
async fn delete_subscriber_and_siblings_by_id(subscribers: Subscribers, id: &NumberOrString) {
    let mut subscribers = subscribers.lock().await;

    let subscriber = subscribers.remove(id);

    log::debug!(
        "Removed subscriber attached to request {id:?} as the final message has been received",
    );

    // Remove any other subscribers which might have been initialised at the same time
    // as the subscriber we're removing
    if let Some(Subscriber { siblings, .. }) = subscriber {
        for sibling in siblings {
            log::trace!(
                "Removed sibling subscriber {sibling:?} as the final message has been received by request {id:?}"
            );
            subscribers.remove(&sibling);
        }
    }
}

#[cfg(test)]
mod test {
    use crate::client::result::{NotificationParameters, RequestParameters, ServerMessage};
    use crate::client::{OutgoingNotification, OutgoingRequest, ProtocolResult};
    use crate::types::{
        Error, InitializeResult, MessageType, ServerCapabilities, ServerInfo, ShowMessage,
        ShowMessageParams, ShowMessageRequest, ShowMessageRequestParams, WorkspaceSymbolResponse,
    };
    use rstest::rstest;
    use serde_json::Value;
    use std::collections::HashMap;
    use tokio::sync::Mutex;
    use tokio_test::io::Mock;

    use super::*;

    #[tokio::test]
    #[test_log::test]
    #[rstest]
    #[case(
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": <ShowMessage as OutgoingNotification>::METHOD,
            "params": ShowMessageParams {
                typ: MessageType::LOG,
                message: "Hello, World!".into(),
            }
        }),
        |incoming| {
            assert!(matches!(incoming, ServerMessage::Notification {
                parameters: NotificationParameters::ShowMessage(ShowMessageParams {
                    typ: MessageType::LOG,
                    message: _
                })
            }));
        }
    )]
    #[case(
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1234,
            "method": ShowMessageRequest::METHOD,
            "params": ShowMessageRequestParams {
                typ: MessageType::LOG,
                message:"Hello, World!".into(), 
                actions: None
            }
        }),
        |incoming| {
            assert!(matches!(incoming, ServerMessage::Request {
                id: NumberOrString::Number(1234),
                parameters: RequestParameters::ShowMessageRequest(ShowMessageRequestParams {
                    typ: MessageType::LOG,
                    message: _,
                    actions: None
                })
            }));
        }
    )]
    pub async fn test_listening_for_server_messages(
        #[case] incoming_message: Value,
        #[case] expected_server_message: impl FnOnce(ServerMessage),
    ) {
        use crate::client::outgoing;

        let (output, mut handle) = tokio_test::io::Builder::new().build_with_handle();

        let output = Arc::new(Mutex::new(output));

        let subscribers: Subscribers = Arc::default();

        let (tx, mut rx) = tokio::sync::mpsc::channel(1);

        {
            let subscribers = Arc::clone(&subscribers);
            let output = Arc::clone(&output);

            tokio::spawn(async move {
                listen_for_messages(output, Option::<Arc<Mutex<Mock>>>::None, subscribers, tx)
                    .await;
            })
        };

        handle.read(outgoing::get_formatted_packet(&incoming_message).as_slice());

        let server_message = rx.recv().await;

        expected_server_message(server_message.expect("Should always have a server message"));
    }

    #[tokio::test]
    #[test_log::test]
    #[rstest]
    #[case(
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1234,
            "result": InitializeResult {
                capabilities: ServerCapabilities::default(),
                server_info: Some(ServerInfo{ name: "Mock".into(), version: Some("1.0".into()) })
            }
        }),
        1234,
        Some(
            Ok(
                IncomingResult {
                    id: NumberOrString::Number(1234),
                    result: serde_json::json!({
                        "capabilities": ServerCapabilities::default(),
                        "serverInfo": Some(ServerInfo{ name: "Mock".into(), version: Some("1.0".into()) })
                    })
                }
            )
        )
    )]
    #[case(
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1234,
            "error": Error {
                code: -1234,
                message: "Uh, oh.".into(),
                data: None
            }
        }),
        1234,
        Some(
            Err(
                IncomingError {
                    id:NumberOrString::Number(1234),
                    error: Error {
                        code: -1234,
                        message: "Uh, oh.".into(),
                        data: None
                    }
                }
            )
        )
    )]
    #[case(
        serde_json::json!({
            "jsonrpc": "2.0",
            "token": 9999,
            "method": "$/progress",
            "value": WorkspaceSymbolResponse::Flat(vec![])
        }),
        9999,
        Some(
            Ok(
                IncomingResult {
                    id: NumberOrString::Number(9999),
                    result: serde_json::json!([])
                }
            )
        )
    )]
    pub async fn test_listening_for_subscribed_messages(
        #[case] incoming_message: Value,
        #[case] subscribed_id: i32,
        #[case] expected_result: Option<Result<IncomingResult, IncomingError>>,
    ) {
        use crate::client::outgoing;

        let (output, mut handle) = tokio_test::io::Builder::new().build_with_handle();

        let output = Arc::new(Mutex::new(output));

        let (tx, mut rx) =
            tokio::sync::mpsc::channel::<ProtocolResult<IncomingResult, IncomingError>>(1);

        let mut subscribers = HashMap::new();
        subscribers.insert(
            NumberOrString::Number(subscribed_id),
            Subscriber::new(tx, []),
        );

        let subscribers: Subscribers = Arc::new(Mutex::new(subscribers));

        {
            let subscribers = Arc::clone(&subscribers);
            let output = Arc::clone(&output);

            tokio::spawn(async move {
                listen_for_messages(
                    output,
                    Option::<Arc<Mutex<Mock>>>::None,
                    subscribers,
                    tokio::sync::mpsc::channel(1).0,
                )
                .await;
            })
        };

        handle.read(outgoing::get_formatted_packet(&incoming_message).as_slice());

        let result = rx.recv().await;

        assert_eq!(result, expected_result);
    }
}
