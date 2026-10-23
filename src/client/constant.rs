/// The maximum number of messages that can be buffered in the channel for
/// server messages intended for client consumption.
pub const SERVER_MESSAGE_CHANNEL_CAPACITY: usize = 16;

/// The maximum number of messages that can be buffered in the channel for
/// streaming results back to the client as part of a [`crate::client::PartialResult`]
/// stream.
pub const RESULT_CHANNEL_BUFFER_CAPACITY: usize = 16;
