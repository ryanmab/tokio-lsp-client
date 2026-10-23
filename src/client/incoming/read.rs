use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, BufReader};

use crate::client::connection::{ErrorOut, Out};
use crate::client::incoming::IncomingMessage;
use crate::client::{self};

/// Read a message off of an output channel and parse it into an incoming message which can
/// be processed downstream.
///
/// <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#baseProtocol>
pub async fn get_message(
    output: Out<impl AsyncRead + Unpin + Send + Sync>,
) -> std::result::Result<Option<IncomingMessage>, client::ReadError> {
    let body = {
        let mut guard = output.lock().await;

        let mut reader = BufReader::new(&mut *guard);

        let content_length = parse_content_length(&mut reader).await;

        if let Err(client::ReadError::CouldNotRead(err)) = &content_length
            && err.kind() == std::io::ErrorKind::UnexpectedEof
        {
            return Ok(None);
        }

        let mut body = vec![0; content_length?];

        reader
            .read_exact(&mut body)
            .await
            .map_err(client::ReadError::CouldNotRead)?;

        drop(guard);

        body
    };

    let message = serde_json::from_slice::<IncomingMessage>(body.as_slice())
        .map_err(client::ReadError::InvalidBytes)?;

    log::trace!("Incoming message: {message:?}");

    Ok(Some(message))
}

/// <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#baseProtocol>
async fn parse_content_length(
    reader: &mut BufReader<impl AsyncRead + Unpin + Send>,
) -> std::result::Result<usize, client::ReadError> {
    let mut is_first_read = true;
    let mut line = String::new();
    let mut content_length = None;

    loop {
        line.clear();

        reader
            .read_line(&mut line)
            .await
            .map_err(client::ReadError::CouldNotRead)?;

        if line.trim().is_empty() {
            break;
        }

        is_first_read = false;

        let Some((header, value)) = line.split_once(':') else {
            return Err(client::ReadError::CouldNotRead(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "Received message was expected to be a Content-Length header, but it was not valid. Full line was: {line}"
                ),
            )));
        };

        match header.trim().to_lowercase().as_str() {
            "content-type" => {
                // Content Type is referenced in the LSP spec (generally defaults to: "application/vscode-jsonrpc; charset=utf-8"),
                // we don't currently need to do anything with this.
                //
                // <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#contentPart>
            }
            "content-length" => {
                content_length = Some(value
                        .trim()
                        .parse()
                        .map_err(
                            |err| client::ReadError::CouldNotRead(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                format!(
                                    "Received message did not provide a valid Content-Length in bytes. Error was {err}. Full header was: {line}",
                                ),
                            ))
                        )?);
            }
            header => {
                return Err(client::ReadError::CouldNotRead(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "An unexpected header ({header}) was provided by the language server. The full header was: {line}"
                    ),
                )));
            }
        }
    }

    let Some(content_length) = content_length else {
        if is_first_read {
            return Err(client::ReadError::CouldNotRead(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Reached EOF",
            )));
        }

        return Err(client::ReadError::CouldNotRead(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Expected a Content-Length header, but none was present.",
        )));
    };

    Ok(content_length)
}

