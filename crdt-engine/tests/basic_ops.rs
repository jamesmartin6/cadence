use crdt_engine::Doc;

/// A client that reconnects to the relay server receives the *entire* op log again, not
/// just what it missed (see progress.md / build plan) -- so re-delivering an op the
/// local doc already has must be a safe no-op, not a duplicate character.
#[test]
fn reapplying_an_already_known_remote_op_is_a_no_op() {
    let mut origin = Doc::new(1);
    let ops: Vec<_> = "hi"
        .chars()
        .enumerate()
        .map(|(i, ch)| origin.insert_local(i, ch))
        .collect();

    let mut replica = Doc::new(2);
    for op in &ops {
        replica.apply_remote(op.clone());
    }
    assert_eq!(replica.to_string(), "hi");

    // Redeliver the exact same ops again (e.g. a fresh WebSocket connection replaying
    // the full history after a reconnect).
    for op in &ops {
        replica.apply_remote(op.clone());
    }
    assert_eq!(
        replica.to_string(),
        "hi",
        "redelivering known ops must not duplicate characters"
    );
    assert_eq!(replica.len(), 2);
}

#[test]
fn sequential_local_inserts_produce_correct_string() {
    let mut doc = Doc::new(1);
    for (i, ch) in "hello".chars().enumerate() {
        doc.insert_local(i, ch);
    }
    assert_eq!(doc.to_string(), "hello");
}

#[test]
fn inserting_in_the_middle_shifts_correctly() {
    let mut doc = Doc::new(1);
    for (i, ch) in "helo".chars().enumerate() {
        doc.insert_local(i, ch);
    }
    // "helo" -> insert 'l' at index 3 -> "hello"
    doc.insert_local(3, 'l');
    assert_eq!(doc.to_string(), "hello");
}

#[test]
fn local_delete_removes_correct_character() {
    let mut doc = Doc::new(1);
    for (i, ch) in "hello".chars().enumerate() {
        doc.insert_local(i, ch);
    }
    doc.delete_local(0); // remove 'h'
    assert_eq!(doc.to_string(), "ello");

    doc.delete_local(3); // remove the trailing 'o'
    assert_eq!(doc.to_string(), "ell");
}

#[test]
fn tombstoned_characters_do_not_reappear_in_output() {
    let mut doc = Doc::new(1);
    doc.insert_local(0, 'a');
    doc.insert_local(1, 'b');
    doc.insert_local(2, 'c');
    doc.delete_local(1); // remove 'b'
    assert_eq!(doc.to_string(), "ac");
    assert_eq!(doc.len(), 2);
}

#[test]
fn remote_ops_replay_to_reconstruct_document() {
    let mut origin = Doc::new(1);
    let ops: Vec<_> = "abc"
        .chars()
        .enumerate()
        .map(|(i, ch)| origin.insert_local(i, ch))
        .collect();

    let mut replica = Doc::new(2);
    for op in ops {
        replica.apply_remote(op);
    }
    assert_eq!(replica.to_string(), "abc");
}

/// Builds three independent replicas (distinct site_ids, as every real site must have)
/// that have all already converged on the 2-character string "ac".
fn three_synced_replicas() -> (Doc, Doc, Doc) {
    let mut seed = Doc::new(0);
    let seed_ops = vec![seed.insert_local(0, 'a'), seed.insert_local(1, 'c')];

    let mut a = Doc::new(1);
    let mut b = Doc::new(2);
    let mut c = Doc::new(3);
    for op in seed_ops {
        a.apply_remote(op.clone());
        b.apply_remote(op.clone());
        c.apply_remote(op);
    }
    (a, b, c)
}

/// The core CRDT guarantee, demonstrated directly: two sites concurrently insert at the
/// same position (neither has seen the other's edit yet). Applying both resulting
/// operations in either order must converge to the identical final string.
#[test]
fn concurrent_inserts_at_same_position_converge_regardless_of_order() {
    let (mut site_a, mut site_b, mut site_c) = three_synced_replicas();

    // Both sites concurrently insert between 'a' and 'c', unaware of each other.
    let op_a = site_a.insert_local(1, 'X');
    let op_b = site_b.insert_local(1, 'Y');

    // Site A applies its own op locally already; now receives B's op.
    site_a.apply_remote(op_b.clone());
    // Site B applies its own op locally already; now receives A's op, in the OPPOSITE order.
    site_b.apply_remote(op_a.clone());

    assert_eq!(
        site_a.to_string(),
        site_b.to_string(),
        "sites must converge even though they applied the concurrent ops in opposite order"
    );

    // A third, fresh replica that only ever sees the remote ops (never generated its own)
    // should also converge to the same result, applied in yet another order.
    site_c.apply_remote(op_b);
    site_c.apply_remote(op_a);
    assert_eq!(site_a.to_string(), site_c.to_string());
}

#[test]
fn delete_of_tombstoned_char_referenced_by_later_insert_still_resolves() {
    // "ab" on two independent, already-synced replicas. Site A deletes 'a' concurrently
    // while Site B inserts right after 'a' — neither has seen the other's edit yet.
    let mut seed = Doc::new(0);
    let seed_ops = vec![seed.insert_local(0, 'a'), seed.insert_local(1, 'b')];

    let mut site_a = Doc::new(1);
    let mut site_b = Doc::new(2);
    for op in seed_ops {
        site_a.apply_remote(op.clone());
        site_b.apply_remote(op);
    }

    let del_op = site_a.delete_local(0); // delete 'a'
    let ins_op = site_b.insert_local(1, 'X'); // insert 'X' after 'a', before 'b' -> "aXb"

    site_a.apply_remote(ins_op.clone());
    site_b.apply_remote(del_op.clone());

    assert_eq!(site_a.to_string(), site_b.to_string());
    assert_eq!(site_a.to_string(), "Xb");
}
