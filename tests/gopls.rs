use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use tokio::process::Command;
use tokio_lsp_client::Client;
use tokio_lsp_client::client::NotificationParameters::ShowMessage;
use tokio_lsp_client::client::ServerMessage::Notification;
use tokio_lsp_client::types::{
    InitializeParams, MessageType, ShowMessageParams, SymbolKind, TraceValue, Uri, WorkspaceFolder,
    WorkspaceSymbolParams, WorkspaceSymbolRequest, WorkspaceSymbolResponse,
};
use tokio_stream::StreamExt;

/// The path to the LSP binary.
const BINARY_PATH: &str = "gopls";

/// The path to the mock Go project.
const MOCK_PROJECT_PATH: &[&str] = &["tests", "fixtures", "go", "mock-project"];

/// The query to use when performing a workspace symbol search.
const WORKSPACE_SYMBOL_QUERY: &str = "HelloWorld";

/// The maximum amount of iterations allowed polling the server message
/// and result streams.
const MAX_POLLING_ITERATIONS: u8 = 20;

/// The maximum amount of time (in milliseconds) to wait between each poll
/// of the server message and result streams.
const MAX_SLEEP_TIME_MS: u8 = 150;

#[tokio::test]
#[test_log::test]
pub async fn test_connecting_to_gopls() {
    let (mut client, mut server_message_stream) =
        Client::from_command(&mut Command::new(BINARY_PATH))
            .expect("Should never fail to start the LSP server");

    assert!(
        client
            .initialize(InitializeParams {
                workspace_folders: Some(vec![WorkspaceFolder {
                    name: String::default(),
                    uri: Uri::from_str(&format!(
                        "file://{}",
                        std::env::current_dir()
                            .expect("Go mock project should always exist")
                            .join(PathBuf::from_iter(MOCK_PROJECT_PATH))
                            .to_string_lossy()
                            .replace('\\', "/")
                    ))
                    .expect("Uri should always be valid")
                }]),
                trace: Some(TraceValue::Verbose),
                ..Default::default()
            })
            .await
            .is_ok()
    );

    assert!(client.initialized().await.is_ok());

    let mut response = client
        .send_request::<WorkspaceSymbolRequest>(WorkspaceSymbolParams {
            query: WORKSPACE_SYMBOL_QUERY.to_string(),
            ..Default::default()
        })
        .await
        .expect("Should never fail to request workspace symbols");

    let mut has_loaded_packages = false;
    let mut has_returned_workspace_symbols = false;

    for _ in 1..MAX_POLLING_ITERATIONS {
        tokio::select! {
            Some(message) = server_message_stream.next() => {
                if let Notification { parameters: ShowMessage(ShowMessageParams{ typ: r#type, message })} = message
                    && r#type == MessageType::INFO && message == "Finished loading packages." {
                    has_loaded_packages = true;
                };
            },
            Some(Ok(Some(WorkspaceSymbolResponse::Flat(symbols)))) = response.next() => {
                has_returned_workspace_symbols = has_returned_workspace_symbols ||
                    symbols
                        .iter()
                        .any(
                            |symbol| symbol.kind == SymbolKind::FUNCTION
                                && symbol.name == WORKSPACE_SYMBOL_QUERY
                        );
            },
            _ = tokio::time::sleep(Duration::from_millis(MAX_SLEEP_TIME_MS as u64)) => { }
        }
        log::debug!(
            "Has returned workspace symbols: {}, has loaded packages: {}",
            has_returned_workspace_symbols,
            has_loaded_packages
        );

        if has_loaded_packages && has_returned_workspace_symbols {
            break;
        }

        tokio::time::sleep(Duration::from_millis(MAX_SLEEP_TIME_MS as u64)).await
    }

    assert!(client.shutdown().await.is_ok());
    assert!(client.exit().await.is_ok());

    assert!(
        has_returned_workspace_symbols,
        "gopls did not return the expected workspace symbols."
    );
    assert!(
        has_loaded_packages,
        "gopls did not finish loading packages."
    );
}
