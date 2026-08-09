//! Property-based convergence testing: the single highest-signal test in this project.
//!
//! Strategy: generate a random sequence of local edits (insert/delete) spread across
//! 2-4 simulated sites, each site only ever acting on its own current view (exactly like
//! real concurrent editors). This produces a set of [`Operation`]s with an implicit
//! causal dependency graph (an insert depends on the `after` id it referenced; a delete
//! depends on the id of the character it targets).
//!
//! We then construct several independent random *topological* orderings of that same
//! op set (valid orderings respect causal dependencies, exactly like a relay server that
//! only guarantees causal, not identical, delivery order to every client) and replay each
//! ordering into a fresh [`Doc`]. If the RGA integration rule is correct, every replica
//! must land on the exact same final string no matter which valid order it saw the ops in.

use std::collections::HashSet;

use crdt_engine::{Doc, OpId, Operation};
use proptest::prelude::*;

const NUM_REPLICAS: usize = 4;
const MAX_PRIORITY_POOL: usize = 40;

#[derive(Debug, Clone)]
enum ActionSpec {
    Insert(u16, char),
    Delete(u16),
}

fn action_strategy() -> impl Strategy<Value = ActionSpec> {
    let ch = (0u8..26).prop_map(|n| (b'a' + n) as char);
    prop_oneof![
        (any::<u16>(), ch).prop_map(|(p, c)| ActionSpec::Insert(p, c)),
        any::<u16>().prop_map(ActionSpec::Delete),
    ]
}

/// The id this op must wait behind, if any: an insert waits on its `after` id, a delete
/// waits on the id of the character it targets.
fn prerequisite(op: &Operation) -> Option<OpId> {
    match op {
        Operation::Insert { after, .. } => *after,
        Operation::Delete { target } => Some(*target),
    }
}

/// A valid (causally-respecting) topological order of `ops`, chosen by always picking the
/// ready op with the lowest assigned priority. Different `priorities` arrays yield
/// different, independently-random valid orderings of the same op set.
fn topo_order_by_priority(ops: &[Operation], priorities: &[u32]) -> Vec<Operation> {
    let mut emitted_ids: HashSet<OpId> = HashSet::new();
    let mut done = vec![false; ops.len()];
    let mut order = Vec::with_capacity(ops.len());

    for _ in 0..ops.len() {
        let mut best: Option<usize> = None;
        for i in 0..ops.len() {
            if done[i] {
                continue;
            }
            let ready = prerequisite(&ops[i]).is_none_or(|dep| emitted_ids.contains(&dep));
            if !ready {
                continue;
            }
            if best.is_none_or(|b| priorities[i] < priorities[b]) {
                best = Some(i);
            }
        }
        let idx = best.expect("dependency graph is acyclic by construction: some op must be ready");
        done[idx] = true;
        if let Operation::Insert { id, .. } = &ops[idx] {
            emitted_ids.insert(*id);
        }
        order.push(ops[idx].clone());
    }
    order
}

proptest! {
    #[test]
    fn convergence_under_random_causal_orderings(
        num_sites in 2usize..=4,
        raw_actions in prop::collection::vec((0usize..4, action_strategy()), 1..MAX_PRIORITY_POOL),
        priority_pools in prop::collection::vec(
            prop::collection::vec(any::<u32>(), MAX_PRIORITY_POOL),
            NUM_REPLICAS,
        ),
    ) {
        let actions: Vec<(usize, ActionSpec)> = raw_actions
            .into_iter()
            .map(|(s, a)| (s % num_sites, a))
            .collect();

        let mut sites: Vec<Doc> = (0..num_sites).map(|i| Doc::new(i as u32)).collect();
        let mut ops: Vec<Operation> = Vec::new();
        let mut op_priorities: Vec<Vec<u32>> = vec![Vec::new(); NUM_REPLICAS];

        for (i, (site_idx, action)) in actions.iter().enumerate() {
            let site = &mut sites[*site_idx];
            let produced = match action {
                ActionSpec::Insert(pos_raw, ch) => {
                    let pos = (*pos_raw as usize) % (site.len() + 1);
                    Some(site.insert_local(pos, *ch))
                }
                ActionSpec::Delete(pos_raw) => {
                    let len = site.len();
                    if len == 0 {
                        None
                    } else {
                        Some(site.delete_local((*pos_raw as usize) % len))
                    }
                }
            };
            if let Some(op) = produced {
                ops.push(op);
                for (r, pool) in op_priorities.iter_mut().enumerate() {
                    pool.push(priority_pools[r][i]);
                }
            }
        }

        prop_assume!(!ops.is_empty());

        let results: Vec<String> = (0..NUM_REPLICAS)
            .map(|r| {
                let order = topo_order_by_priority(&ops, &op_priorities[r]);
                let mut replica = Doc::new(1000 + r as u32);
                for op in order {
                    replica.apply_remote(op);
                }
                replica.to_string()
            })
            .collect();

        for pair in results.windows(2) {
            prop_assert_eq!(
                &pair[0],
                &pair[1],
                "replicas diverged after integrating the same ops in different causally-valid orders"
            );
        }
    }
}
