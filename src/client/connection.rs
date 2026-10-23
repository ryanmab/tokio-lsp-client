use std::sync::Arc;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout};
use tokio::sync::Mutex;

#[allow(type_alias_bounds)]
pub type In<W: AsyncWrite> = Arc<Mutex<W>>;
#[allow(type_alias_bounds)]
pub type Out<R: AsyncRead> = Arc<Mutex<R>>;
#[allow(type_alias_bounds)]
pub type ErrorOut<R: AsyncRead> = Out<R>;

#[derive(Debug)]
pub enum Connection {
    /// <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#implementationConsiderations>
    StdIo {
        #[allow(dead_code)]
        process: Child,
        stdin: In<ChildStdin>,
        stdout: Out<ChildStdout>,
        stderr: Option<ErrorOut<ChildStderr>>,
    },
    // TODO(RM): The language server protocol also recommends servers implement support
    // for pipe, socket, and node-ipc transport protocols. This would be the right place to
    // provide client support for this.
    // <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#implementationConsiderations>
}

impl Connection {
    pub(crate) fn input(&self) -> In<impl AsyncWrite> {
        match self {
            Self::StdIo { stdin, .. } => Arc::clone(stdin),
        }
    }

    pub(crate) fn output(&self) -> Out<impl AsyncRead> {
        match self {
            Self::StdIo { stdout, .. } => Arc::clone(stdout),
        }
    }

    pub(crate) fn error(&self) -> Option<ErrorOut<impl AsyncRead>> {
        #[allow(unreachable_patterns)]
        #[allow(clippy::match_wildcard_for_single_variants)]
        match self {
            Self::StdIo {
                stderr: Some(stderr),
                ..
            } => Some(Arc::clone(stderr)),
            _ => None,
        }
    }
}
