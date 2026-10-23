use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::types::{
    ApplyWorkspaceEditParams, CallHierarchyIncomingCallsParams, CallHierarchyOutgoingCallsParams,
    CallHierarchyPrepareParams, CancelParams, CodeAction, CodeActionParams, CodeLens,
    CodeLensParams, ColorPresentationParams, CompletionItem, CompletionParams, ConfigurationParams,
    CreateFilesParams, DeleteFilesParams, DidChangeConfigurationParams,
    DidChangeNotebookDocumentParams, DidChangeWatchedFilesParams, DidChangeWorkspaceFoldersParams,
    DidCloseNotebookDocumentParams, DidCloseTextDocumentParams, DidOpenNotebookDocumentParams,
    DidOpenTextDocumentParams, DidSaveNotebookDocumentParams, DidSaveTextDocumentParams,
    DocumentColorParams, DocumentDiagnosticParams, DocumentFormattingParams,
    DocumentHighlightParams, DocumentLink, DocumentLinkParams, DocumentOnTypeFormattingParams,
    DocumentRangeFormattingParams, DocumentSymbolParams, ExecuteCommandParams, FoldingRangeParams,
    GotoDefinitionParams, HoverParams, InitializeParams, InitializedParams, InlayHint,
    InlayHintParams, InlineValueParams, LinkedEditingRangeParams, LogMessageParams, LogTraceParams,
    MonikerParams, NumberOrString, OneOf, ProgressParams, PublishDiagnosticsParams,
    ReferenceParams, RegistrationParams, RenameFilesParams, RenameParams, SelectionRangeParams,
    SemanticTokensDeltaParams, SemanticTokensParams, SemanticTokensRangeParams, SetTraceParams,
    ShowDocumentParams, ShowMessageParams, ShowMessageRequestParams, SignatureHelpParams,
    TextDocumentPositionParams, TypeHierarchyPrepareParams, TypeHierarchySubtypesParams,
    TypeHierarchySupertypesParams, UnregistrationParams, WillSaveTextDocumentParams,
    WorkDoneProgressCancelParams, WorkDoneProgressCreateParams, WorkspaceDiagnosticParams,
    WorkspaceSymbol, WorkspaceSymbolParams,
};

/// A message received from the language server.
///
/// [`ServerMessage`] enables bidirectional communication (and out-of-band
/// messages) between the server and client.
///
/// <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#abstractMessage>
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum ServerMessage {
    /// A request originating from the server.
    ///
    /// <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#requestMessage>
    Request {
        /// The unique ID of the request.
        ///
        /// A response served from the client should carry this ID in order for the
        /// server to reconcile the response to the request.
        ///
        /// <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#responseMessage>
        id: NumberOrString,

        /// The parameters of the request.
        #[serde(rename = "params", flatten)]
        parameters: RequestParameters,
    },
    /// A notification originating from the server.
    ///
    /// <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#notificationMessage>
    Notification {
        /// The parameters of the notification.
        #[serde(rename = "params", flatten)]
        parameters: NotificationParameters,
    },
}

/// Request parameters which can be received through a [`crate::client::ServerMessage::Request`].
#[allow(clippy::large_enum_variant, missing_docs)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "method", content = "params")]
#[non_exhaustive]
pub enum RequestParameters {
    #[serde(rename = "workspace/applyEdit")]
    ApplyWorkspaceEdit(ApplyWorkspaceEditParams),

    #[serde(rename = "callHierarchy/incomingCalls")]
    CallHierarchyIncomingCalls(CallHierarchyIncomingCallsParams),

    #[serde(rename = "callHierarchy/outgoingCalls")]
    CallHierarchyOutgoingCalls(CallHierarchyOutgoingCallsParams),

    #[serde(rename = "textDocument/prepareCallHierarchy")]
    CallHierarchyPrepare(CallHierarchyPrepareParams),

    #[serde(rename = "textDocument/codeAction")]
    CodeActionRequest(CodeActionParams),

    #[serde(rename = "codeAction/resolve")]
    CodeActionResolveRequest(CodeAction),

    #[serde(rename = "workspace/codeLens/refresh")]
    CodeLensRefresh(()),

    #[serde(rename = "textDocument/codeLens")]
    CodeLensRequest(CodeLensParams),

    #[serde(rename = "codeLens/resolve")]
    CodeLensResolve(CodeLens),

    #[serde(rename = "textDocument/colorPresentation")]
    ColorPresentationRequest(ColorPresentationParams),

    #[serde(rename = "textDocument/completion")]
    Completion(CompletionParams),

