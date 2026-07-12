use aether::crdt::{DistributedSystem, GCounter, GSet, ORSet, PNCounter};

// =========================================================================
// 1. PROPERTY-BASED CRDT MATHEMATICAL TESTS (Commutativity, Associativity, Idempotency)
// =========================================================================

#[test]
fn test_gcounter_properties() {
    let mut a = GCounter::new("node_a");
    let mut b = GCounter::new("node_b");
    let mut c = GCounter::new("node_c");

    a.increment_by(5);
    b.increment_by(10);
    c.increment_by(3);

    // 1. Idempotency: x.merge(x) == x
    let mut a_idemp = a.clone();
    a_idemp.merge(&a);
    assert_eq!(a_idemp, a);

    // 2. Commutativity: a.merge(b) == b.merge(a)
    let mut ab = a.clone();
    ab.merge(&b);

    let mut ba = b.clone();
    ba.merge(&a);

    assert_eq!(ab.value(), ba.value());
    assert_eq!(ab.counts, ba.counts);

    // 3. Associativity: (a.merge(b)).merge(c) == a.merge(b.merge(c))
    let mut left = ab.clone();
    left.merge(&c);

    let mut bc = b.clone();
    bc.merge(&c);
    let mut right = a.clone();
    right.merge(&bc);

    assert_eq!(left.value(), right.value());
    assert_eq!(left.counts, right.counts);
}

#[test]
fn test_gset_properties() {
    let mut a = GSet::new();
    let mut b = GSet::new();
    let mut c = GSet::new();

    a.add(1);
    a.add(2);
    b.add(2);
    b.add(3);
    c.add(4);

    // Idempotency
    let mut a_idemp = a.clone();
    a_idemp.merge(&a);
    assert_eq!(a_idemp, a);

    // Commutativity
    let mut ab = a.clone();
    ab.merge(&b);
    let mut ba = b.clone();
    ba.merge(&a);
    assert_eq!(ab, ba);

    // Associativity
    let mut left = ab.clone();
    left.merge(&c);
    let mut bc = b.clone();
    bc.merge(&c);
    let mut right = a.clone();
    right.merge(&bc);
    assert_eq!(left, right);
}

#[test]
fn test_pncounter_properties() {
    let mut a = PNCounter::new("node_a");
    let mut b = PNCounter::new("node_b");
    let mut c = PNCounter::new("node_c");

    a.increment();
    a.increment();
    a.decrement();

    b.increment();
    b.decrement();
    b.decrement();

    c.increment();

    // Idempotency
    let mut a_idemp = a.clone();
    a_idemp.merge(&a);
    assert_eq!(a_idemp, a);

    // Commutativity
    let mut ab = a.clone();
    ab.merge(&b);
    let mut ba = b.clone();
    ba.merge(&a);
    assert_eq!(ab, ba);

    // Associativity
    let mut left = ab.clone();
    left.merge(&c);
    let mut bc = b.clone();
    bc.merge(&c);
    let mut right = a.clone();
    right.merge(&bc);
    assert_eq!(left, right);
}

#[test]
fn test_orset_properties() {
    let mut a = ORSet::new("node_a");
    let mut b = ORSet::new("node_b");
    let mut c = ORSet::new("node_c");

    a.add("apple");
    a.add("banana");
    a.remove(&"apple");

    b.add("banana");
    b.add("cherry");

    c.add("date");
    c.remove(&"date");

    // Idempotency
    let mut a_idemp = a.clone();
    a_idemp.merge(&a);
    assert_eq!(a_idemp, a);

    // Commutativity
    let mut ab = a.clone();
    ab.merge(&b);
    let mut ba = b.clone();
    ba.merge(&a);
    assert_eq!(ab.read(), ba.read());

    // Associativity
    let mut left = ab.clone();
    left.merge(&c);
    let mut bc = b.clone();
    bc.merge(&c);
    let mut right = a.clone();
    right.merge(&bc);
    assert_eq!(left.read(), right.read());
}

// =========================================================================
// 2. CONVERGENCE & DISTRIBUTED SIMULATION TESTS
// =========================================================================

#[test]
fn test_distributed_convergence_gcounter() {
    let mut cluster = DistributedSystem::new();
    let num_nodes = 50;

    // Initialize 50 nodes
    for i in 0..num_nodes {
        let name = format!("node_{}", i);
        cluster.add_node(&name, GCounter::new(&name));
    }

    // Perform random increments across random nodes
    let mut seed = 42;
    for _ in 0..500 {
        seed = (seed * 31 + 17) % 50;
        let node_name = format!("node_{}", seed);
        cluster.update(&node_name, |c| c.increment());
    }

    // Sync all nodes to ensure full convergence
    for i in 0..num_nodes {
        for j in 0..num_nodes {
            let from = format!("node_{}", i);
            let to = format!("node_{}", j);
            cluster.sync(&from, &to);
        }
    }

    assert!(cluster.verify_convergence());

    // Value should be correct and identical across all
    let val = cluster.nodes.values().next().unwrap().value();
    assert_eq!(val, 500);
}

#[test]
fn test_distributed_convergence_orset() {
    let mut cluster = DistributedSystem::new();
    let num_nodes = 10;

    for i in 0..num_nodes {
        let name = format!("node_{}", i);
        cluster.add_node(&name, ORSet::new(&name));
    }

    // Node 0 adds, Node 1 adds, Node 0 syncs to 2, Node 2 removes
    cluster.update("node_0", |s| s.add("hello"));
    cluster.update("node_1", |s| s.add("world"));

    // Sync node 0 to node 2
    cluster.sync("node_0", "node_2");
    assert!(cluster.nodes.get("node_2").unwrap().contains(&"hello"));

    // Node 2 removes "hello"
    cluster.update("node_2", |s| s.remove(&"hello"));
    assert!(!cluster.nodes.get("node_2").unwrap().contains(&"hello"));

    // Sync all nodes to each other recursively
    for _ in 0..3 {
        for i in 0..num_nodes {
            for j in 0..num_nodes {
                let from = format!("node_{}", i);
                let to = format!("node_{}", j);
                cluster.sync(&from, &to);
            }
        }
    }

    // Verify all nodes converged
    assert!(cluster.verify_convergence());

    // The state should have converged: "hello" is removed (tombstoned), "world" is active
    let converged_state = cluster.nodes.values().next().unwrap();
    assert!(!converged_state.contains(&"hello"));
    assert!(converged_state.contains(&"world"));
}

// =========================================================================
// 3. STRESS TEST (10,000 operations)
// =========================================================================

#[test]
fn test_crdt_stress_execution() {
    let mut cluster = DistributedSystem::new();
    let num_nodes = 5;

    for i in 0..num_nodes {
        let name = format!("node_{}", i);
        cluster.add_node(&name, PNCounter::new(&name));
    }

    let mut seed: u64 = 12345;
    for _ in 0..10000 {
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345) & 0x7fffffff;
        let node_idx = seed % num_nodes;
        let op = seed % 2;
        let node_name = format!("node_{}", node_idx);

        if op == 0 {
            cluster.update(&node_name, |c| c.increment());
        } else {
            cluster.update(&node_name, |c| c.decrement());
        }
    }

    // Sync all nodes
    for _ in 0..2 {
        for i in 0..num_nodes {
            for j in 0..num_nodes {
                let from = format!("node_{}", i);
                let to = format!("node_{}", j);
                cluster.sync(&from, &to);
            }
        }
    }

    assert!(cluster.verify_convergence());
    println!(
        "Stress test convergence success. PNCounter value: {}",
        cluster.nodes.values().next().unwrap().value()
    );
}
