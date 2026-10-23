[![Coverage](https://api.coveragerobot.com/v1/graph/github/ryanmab/tokio-lsp-client/badge.svg?token=578f758e9fed7430ccd2784301005f6963f3372831f7cf0c12)](https://coveragerobot.com)
[![Crates.io Version](https://img.shields.io/crates/v/tokio-lsp-client)](https://crates.io/crates/tokio-lsp-client)
![Crates.io Total Downloads](https://img.shields.io/crates/d/tokio-lsp-client)
[![docs.rs](https://img.shields.io/docsrs/tokio-lsp-client)](https://docs.rs/tokio-lsp-client)
[![Build](https://github.com/ryanmab/tokio-lsp-client/actions/workflows/build.yml/badge.svg)](https://github.com/ryanmab/tokio-lsp-client/actions/workflows/build.yml)
![GitHub License](https://img.shields.io/github/license/ryanmab/tokio-lsp-client)

<!-- cargo-rdme start -->

# tokio-lsp-client

A community-maintained asynchronous client for communicating with Language Servers over the Language Server Protocol (LSP).

It is fully spec-complaint against v3.18.0 of the [Language Server Protocol](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/),
including support for streamed results using [partial result tokens](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#partialResults),
automatic [request cancellation](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#cancelRequest) and much more.

## Getting Started

```toml
[dependencies]
tokio-lsp-client = "0.0.1"
```

### Example

```rust
use tokio_lsp_client::Client;
use tokio_lsp_client::types::{InitializeParams, InitializeResult, WorkspaceSymbolParams, WorkspaceSymbolRequest};
use tokio::process::Command;

let (mut client, mut server_message_stream) = Client::from_command(&mut Command::new("rust-analyzer"))?;

let _ = client
    .initialize(InitializeParams::default())
    .await?;

client.initialized().await?;

let workspace_symbols: Vec<_> = client
    .send_request::<WorkspaceSymbolRequest>(WorkspaceSymbolParams {
        query: "Client".to_string(),
        ..Default::default()
    })
    .await?
    .collect()
    .await;

println!("{:#?}", workspace_symbols);

client.shutdown().await?;
client.exit().await.map_err(|(_, err)| err)?;

```

### Documentation

Full documentation is available on [docs.rs](https://docs.rs/tokio_lsp_client/latest/tokio_lsp_client/).

## Contributing

Contributions are very welcome!

If you have suggestions, bug reports, or would like to contribute, please open an issue
or submit a pull request.

### Testing

This crate includes comprehensive tests all the way from the low-level transport logic, up to
integration testing against live language servers.

In order to run the full test suite, the following language server binaries will need to be
in the terminal `$PATH`:

1. [`rust-analyzer`](https://rust-analyzer.github.io/book/rust_analyzer_binary.html)
2. [`gopls`](https://go.dev/gopls/#installation)

Run the tests with:

```bash
# Without logging
cargo test

# With logging
RUST_LOG=<trace/debug/info/...> cargo test -- --nocapture
```

<!-- cargo-rdme end -->
