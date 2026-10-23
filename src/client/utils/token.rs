use crate::types::NumberOrString;
use serde_json::Value;

#[derive(Debug, PartialEq, Eq, Default)]
pub struct RequestTokens {
    pub partial_result: Option<NumberOrString>,

    #[allow(dead_code)]
    pub work_done: Option<NumberOrString>,
}

impl RequestTokens {
    pub fn iter(&self) -> impl Iterator<Item = Option<&NumberOrString>> {
        [self.partial_result.as_ref(), self.work_done.as_ref()].into_iter()
    }

    pub const fn get_partial_result_token(&self) -> Option<&NumberOrString> {
        self.partial_result.as_ref()
    }

    pub const fn get_work_done_token(&self) -> Option<&NumberOrString> {
        self.work_done.as_ref()
    }
}

/// Extract a copy of the tokens provided to the language server (by the caller) on the request, in order for
/// the LSP client to listen for notifications sent back using those tokens.
///
/// Specifically used for work done and partial result notifications.
///
/// <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#partialResultParams>
/// <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workDoneProgressParams>
pub fn extract_tokens_from_request_params(request: &Value) -> RequestTokens {
    // <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workDoneProgressParams>
    let partial_result = match &request["params"]["partialResultToken"] {
        Value::String(string) => Some(NumberOrString::String(string.clone())),
        Value::Number(number) => number
            .as_i64()
            .and_then(|n| i32::try_from(n).map(NumberOrString::Number).inspect_err(|err| {
                log::error!("Could not extract partial result token ({n}) as it was not a valid signed 32 bit integer: {err}");
            }).ok()),
        _ => None,
    };

    // <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workDoneProgressParams>
    let work_done = match &request["params"]["workDoneToken"] {
        Value::String(string) => Some(NumberOrString::String(string.clone())),
        Value::Number(number) => number
            .as_i64()
            .and_then(|n| i32::try_from(n).map(NumberOrString::Number).inspect_err(|err| {
                log::error!("Could not extract work done token ({n}) as it was not a valid signed 32 bit integer: {err}");
            }).ok()),
        _ => None,
    };

    RequestTokens {
        partial_result,
        work_done,
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use crate::client::OutgoingRequest;
    use crate::types::{
        Initialize, InitializeParams, NumberOrString, PartialResultParams, Uri,
        WorkDoneProgressParams, WorkspaceFolder, WorkspaceSymbolParams, WorkspaceSymbolRequest,
    };
    use rstest::rstest;

    #[rstest]
    #[case(
        serde_json::json!({}),
        RequestTokens {
            partial_result: None,
            work_done: None
        }
    )]
    #[case(
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": "1234",
            "method": Initialize::METHOD,
            "params": InitializeParams {
                workspace_folders: Some(vec![WorkspaceFolder {
                    name: String::default(),
                    uri: Uri::from_str("file://mock-path").expect("String should always be a valid URI"),
                }]),
                ..Default::default()
            }
        }),
        RequestTokens {
            partial_result: None,
            work_done: None
        }
    )]
    #[case(
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": "1234",
            "method": WorkspaceSymbolRequest::METHOD,
            "params": WorkspaceSymbolParams {
                query:"Hello, World!".into(),
                partial_result_params: PartialResultParams { partial_result_token: Some(NumberOrString::String("1".into())) },
                work_done_progress_params: WorkDoneProgressParams { work_done_token: None }
            }
        }),
        RequestTokens {
            partial_result: Some(NumberOrString::String("1".into())),
            work_done: None
        }
    )]
    #[case(
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": "1234",
            "method": WorkspaceSymbolRequest::METHOD,
            "params": WorkspaceSymbolParams {
                query:"Hello, World!".into(),
                partial_result_params: PartialResultParams{ partial_result_token: None },
                work_done_progress_params: WorkDoneProgressParams { work_done_token: Some(NumberOrString::Number(2)) }
            }
        }),
        RequestTokens {
            partial_result: None,
            work_done: Some(NumberOrString::Number(2)),
        }
    )]
    #[case(
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": "1234",
            "method": WorkspaceSymbolRequest::METHOD,
            "params": WorkspaceSymbolParams {
                query:"Hello, World!".into(),
                partial_result_params: PartialResultParams{ partial_result_token: Some(NumberOrString::Number(1)) },
                work_done_progress_params: WorkDoneProgressParams { work_done_token: None }
            }
        }),
        RequestTokens {
            partial_result: Some(NumberOrString::Number(1)),
            work_done: None
        }
    )]
    pub fn test_extracting_tokens_matches_spec(
        #[case] request: Value,
        #[case] expected_tokens: RequestTokens,
    ) {
        assert_eq!(
            extract_tokens_from_request_params(&request),
            expected_tokens
        );
    }
}
