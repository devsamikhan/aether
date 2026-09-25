// ==============================================================================
// AetherGraph Unit & Integration Test Suite
// Verifying In-Memory Property Graph, Dijkstra, PageRank, BFS/DFS & Hybrid Vectors
// Pure Rust Standard Library (Zero Third-Party Crates)
// ==============================================================================

use aether::vm::graph::{create_graph, get_graph};
use aether::vm::run_source;
use std::collections::HashMap;

#[test]
fn test_graph_dijkstra_shortest_path() {
    let gid = create_graph("transport_network");
    let g_arc = get_graph(gid).expect("Failed to get graph");

    let (node_a, node_b, node_c, node_d);
    {
        let mut g = g_arc.lock().unwrap();
        node_a = g.add_node("Station_A", HashMap::new(), None);
        node_b = g.add_node("Station_B", HashMap::new(), None);
        node_c = g.add_node("Station_C", HashMap::new(), None);
        node_d = g.add_node("Station_D", HashMap::new(), None);

        // A -> B (weight 4.0)
        // A -> C (weight 2.0)
        // C -> B (weight 1.0)
        // B -> D (weight 5.0)
        // C -> D (weight 8.0)
        let _ = g.add_edge(node_a, node_b, "TRACK", 4.0, HashMap::new());
        let _ = g.add_edge(node_a, node_c, "TRACK", 2.0, HashMap::new());
        let _ = g.add_edge(node_c, node_b, "TRACK", 1.0, HashMap::new());
        let _ = g.add_edge(node_b, node_d, "TRACK", 5.0, HashMap::new());
        let _ = g.add_edge(node_c, node_d, "TRACK", 8.0, HashMap::new());
    }

    let g = g_arc.lock().unwrap();
    let res = g.dijkstra_shortest_path(node_a, node_d);
    assert!(res.is_some());
    let (path, cost) = res.unwrap();

    // Optimal path is A -> C -> B -> D with cost 2 + 1 + 5 = 8.0
    assert_eq!(path, vec![node_a, node_c, node_b, node_d]);
    assert!((cost - 8.0).abs() < 1e-6);
}

#[test]
fn test_graph_pagerank_authority_distribution() {
    let gid = create_graph("web_pages");
    let g_arc = get_graph(gid).expect("Failed to get graph");

    let (p1, p2, p3, authority);
    {
        let mut g = g_arc.lock().unwrap();
        p1 = g.add_node("Page1", HashMap::new(), None);
        p2 = g.add_node("Page2", HashMap::new(), None);
        p3 = g.add_node("Page3", HashMap::new(), None);
        authority = g.add_node("AuthorityHub", HashMap::new(), None);

        // All pages link to AuthorityHub
        let _ = g.add_edge(p1, authority, "LINKS_TO", 1.0, HashMap::new());
        let _ = g.add_edge(p2, authority, "LINKS_TO", 1.0, HashMap::new());
        let _ = g.add_edge(p3, authority, "LINKS_TO", 1.0, HashMap::new());
        // Hub links back to Page1
        let _ = g.add_edge(authority, p1, "LINKS_TO", 1.0, HashMap::new());
    }

    let g = g_arc.lock().unwrap();
    let ranks = g.pagerank(30, 0.85);

    let auth_rank = ranks.get(&authority).copied().unwrap_or(0.0);
    let p2_rank = ranks.get(&p2).copied().unwrap_or(0.0);
    let p3_rank = ranks.get(&p3).copied().unwrap_or(0.0);

    // AuthorityHub receives 3 incoming links, so its rank must be significantly higher than P2 and P3
    assert!(auth_rank > p2_rank);
    assert!(auth_rank > p3_rank);
}

#[test]
fn test_graph_bfs_and_dfs_traversals() {
    let gid = create_graph("hierarchy_tree");
    let g_arc = get_graph(gid).expect("Failed to get graph");

    let (root, child1, child2, leaf1);
    {
        let mut g = g_arc.lock().unwrap();
        root = g.add_node("Root", HashMap::new(), None);
        child1 = g.add_node("Child1", HashMap::new(), None);
        child2 = g.add_node("Child2", HashMap::new(), None);
        leaf1 = g.add_node("Leaf1", HashMap::new(), None);

        let _ = g.add_edge(root, child1, "PARENT_OF", 1.0, HashMap::new());
        let _ = g.add_edge(root, child2, "PARENT_OF", 1.0, HashMap::new());
        let _ = g.add_edge(child1, leaf1, "PARENT_OF", 1.0, HashMap::new());
    }

    let g = g_arc.lock().unwrap();
    let bfs_order = g.bfs(root, 5);
    assert_eq!(bfs_order[0], root);
    assert!(bfs_order.contains(&child1));
    assert!(bfs_order.contains(&child2));
    assert!(bfs_order.contains(&leaf1));

    let dfs_order = g.dfs(root, 5);
    assert_eq!(dfs_order[0], root);
    assert_eq!(dfs_order.len(), 4);
}

#[test]
fn test_graph_hybrid_vector_semantic_search() {
    let gid = create_graph("knowledge_hybrid");
    let g_arc = get_graph(gid).expect("Failed to get graph");

    let (ai_node, physics_node);
    {
        let mut g = g_arc.lock().unwrap();
        // Concept: Artificial Intelligence
        ai_node = g.add_node("Artificial Intelligence", HashMap::new(), Some(vec![0.9, 0.8, 0.1]));
        // Concept: Quantum Physics
        physics_node = g.add_node("Quantum Physics", HashMap::new(), Some(vec![0.1, 0.2, 0.9]));
        // Concept: Molecular Biology
        let _bio_node = g.add_node("Molecular Biology", HashMap::new(), Some(vec![0.2, 0.7, 0.3]));

        let _ = g.add_edge(ai_node, physics_node, "INTERSECTS", 1.0, HashMap::new());
    }

    let g = g_arc.lock().unwrap();
    // Query vector close to AI
    let results = g.semantic_search(&[0.85, 0.75, 0.15], 2);
    assert_eq!(results.len(), 2);
    // Highest match should be ai_node
    assert_eq!(results[0].0, ai_node);
    assert!(results[0].1 > 0.95);
}

#[test]
fn test_graph_aether_stdlib_execution() {
    let script = r#"
import aether_graph

let g = aether_graph.Graph("social_network")
let alice = g.add_node("Alice", {"role": "Engineer"})
let bob = g.add_node("Bob", {"role": "Designer"})
let charlie = g.add_node("Charlie", {"role": "Manager"})

g.add_edge(alice, bob, "COLLABORATES", 1.5)
g.add_edge(bob, charlie, "REPORTS_TO", 2.0)

let path_res = g.shortest_path(alice, charlie)
assert(path_res != nil, "Shortest path should exist")
assert(path_res["cost"] == 3.5, "Shortest path cost should be 3.5")

let order1 = g.bfs(alice, 1)
assert(len(order1) == 2, "Alice + neighbor Bob should be 2 nodes")

let order2 = g.bfs(alice, 2)
assert(len(order2) == 3, "Alice + Bob + Charlie should be 3 nodes")
"#;

    let res = run_source(script);
    assert!(res.is_ok(), "AetherGraph script failed: {:?}", res.err());
}
