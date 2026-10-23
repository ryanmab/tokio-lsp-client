use thiserror::Error;

/// Errors which occur while maintaining (or establishing) a connection with a language server.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum ConnectionError {
    /// A connection to the language server could not be established.
    #[error(
        "A connection to the input and output handles of the language server connection could not be established."
    )]
    InvalidConnection,

    /// The language server process could not be spawned.
    #[error("Failed to spawn the language server's process: {0}")]
    CouldNotSpawn(std::io::Error),
}

/// Errors which occur while interacting with a connection to a language server.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum ProtocolError {
    /// Failed to read a message from the output handle of a connection.
    #[error(transparent)]
    Read(ReadError),

    /// Failed to write a message to the input handle of a connection.
    #[error(transparent)]
    Write(WriteError),
}

/// Errors which occurred while reading off of an output stream.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum ReadError {
    /// Failed to read bytes off of an output stream.
    #[error("Unable to read message from server: {0}")]
    CouldNotRead(std::io::Error),

    /// Failed to read the provided bytes as a valid message.
    #[error("Unable to deserialize message from server: {0}")]
    InvalidBytes(serde_json::Error),

    /// Response provided by the language server was not valid (unexpected).
    #[error("The server sent an invalid response: {0:?}")]
    InvalidResponse(crate::types::Error),
}

/// Errors which occurred while writing to an input stream.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum WriteError {
    /// Failed to write a message to the input handle of a connection to a language server.
    #[error("Unable to send message to server: {0}")]
    CouldNotWrite(std::io::Error),
}
