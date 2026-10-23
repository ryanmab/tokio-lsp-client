#![crate_name = "tokio_lsp_client"]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(
    clippy::borrowed_box,
    clippy::should_implement_trait,
    clippy::new_without_default,
    clippy::wrong_self_convention,
    clippy::len_without_is_empty,
    clippy::exhaustive_enums
)]
#![warn(
    missing_debug_implementations,
    missing_copy_implementations,
    rust_2018_idioms,
    rustdoc::all
)]
#![allow(rustdoc::private_doc_tests)]
#![forbid(unsafe_code)]

//! # tokio-lsp-client
//!
//! A community-maintained asynchronous client for communicating with Language Servers over the Language Server Protocol (LSP).
//!
//! It is fully spec-complaint against v3.18.0 of the [Language Server Protocol](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/),
//! including support for streamed results using [partial result tokens](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#partialResults),
//! automatic [request cancellation](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#cancelRequest) and much more.
//!
//! ## Getting Started
//!
//! ```toml
//! [dependencies]
//! tokio-lsp-client = "0.0.1"
//! ```
//!
//! ### Example
//!
//! ```rust
//! use tokio_lsp_client::Client;
//! use tokio_lsp_client::types::{InitializeParams, InitializeResult, WorkspaceSymbolParams, WorkspaceSymbolRequest};
//! use tokio::process::Command;
//! # use tokio_stream::StreamExt;
//! # use std::result::Result;
//! # use std::error::Error;
//!
//! # #[tokio::main]
//! # pub async fn main() -> Result<(), Box<dyn Error>> {
//! let (mut client, mut server_message_stream) = Client::from_command(&mut Command::new("rust-analyzer"))?;
//!
//! let _ = client
//!     .initialize(InitializeParams::default())
//!     .await?;
//!
//! client.initialized().await?;
//!
//! # // Wait for the first server message to come back (usually a diagnostic refresh)
//! # // to ensure we're not sending requests before the server is ready
//! # server_message_stream.next().await;
//! #
//! let workspace_symbols: Vec<_> = client
//!     .send_request::<WorkspaceSymbolRequest>(WorkspaceSymbolParams {
//!         query: "Client".to_string(),
//!         ..Default::default()
//!     })
//!     .await?
//!     .collect()
//!     .await;
//!
//! println!("{:#?}", workspace_symbols);
//!
//! client.shutdown().await?;
//! client.exit().await.map_err(|(_, err)| err)?;
//!
//! # Ok(())
//! # }
//! ```
//!
//! ### Documentation
//!
//! Full documentation is available on [docs.rs](https://docs.rs/tokio_lsp_client/latest/tokio_lsp_client/).
//!
//! ## Contributing
//!
//! Contributions are very welcome!
//!
//! If you have suggestions, bug reports, or would like to contribute, please open an issue
//! or submit a pull request.
//!
//! ### Testing
//!
//! This crate includes comprehensive tests all the way from the low-level transport logic, up to
//! integration testing against live language servers.
//!
//! In order to run the full test suite, the following language server binaries will need to be
//! in the terminal `$PATH`:
//!
//! 1. [`rust-analyzer`](https://rust-analyzer.github.io/book/rust_analyzer_binary.html)
//! 2. [`gopls`](https://go.dev/gopls/#installation)
//!
//! Run the tests with:
//!
//! ```bash
//! # Without logging
//! cargo test
//!
//! # With logging
//! RUST_LOG=<trace/debug/info/...> cargo test -- --nocapture
//! ```

/// Client for establishing a connection with a language server.
pub mod client;

/// Types for sending type safe requests and notifications, as well as
/// receiving results, notifications, requests, and errors from a
/// language server.
pub mod types;

pub use client::Client;
