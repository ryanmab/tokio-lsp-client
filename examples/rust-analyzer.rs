use tokio::process::Command;
use tokio_lsp_client::Client;
use tokio_lsp_client::types::{InitializeParams, InitializeResult};
use tokio_stream::StreamExt;

#[tokio::main]
pub async fn main() {
    let (mut client, mut server_message_stream) =
        Client::from_command(&mut Command::new("rust-analyzer"))
            .expect("Should never fail to start the LSP server");

    tokio::spawn(async move {
        while let Some(message) = server_message_stream.next().await {
            println!("Received message from server: {:?}", message);
        }
    });

    let InitializeResult { server_info, .. } = client
        .initialize(InitializeParams::default())
        .await
        .expect("Should never fail to initialize the server");

    println!(
        "Rust Analyzer version: {:?}",
        server_info
            .map(|info| info.version.unwrap_or_else(|| "Unknown".into()))
            .unwrap_or_else(|| "Unknown".into())
    );

    assert!(client.initialized().await.is_ok());

    assert!(client.shutdown().await.is_ok());
    assert!(client.exit().await.is_ok());
}
