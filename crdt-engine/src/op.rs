use serde::{Deserialize, Serialize};

use crate::id::OpId;

/// A single edit, as generated locally or received from a remote site.
///
/// Operations are the only thing that ever crosses the network — the relay server
/// just relays and stores these, it never needs to understand RGA integration itself.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Operation {
    Insert {
        id: OpId,
        after: Option<OpId>,
        char: char,
    },
    Delete {
        target: OpId,
    },
}
