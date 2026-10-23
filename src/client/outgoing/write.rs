use crate::types::{self, NumberOrString};
use serde_json::{Value, json};
use std::io::Write;
use tokio::io::{AsyncWrite, AsyncWriteExt};

use crate::client::{self, utils};

/// Send a request to the LSP server.
///
/// With a request, the language server MUST subsequently respond with results on
/// the output channel, which will be picked up by a separate process ([`client::message::listen_for_messages`])
/// and reconciled with the caller.
///
/// <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#requestMessage>
pub async fn send_request<R, In>(
    id: &NumberOrString,
    params: &R::Params,
    input: &mut In,
) -> std::result::Result<utils::token::RequestTokens, client::WriteError>
where
    R: crate::client::OutgoingRequest,
    In: AsyncWrite + Unpin + Send + Sync,
{
    let request = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": R::METHOD,
        "params": params
    });

    write_message(&request, input)
        .await
        .map_err(client::WriteError::CouldNotWrite)?;

    Ok(utils::token::extract_tokens_from_request_params(&request))
}

/// Send a notification to the LSP server.
///
/// <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#notificationMessage>
pub async fn send_notification<N, In>(
    params: &N::Params,
    input: &mut In,
) -> std::result::Result<(), client::WriteError>
where
    N: crate::client::OutgoingNotification,
    In: AsyncWrite + Unpin + Send + Sync,
{
    write_message(
        &json!({
            "jsonrpc": "2.0",
            "method": N::METHOD,
            "params": params
        }),
        input,
    )
    .await
    .map_err(client::WriteError::CouldNotWrite)
}

/// Send a result to the LSP server.
///
/// <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#responseMessage>
pub async fn send_result<R, In>(
    id: NumberOrString,
    result: &R::Result,
    input: &mut In,
) -> std::result::Result<(), client::WriteError>
where
    R: crate::client::OutgoingRequest,
    In: AsyncWrite + Unpin + Send + Sync,
{
    write_message(
        &json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result,
        }),
        input,
    )
    .await
    .map_err(client::WriteError::CouldNotWrite)
}

/// Send an error response to the LSP server.
///
/// <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#responseMessage>
pub async fn send_error<In>(
    id: NumberOrString,
    error: &types::Error,
    input: &mut In,
) -> std::result::Result<(), client::WriteError>
where
    In: AsyncWrite + Unpin + Send + Sync,
{
    write_message(
        &json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": error,
        }),
        input,
    )
    .await
    .map_err(client::WriteError::CouldNotWrite)
}

/// Write a JSON blob onto a [`client::Connection`]'s input channel.
///
/// <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#baseProtocol>
pub async fn write_message<In>(
    message: &Value,
    input: &mut In,
) -> std::result::Result<(), std::io::Error>
where
    In: AsyncWrite + Unpin + Send + Sync,
{
    input
        .write_all(get_formatted_packet(message).as_slice())
        .await?;

    input.flush().await?;

    log::trace!("Outgoing message: {message}");

    Ok(())
}

/// Get a formatted packet which can be consumed by the server, from a json-rpc message - this may
/// be a [`crate::client::OutgoingRequest`] or [`crate::client::OutgoingNotification`].
///
/// The format of a packet is:
///
/// ```ignore
/// Content-Length: <length of message>
///
/// <message>
/// ```
///
/// <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#baseProtocol>
pub fn get_formatted_packet(message: &Value) -> Vec<u8> {
    let message = message.to_string();
    let content_length_prefix = b"Content-Length: ";
    let separator = b"\r\n\r\n";

    let mut packet = Vec::with_capacity(
        content_length_prefix.len()
            + 20 // At most 20 bytes for the usize for the messages length
            + separator.len()
            + message.len(),
    );

    packet.extend_from_slice(content_length_prefix);

    write!(packet, "{}", message.len()).unwrap();

    packet.extend_from_slice(separator);
    packet.extend_from_slice(message.as_bytes());

    packet
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::utils::token::RequestTokens;
    use crate::types;

    use std::str::FromStr;

    #[tokio::test]
    #[test_log::test]
    pub async fn test_writing_request() {
        let params = types::InlayHintParams {
            range: types::Range {
                start: types::Position {
                    line: 0,
                    character: 0,
                },
                end: types::Position {
                    line: 0,
                    character: 1,
                },
            },
            work_done_progress_params: types::WorkDoneProgressParams {
                work_done_token: Some(NumberOrString::String("mock-token".to_string())),
            },
            text_document: types::TextDocumentIdentifier {
                uri: types::Uri::from_str("file://mock/file").expect("URI should always be valid."),
            },
        };

        let mut input = tokio_test::io::Builder::new()
            .write(b"Content-Length: 214\r\n\r\n{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"textDocument/inlayHint\",\"params\":{\"range\":{\"end\":{\"character\":1,\"line\":0},\"start\":{\"character\":0,\"line\":0}},\"textDocument\":{\"uri\":\"file://mock/file\"},\"workDoneToken\":\"mock-token\"}}")
            .build();

        let request_tokens = send_request::<types::InlayHintRequest, _>(
            &types::NumberOrString::Number(1),
            &params,
            &mut input,
        )
        .await
        .expect("Writing request should always succeed.");

        assert_eq!(
            request_tokens,
            RequestTokens {
                work_done: Some(NumberOrString::String("mock-token".into())),
                partial_result: None
            }
        );
    }

    #[tokio::test]
    #[test_log::test]
    pub async fn test_writing_notification() {
        let params = types::ProgressParams {
            token: types::NumberOrString::String("mock-token".to_string()),
            value: types::ProgressParamsValue::WorkDone(types::WorkDoneProgress::Begin(
                types::WorkDoneProgressBegin {
                    title: "Mock".to_string(),
                    cancellable: None,
                    message: Some("Mock".to_string()),
                    percentage: Some(98),
                },
            )),
        };

        let mut input = tokio_test::io::Builder::new()
            .write(b"Content-Length: 144\r\n\r\n{\"jsonrpc\":\"2.0\",\"method\":\"$/progress\",\"params\":{\"token\":\"mock-token\",\"value\":{\"kind\":\"begin\",\"message\":\"Mock\",\"percentage\":98,\"title\":\"Mock\"}}}")
            .build();

        send_notification::<types::Progress, _>(&params, &mut input)
            .await
            .expect("Writing notification should always succeed.");
    }

    #[tokio::test]
    #[test_log::test]
    pub async fn test_writing_result() {
        let mut input = tokio_test::io::Builder::new()
            .write(b"Content-Length: 54\r\n\r\n{\"id\":42,\"jsonrpc\":\"2.0\",\"result\":{\"capabilities\":{}}}")
            .build();

        send_result::<types::Initialize, _>(
            types::NumberOrString::Number(42),
            &types::InitializeResult::default(),
            &mut input,
        )
        .await
        .expect("Writing result should always succeed.");
    }

    #[tokio::test]
    #[test_log::test]
    pub async fn test_writing_error() {
        let mut input = tokio_test::io::Builder::new()
            .write(b"Content-Length: 89\r\n\r\n{\"error\":{\"code\":42,\"data\":{\"Hello\":\"World\"},\"message\":\"Uh oh.\"},\"id\":42,\"jsonrpc\":\"2.0\"}")
            .build();

        send_error(
            types::NumberOrString::Number(42),
            &types::Error {
                code: 42,
                message: "Uh oh.".to_string(),
                data: Some(json!({ "Hello": "World"})),
            },
            &mut input,
        )
        .await
        .expect("Writing error should always succeed.");
    }
}
