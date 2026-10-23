use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::client::ProtocolResult;
use crate::types::{Error, NumberOrString};
use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;
use tokio::sync::mpsc::Sender;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct IncomingResult {
    pub id: NumberOrString,
    pub result: Value,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct IncomingError {
    pub id: NumberOrString,
    pub error: Error,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(untagged)]
#[non_exhaustive]
pub enum IncomingMessage {
    Request {
        id: NumberOrString,

        method: String,

        #[serde(rename = "params")]
        parameters: Option<Value>,
    },
    Result(IncomingResult),
    Error(IncomingError),
    Notification {
        method: String,

        token: Option<NumberOrString>,

        #[serde(rename = "params")]
        parameters: Option<Value>,

        value: Option<Value>,
    },
}

pub type Subscribers = Arc<Mutex<HashMap<NumberOrString, Subscriber>>>;

#[derive(Debug)]
pub struct Subscriber {
    pub(crate) tx: Sender<ProtocolResult<IncomingResult, IncomingError>>,

    /// Sibling subscriber IDs which should be unsubscribed from messages
    /// at the same time as the current subscriber.
    ///
    ///
    /// Particularly important for language servers which (may) make use of partial result progress
    /// messages, and therefore may deliver delivers using [`crate::client::incoming::IncomingMessage::Result`], or
    /// [`crate::client::incoming::IncomingMessage::Notification`] with different IDs.
    ///
    /// <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#partialResults>
    pub(crate) siblings: Vec<NumberOrString>,
}

impl Subscriber {
    pub fn new<'a>(
        tx: Sender<ProtocolResult<IncomingResult, IncomingError>>,
        siblings: impl IntoIterator<Item = Option<&'a NumberOrString>>,
    ) -> Self {
        Self {
            tx,
            siblings: siblings.into_iter().fold(Vec::new(), |mut acc, s| {
                if let Some(sibling) = s {
                    acc.push(sibling.clone());
                }

                acc
            }),
        }
    }
}
