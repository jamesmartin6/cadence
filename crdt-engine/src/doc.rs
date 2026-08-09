use std::fmt;

use crate::id::OpId;
use crate::op::Operation;
use crate::rga::Rga;

/// A single site's view of a collaboratively-edited document.
///
/// Wraps an [`Rga`] and adds the bookkeeping needed to translate plain-text indices
/// (what a text editor UI works with) into the id-based references the CRDT needs, and
/// to mint fresh ids for this site's own edits.
///
/// `next_counter` is a Lamport clock, not a plain local counter: it advances not only on
/// each local edit but also whenever a remote op is observed, to at least one past that
/// op's counter. This matters for correctness, not just bookkeeping — the RGA integration
/// rule breaks ties between sibling inserts (nodes inserted after the same anchor) by
/// comparing ids, and that tie-break is only meaningful if "already observed when I made
/// this edit" reliably implies "smaller id". A plain per-site counter doesn't guarantee
/// that: a site's very first local edit would get counter 0 even after it had already
/// observed a remote op with a much higher counter, wrongly making the remote op look
/// "newer" and scrambling insertion order for a purely causal (non-concurrent) edit.
#[derive(Debug, Clone)]
pub struct Doc {
    site_id: u32,
    next_counter: u64,
    rga: Rga,
}

impl Doc {
    pub fn new(site_id: u32) -> Self {
        Self {
            site_id,
            next_counter: 0,
            rga: Rga::new(),
        }
    }

    pub fn site_id(&self) -> u32 {
        self.site_id
    }

    pub fn len(&self) -> usize {
        self.rga.visible_len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn fresh_id(&mut self) -> OpId {
        let id = OpId::new(self.site_id, self.next_counter);
        self.next_counter += 1;
        id
    }

    /// Advance the Lamport clock past an id we've just observed (generated remotely).
    fn observe(&mut self, id: OpId) {
        self.next_counter = self.next_counter.max(id.counter + 1);
    }

    /// The id to use as `after` for an insert at visible-text position `index`:
    /// the id of the character currently at `index - 1`, or `None` at the very start.
    fn after_for_index(&self, index: usize) -> Option<OpId> {
        if index == 0 {
            None
        } else {
            self.rga.nth_visible_id(index - 1)
        }
    }

    /// Insert `ch` at visible-text position `index`, generating a fresh local id.
    /// Returns the [`Operation`] to broadcast to other sites.
    pub fn insert_local(&mut self, index: usize, ch: char) -> Operation {
        let after = self.after_for_index(index);
        let id = self.fresh_id();
        self.rga.integrate_insert(id, after, ch);
        Operation::Insert {
            id,
            after,
            char: ch,
        }
    }

    /// Delete the character at visible-text position `index`.
    /// Returns the [`Operation`] to broadcast to other sites.
    ///
    /// # Panics
    /// Panics if `index` is out of bounds — callers (the WASM/UI layer) are expected to
    /// only call this for a valid, currently-visible index.
    pub fn delete_local(&mut self, index: usize) -> Operation {
        let target = self
            .rga
            .nth_visible_id(index)
            .expect("delete_local: index out of bounds");
        self.rga.integrate_delete(target);
        Operation::Delete { target }
    }

    /// Integrate an operation received from another site.
    pub fn apply_remote(&mut self, op: Operation) {
        match op {
            Operation::Insert { id, after, char } => {
                self.observe(id);
                self.rga.integrate_insert(id, after, char);
            }
            Operation::Delete { target } => self.rga.integrate_delete(target),
        }
    }
}

impl fmt::Display for Doc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.rga)
    }
}
