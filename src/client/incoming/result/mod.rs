mod partial_result;
mod server_message;
mod server_message_stream;

use crate::{client, types};
pub use partial_result::PartialResult;
pub use server_message::{NotificationParameters, RequestParameters, ServerMessage};
pub use server_message_stream::ServerMessageStream;

/// Result from communicating over the language server protocol.
pub type ProtocolResult<T, E = client::ProtocolError> = std::result::Result<T, E>;

/// Result from attempting to establish a connection with a language server.
pub type ConnectionResult<T, E = client::ConnectionError> = std::result::Result<T, E>;

/// Result from the active language server.
pub type LspResult<T, E = types::Error> = std::result::Result<T, E>;