    #[serde(rename = "textDocument/documentColor")]
    DocumentColor(DocumentColorParams),

    #[serde(rename = "textDocument/diagnostic")]
    DocumentDiagnosticRequest(DocumentDiagnosticParams),

    #[serde(rename = "textDocument/documentHighlight")]
    DocumentHighlightRequest(DocumentHighlightParams),

    #[serde(rename = "textDocument/documentLink")]
    DocumentLinkRequest(DocumentLinkParams),

    #[serde(rename = "documentLink/resolve")]
    DocumentLinkResolve(DocumentLink),

    #[serde(rename = "textDocument/documentSymbol")]
    DocumentSymbolRequest(DocumentSymbolParams),

    #[serde(rename = "workspace/executeCommand")]
    ExecuteCommand(ExecuteCommandParams),

    #[serde(rename = "textDocument/foldingRange")]
    FoldingRangeRequest(FoldingRangeParams),

    #[serde(rename = "textDocument/formatting")]
    Formatting(DocumentFormattingParams),

    #[serde(rename = "textDocument/declaration")]
    GotoDeclaration(GotoDefinitionParams),

    #[serde(rename = "textDocument/definition")]
    GotoDefinition(GotoDefinitionParams),

    #[serde(rename = "textDocument/implementation")]
    GotoImplementation(GotoDefinitionParams),

    #[serde(rename = "textDocument/typeDefinition")]
    GotoTypeDefinition(GotoDefinitionParams),

    #[serde(rename = "textDocument/hover")]
    HoverRequest(HoverParams),

    #[serde(rename = "initialize")]
    Initialize(InitializeParams),

    #[serde(rename = "workspace/inlayHint/refresh")]
    InlayHintRefreshRequest(()),

    #[serde(rename = "textDocument/inlayHint")]
    InlayHintRequest(InlayHintParams),

    #[serde(rename = "inlayHint/resolve")]
    InlayHintResolveRequest(InlayHint),

    #[serde(rename = "workspace/inlineValue/refresh")]
    InlineValueRefreshRequest(()),

    #[serde(rename = "textDocument/inlineValue")]
    InlineValueRequest(InlineValueParams),

    #[serde(rename = "textDocument/linkedEditingRange")]
    LinkedEditingRange(LinkedEditingRangeParams),

    #[serde(rename = "textDocument/moniker")]
    Moniker(MonikerParams),

    #[serde(rename = "textDocument/onTypeFormatting")]
    OnTypeFormatting(DocumentOnTypeFormattingParams),

    #[serde(rename = "textDocument/prepareRename")]
    PrepareRenameRequest(TextDocumentPositionParams),

    #[serde(rename = "textDocument/rangeFormatting")]
    RangeFormatting(DocumentRangeFormattingParams),

    #[serde(rename = "textDocument/references")]
    Reference(ReferenceParams),

    #[serde(rename = "client/registerCapability")]
    RegisterCapability(RegistrationParams),

    #[serde(rename = "textDocument/rename")]
    Rename(RenameParams),

    #[serde(rename = "completionItem/resolve")]
    ResolveCompletionItem(CompletionItem),

    #[serde(rename = "textDocument/selectionRange")]
    SelectionRangeRequest(SelectionRangeParams),

    #[serde(rename = "textDocument/semanticTokens/full/delta")]
    SemanticTokensFullDeltaRequest(SemanticTokensDeltaParams),

    #[serde(rename = "textDocument/semanticTokens/full")]
    SemanticTokensFullRequest(SemanticTokensParams),

    #[serde(rename = "textDocument/semanticTokens/range")]
    SemanticTokensRangeRequest(SemanticTokensRangeParams),

    #[serde(rename = "workspace/semanticTokens/refresh")]
    SemanticTokensRefresh(()),

    #[serde(rename = "window/showDocument")]
    ShowDocument(ShowDocumentParams),

    #[serde(rename = "window/showMessageRequest")]
    ShowMessageRequest(ShowMessageRequestParams),

    #[serde(rename = "shutdown")]
    Shutdown(()),

    #[serde(rename = "textDocument/signatureHelp")]
    SignatureHelpRequest(SignatureHelpParams),

    #[serde(rename = "textDocument/prepareTypeHierarchy")]
    TypeHierarchyPrepare(TypeHierarchyPrepareParams),

    #[serde(rename = "typeHierarchy/subtypes")]
    TypeHierarchySubtypes(TypeHierarchySubtypesParams),

    #[serde(rename = "typeHierarchy/supertypes")]
    TypeHierarchySupertypes(TypeHierarchySupertypesParams),

    #[serde(rename = "client/unregisterCapability")]
    UnregisterCapability(UnregistrationParams),

