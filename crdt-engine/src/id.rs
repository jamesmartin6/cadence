use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// A globally unique, totally ordered identifier for a single inserted character.
///
/// Ordering is by `counter` first, then `site_id` as a tiebreaker. Every site hands out
/// strictly increasing counters for its own operations, so `(site_id, counter)` pairs
/// are never reused and the resulting total order is the same on every replica —
/// this is what lets concurrent inserts at the same position resolve deterministically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OpId {
    pub site_id: u32,
    pub counter: u64,
}

impl OpId {
    pub fn new(site_id: u32, counter: u64) -> Self {
        Self { site_id, counter }
    }
}

impl PartialOrd for OpId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OpId {
    fn cmp(&self, other: &Self) -> Ordering {
        self.counter
            .cmp(&other.counter)
            .then_with(|| self.site_id.cmp(&other.site_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orders_by_counter_first() {
        let a = OpId::new(5, 1);
        let b = OpId::new(1, 2);
        assert!(a < b, "lower counter must sort first regardless of site_id");
    }

    #[test]
    fn ties_broken_by_site_id() {
        let a = OpId::new(1, 7);
        let b = OpId::new(2, 7);
        assert!(a < b, "equal counters break ties by site_id");
    }

    #[test]
    fn equal_ids_are_equal() {
        assert_eq!(OpId::new(3, 4), OpId::new(3, 4));
    }
}
