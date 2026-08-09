use std::fmt;

use crate::id::OpId;

/// One character in the sequence, tombstoned rather than removed on delete so that
/// concurrent operations which reference it (e.g. "insert after this id") still resolve.
#[derive(Debug, Clone)]
pub struct Node {
    pub id: OpId,
    pub after: Option<OpId>,
    pub ch: char,
    pub deleted: bool,
}

/// The Replicated Growable Array: an ordered list of nodes (including tombstones) that
/// every replica converges on regardless of the order operations are integrated in.
#[derive(Debug, Clone, Default)]
pub struct Rga {
    nodes: Vec<Node>,
}

impl Rga {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    fn index_of(&self, id: OpId) -> Option<usize> {
        self.nodes.iter().position(|n| n.id == id)
    }

    /// Integrate an insert (local or remote) at its causally-correct position.
    ///
    /// `after` is the id of the node this was inserted after (`None` = start of document).
    /// The core RGA rule: starting right after `after`, skip forward over any run of
    /// sibling nodes (nodes also inserted directly after the same `after`) that sort
    /// higher than `id`, and also skip over any nodes inserted even deeper (after one of
    /// those siblings) since they belong further right regardless of their id. This
    /// produces the same final ordering no matter what order concurrent inserts arrive in.
    pub fn integrate_insert(&mut self, id: OpId, after: Option<OpId>, ch: char) {
        let left: isize = match after {
            None => -1,
            Some(a) => self
                .index_of(a)
                .expect("integrate_insert: `after` id must already be present")
                as isize,
        };

        let mut right = left + 1;
        while (right as usize) < self.nodes.len() {
            let candidate = &self.nodes[right as usize];
            let candidate_origin: isize = match candidate.after {
                None => -1,
                Some(o) => self
                    .index_of(o)
                    .expect("integrate_insert: candidate origin must already be present")
                    as isize,
            };

            if candidate_origin < left {
                // candidate's origin is to the left of our anchor: it belongs before us.
                break;
            } else if candidate_origin == left {
                // direct sibling of the same anchor: higher id sorts first, deterministically.
                if candidate.id > id {
                    right += 1;
                    continue;
                } else {
                    break;
                }
            } else {
                // candidate is nested deeper than our anchor (inserted after a sibling of
                // ours, or after something further right) — it belongs further right too.
                right += 1;
                continue;
            }
        }

        self.nodes.insert(
            right as usize,
            Node {
                id,
                after,
                ch,
                deleted: false,
            },
        );
    }

    /// Mark a node as deleted (tombstone). A no-op if the target isn't known yet —
    /// callers are expected to only deliver deletes after their target insert has
    /// already been integrated (true for both local edits and the relay's causal
    /// delivery order).
    pub fn integrate_delete(&mut self, target: OpId) {
        if let Some(idx) = self.index_of(target) {
            self.nodes[idx].deleted = true;
        }
    }

    /// Number of visible (non-tombstoned) characters.
    pub fn visible_len(&self) -> usize {
        self.nodes.iter().filter(|n| !n.deleted).count()
    }

    /// The id of the `n`th visible character (0-indexed), if it exists.
    pub fn nth_visible_id(&self, n: usize) -> Option<OpId> {
        self.nodes
            .iter()
            .filter(|node| !node.deleted)
            .nth(n)
            .map(|node| node.id)
    }

    fn text(&self) -> String {
        self.nodes
            .iter()
            .filter(|n| !n.deleted)
            .map(|n| n.ch)
            .collect()
    }
}

impl fmt::Display for Rga {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.text())
    }
}
