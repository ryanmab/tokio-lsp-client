use serde::{Deserialize, Serialize};

/// Error originating from the language server.
///
/// <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#responseError>
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
#[non_exhaustive]
pub struct Error {
    /// A unique code indicating the cause of the error.
    ///
    /// Some codes are reserved for the JSON-RPC specification (-32099 to -32000 inclusive) and inherited by
    /// the language server protocol, and others are reserved by the specification (-32899 to -32800 inclusive).
    ///
    /// <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#errorCodes>
    pub code: i64,

    /// A human-readable descriptive message of the error.
    pub message: String,

    /// Data relevant to the error.
    pub data: Option<crate::types::LSPAny>,
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        // "-32700" indicates a parse error, as defined by the JSON-RPC specification.
        //
        // <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#errorCodes>
        Self {
            code: -32700,
            message: value.to_string(),
            data: None,
        }
    }
}