pub async fn handle_error(output: ErrorOut<impl AsyncRead + Unpin + Send>) {
    let error = {
        let mut error = String::new();

        let mut guard = output.lock().await;

        let mut reader = BufReader::new(&mut *guard);

        if let Err(e) = reader.read_line(&mut error).await {
            Err(e)
        } else {
            Ok(error)
        }
    };

    log::warn!("Received an unexpected error message from language server: {error:?}");
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use serde_json::error::Category;
    use serde_json::json;
    use std::io;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    use crate::client::ReadError::{CouldNotRead, InvalidBytes};
    use crate::client::incoming::IncomingMessage::{Notification, Request};

    use super::*;

    #[tokio::test]
    #[rstest]
    #[case(b"", None)]
    #[case(
        b"Content-Length: 52\n\n{\"jsonrpc\":\"2.0\",\"method\":\"initialized\",\"params\":{}}",
        Some(Notification { method: "initialized".into(), token: None, parameters: Some(json!({})), value: None })
    )]
    #[case(
        b"Content-Length: 52\nContent-Type: mock\n\n{\"jsonrpc\":\"2.0\",\"method\":\"initialized\",\"params\":{}}",
        Some(Notification { method: "initialized".into(), token: None, parameters: Some(json!({})), value: None })
    )]
    #[case(
        b"Content-Length: 204\n\n{\"id\":\"0e6e9094-1dbe-4cc6-868f-54f7cb713793\",\"jsonrpc\":\"2.0\",\"method\":\"initialize\",\"params\":{\"capabilities\":{},\"processId\":null,\"rootUri\":null,\"workspaceFolders\":[{\"name\":\"mock-name\",\"uri\":\"mock-path\"}]}}",
       Some(Request { id: crate::types::NumberOrString::String("0e6e9094-1dbe-4cc6-868f-54f7cb713793".into()), method: "initialize".into(), parameters: Some(json!({"capabilities": {}, "processId": null, "rootUri": null, "workspaceFolders": [{"name": "mock-name", "uri": "mock-path"}]})) }) 
    )]
    #[case(
        b"Content-Length: 95\n\n{\"id\":\"c1d849d9-06a8-4f3b-9eab-2619b7cc9386\",\"jsonrpc\":\"2.0\",\"method\":\"shutdown\",\"params\":null}",
        Some(Request { id: crate::types::NumberOrString::String("c1d849d9-06a8-4f3b-9eab-2619b7cc9386".into()), method: "shutdown".into(), parameters: None })
    )]
    pub async fn test_reading_message(
        #[case] input: &[u8],
        #[case] expected: Option<IncomingMessage>,
    ) {
        let out = tokio_test::io::Builder::new().read(input).build();

        let message = get_message(Arc::new(Mutex::new(out)))
            .await
            .expect("Should always read a valid message");

        assert_eq!(message, expected);
    }

    #[tokio::test]
    pub async fn test_reading_malformed_message_json() {
        let out = tokio_test::io::Builder::new()
            .read(b"Content-Length: 17\n\n{\"jsonrpc\":\"2.0\"}")
            .build();

        let error = get_message(Arc::new(Mutex::new(out)))
            .await
            .expect_err("Should always fail to read the malformed JSON");

        let InvalidBytes(error) = error else {
            panic!("Reading message should yield an InvalidBytes error");
        };

        assert_eq!(error.classify(), Category::Data);
    }

    #[tokio::test]
    pub async fn test_reading_malformed_content_length() {
        let out = tokio_test::io::Builder::new()
            .read(b"Content-Length: 5\n\n{\"jsonrpc\":\"2.0\",\"method\":\"initialized\",\"params\":{}}")
            .build();

        let error = get_message(Arc::new(Mutex::new(out)))
            .await
            .expect_err("Should always fail to read the malformed JSON");

        let InvalidBytes(error) = error else {
            panic!("Reading message should yield an InvalidBytes error");
        };

        assert_eq!(error.classify(), Category::Eof);
    }

    #[tokio::test]
    #[rstest]
    #[case(b"{\"jsonrpc\":\"2.0\",\"method\":\"initialized\",\"params\":{}}")]
    #[case(b"Content-Type: mock\n\n{\"jsonrpc\":\"2.0\",\"method\":\"initialized\",\"params\":{}}")]
    #[case(b"Content-Length: 5\nSome-Unexpected-Header: true\n\n{\"jsonrpc\":\"2.0\",\"method\":\"initialized\",\"params\":{}}")]
    #[case(b"Content-Length: not-a-number\n\n{\"jsonrpc\":\"2.0\",\"method\":\"initialized\",\"params\":{}}")]
    #[case(
        b"Content-Length: -999\n\n{\"jsonrpc\":\"2.0\",\"method\":\"initialized\",\"params\":{}}"
    )]
    pub async fn test_reading_malformed_headers(#[case()] input: &[u8]) {
        let out = tokio_test::io::Builder::new().read(input).build();

        let error = get_message(Arc::new(Mutex::new(out)))
            .await
            .expect_err("Should always fail to read the malformed JSON");

        let CouldNotRead(error) = error else {
            panic!("Reading message should yield an InvalidBytes error");
        };

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }
}