    #[serde(rename = "workspace/willCreateFiles")]
    WillCreateFiles(CreateFilesParams),

    #[serde(rename = "workspace/willDeleteFiles")]
    WillDeleteFiles(DeleteFilesParams),

    #[serde(rename = "workspace/willRenameFiles")]
    WillRenameFiles(RenameFilesParams),

    #[serde(rename = "textDocument/willSaveWaitUntil")]
    WillSaveTextDocument(WillSaveTextDocumentParams),

    #[serde(rename = "window/workDoneProgress/create")]
    WorkDoneProgressCreate(WorkDoneProgressCreateParams),

    #[serde(rename = "workspace/configuration")]
    WorkspaceConfiguration(ConfigurationParams),

    #[serde(rename = "workspace/diagnostic/refresh")]
    WorkspaceDiagnosticRefresh(()),

    #[serde(rename = "workspace/diagnostic")]
    WorkspaceDiagnosticRequest(WorkspaceDiagnosticParams),

    #[serde(rename = "workspace/workspaceFolders")]
    WorkspaceFoldersRequest(()),

    #[serde(rename = "workspace/symbol")]
    WorkspaceSymbolRequest(WorkspaceSymbolParams),

    #[serde(rename = "workspaceSymbol/resolve")]
    WorkspaceSymbolResolve(WorkspaceSymbol),
}

/// Notification parameters which can be received through a [`crate::client::ServerMessage::Notification`].
#[allow(missing_docs)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "method", content = "params")]
#[non_exhaustive]
pub enum NotificationParameters {
    #[serde(rename = "$/cancelRequest")]
    Cancel(CancelParams),

    #[serde(rename = "workspace/didChangeConfiguration")]
    DidChangeConfiguration(DidChangeConfigurationParams),

    #[serde(rename = "notebookDocument/didChange")]
    DidChangeNotebookDocument(DidChangeNotebookDocumentParams),

    #[serde(rename = "textDocument/didChange")]
    DidChangeTextDocument(DidChangeNotebookDocumentParams),

    #[serde(rename = "workspace/didChangeWatchedFiles")]
    DidChangeWatchedFiles(DidChangeWatchedFilesParams),

    #[serde(rename = "workspace/didChangeWorkspaceFolders")]
    DidChangeWorkspaceFolders(DidChangeWorkspaceFoldersParams),

    #[serde(rename = "notebookDocument/didClose")]
    DidCloseNotebookDocument(DidCloseNotebookDocumentParams),

    #[serde(rename = "textDocument/didClose")]
    DidCloseTextDocument(DidCloseTextDocumentParams),

    #[serde(rename = "workspace/didCreateFiles")]
    DidCreateFiles(CreateFilesParams),

    #[serde(rename = "workspace/didDeleteFiles")]
    DidDeleteFiles(DeleteFilesParams),

    #[serde(rename = "notebookDocument/didOpen")]
    DidOpenNotebookDocument(DidOpenNotebookDocumentParams),

    #[serde(rename = "textDocument/didOpen")]
    DidOpenTextDocument(DidOpenTextDocumentParams),

    #[serde(rename = "workspace/didRenameFiles")]
    DidRenameFiles(RenameFilesParams),

    #[serde(rename = "notebookDocument/didSave")]
    DidSaveNotebookDocument(DidSaveNotebookDocumentParams),

    #[serde(rename = "textDocument/didSave")]
    DidSaveTextDocument(DidSaveTextDocumentParams),

    #[serde(rename = "exit")]
    Exit(()),

    #[serde(rename = "initialized")]
    Initialized(InitializedParams),

    #[serde(rename = "window/logMessage")]
    LogMessage(LogMessageParams),

    #[serde(rename = "$/logTrace")]
    LogTrace(LogTraceParams),

    #[serde(rename = "$/progress")]
    Progress(ProgressParams),

    #[serde(rename = "textDocument/publishDiagnostics")]
    PublishDiagnostics(PublishDiagnosticsParams),

    #[serde(rename = "$/setTrace")]
    SetTrace(SetTraceParams),

    #[serde(rename = "window/showMessage")]
    ShowMessage(ShowMessageParams),

    #[serde(rename = "telemetry/event")]
    TelemetryEvent(OneOf<Map<String, Value>, Vec<Value>>),

    #[serde(rename = "textDocument/willSave")]
    WillSaveTextDocument(WillSaveTextDocumentParams),

    #[serde(rename = "window/workDoneProgress/cancel")]
    WorkDoneProgressCancel(WorkDoneProgressCancelParams),
}
