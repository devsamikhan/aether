// ==============================================================================
// AetherGraph — Native Property Graph & Knowledge Traversal Engine
// In-Memory Graph Database, Dijkstra Shortest Path, PageRank & Hybrid Vector Search
// Pure Rust Standard Library (Zero Third-Party Crates)
// ==============================================================================

use super::value::Value;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::sync::{Arc, Mutex, OnceLock};

// ==============================================================================
// 1. Graph Data Structures
// ==============================================================================

#[derive(Clone, Debug)]
pub struct GraphNode {
    pub id: u64,
    pub label: String,
    pub properties: HashMap<String, Value>,
    pub embedding: Option<Vec<f64>>,
}

#[derive(Clone, Debug)]
pub struct GraphEdge {
    pub id: u64,
    pub from: u64,
    pub to: u64,
    pub label: String,
    pub weight: f64,
    pub properties: HashMap<String, Value>,
}

#[derive(Clone, Debug)]
pub struct PropertyGraph {
    pub id: u64,
    pub name: String,
    pub nodes: HashMap<u64, GraphNode>,
    pub edges: HashMap<u64, GraphEdge>,
    pub outgoing: HashMap<u64, Vec<u64>>, // node_id -> Vec<edge_id>
    pub incoming: HashMap<u64, Vec<u64>>, // node_id -> Vec<edge_id>
    next_node_id: u64,
    next_edge_id: u64,
}

impl PropertyGraph {
    pub fn new(id: u64, name: String) -> Self {
        Self {
            id,
            name,
            nodes: HashMap::new(),
            edges: HashMap::new(),
            outgoing: HashMap::new(),
            incoming: HashMap::new(),
            next_node_id: 1,
            next_edge_id: 1,
        }
    }

    pub fn add_node(
        &mut self,
        label: &str,
        properties: HashMap<String, Value>,
        embedding: Option<Vec<f64>>,
    ) -> u64 {
        let node_id = self.next_node_id;
        self.next_node_id += 1;

        self.nodes.insert(
            node_id,
            GraphNode {
                id: node_id,
                label: label.to_string(),
                properties,
                embedding,
            },
        );

        self.outgoing.insert(node_id, Vec::new());
        self.incoming.insert(node_id, Vec::new());

        node_id
    }

    pub fn add_edge(
        &mut self,
        from: u64,
        to: u64,
        label: &str,
        weight: f64,
        properties: HashMap<String, Value>,
    ) -> Result<u64, String> {
        if !self.nodes.contains_key(&from) {
            return Err(format!("AetherGraph: Node {} does not exist", from));
        }
        if !self.nodes.contains_key(&to) {
            return Err(format!("AetherGraph: Node {} does not exist", to));
        }

        let edge_id = self.next_edge_id;
        self.next_edge_id += 1;

        self.edges.insert(
            edge_id,
            GraphEdge {
                id: edge_id,
                from,
                to,
                label: label.to_string(),
                weight: if weight <= 0.0 { 1.0 } else { weight },
                properties,
            },
        );

        self.outgoing.entry(from).or_default().push(edge_id);
        self.incoming.entry(to).or_default().push(edge_id);

        Ok(edge_id)
    }

    pub fn neighbors(&self, node_id: u64, direction: &str) -> Vec<u64> {
        let mut neighbors = Vec::new();

        if direction == "outgoing" || direction == "both" {
            if let Some(edge_ids) = self.outgoing.get(&node_id) {
                for eid in edge_ids {
                    if let Some(edge) = self.edges.get(eid) {
                        neighbors.push(edge.to);
                    }
                }
            }
        }

        if direction == "incoming" || direction == "both" {
            if let Some(edge_ids) = self.incoming.get(&node_id) {
                for eid in edge_ids {
                    if let Some(edge) = self.edges.get(eid) {
                        neighbors.push(edge.from);
                    }
                }
            }
        }

        neighbors
    }

    /// Breadth-First Search (BFS) traversal starting from a node
    pub fn bfs(&self, start_id: u64, max_depth: usize) -> Vec<u64> {
        if !self.nodes.contains_key(&start_id) {
            return Vec::new();
        }

        let mut visited = HashSet::new();
        let mut order = Vec::new();
        let mut queue = VecDeque::new();

        queue.push_back((start_id, 0));
        visited.insert(start_id);

        while let Some((curr, depth)) = queue.pop_front() {
            order.push(curr);
            if depth >= max_depth {
                continue;
            }

            for neighbor in self.neighbors(curr, "outgoing") {
                if !visited.contains(&neighbor) {
                    visited.insert(neighbor);
                    queue.push_back((neighbor, depth + 1));
                }
            }
        }

        order
    }

    /// Depth-First Search (DFS) traversal starting from a node
    pub fn dfs(&self, start_id: u64, max_depth: usize) -> Vec<u64> {
        if !self.nodes.contains_key(&start_id) {
            return Vec::new();
        }

        let mut visited = HashSet::new();
        let mut order = Vec::new();
        let mut stack = Vec::new();

        stack.push((start_id, 0));

        while let Some((curr, depth)) = stack.pop() {
            if visited.contains(&curr) {
                continue;
            }
            visited.insert(curr);
            order.push(curr);

            if depth < max_depth {
                let mut nbs = self.neighbors(curr, "outgoing");
                nbs.reverse(); // Maintain forward order
                for neighbor in nbs {
                    if !visited.contains(&neighbor) {
                        stack.push((neighbor, depth + 1));
                    }
                }
            }
        }

        order
    }

    /// Dijkstra's algorithm for shortest weighted path
    pub fn dijkstra_shortest_path(&self, start_id: u64, target_id: u64) -> Option<(Vec<u64>, f64)> {
        if !self.nodes.contains_key(&start_id) || !self.nodes.contains_key(&target_id) {
            return None;
        }

        #[derive(Copy, Clone, PartialEq)]
        struct State {
            cost: f64,
            node: u64,
        }

        impl Eq for State {}

        impl Ord for State {
            fn cmp(&self, other: &Self) -> Ordering {
                other.cost.partial_cmp(&self.cost).unwrap_or(Ordering::Equal)
            }
        }

        impl PartialOrd for State {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        let mut dist: HashMap<u64, f64> = HashMap::new();
        let mut prev: HashMap<u64, u64> = HashMap::new();
        let mut heap = BinaryHeap::new();

        dist.insert(start_id, 0.0);
        heap.push(State { cost: 0.0, node: start_id });

        while let Some(State { cost, node }) = heap.pop() {
            if node == target_id {
                let mut path = Vec::new();
                let mut curr = target_id;
                while let Some(&p) = prev.get(&curr) {
                    path.push(curr);
                    curr = p;
                }
                path.push(start_id);
                path.reverse();
                return Some((path, cost));
            }

            if cost > *dist.get(&node).unwrap_or(&f64::INFINITY) {
                continue;
            }

            if let Some(edge_ids) = self.outgoing.get(&node) {
                for &eid in edge_ids {
                    if let Some(edge) = self.edges.get(&eid) {
                        let next = edge.to;
                        let next_cost = cost + edge.weight;

                        if next_cost < *dist.get(&next).unwrap_or(&f64::INFINITY) {
                            dist.insert(next, next_cost);
                            prev.insert(next, node);
                            heap.push(State { cost: next_cost, node: next });
                        }
                    }
                }
            }
        }

        None
    }

    /// PageRank algorithm for node authority centrality
    pub fn pagerank(&self, iterations: usize, damping: f64) -> HashMap<u64, f64> {
        let n = self.nodes.len();
        if n == 0 {
            return HashMap::new();
        }

        let init_rank = 1.0 / n as f64;
        let mut ranks: HashMap<u64, f64> = self.nodes.keys().map(|&id| (id, init_rank)).collect();

        for _ in 0..iterations {
            let mut next_ranks: HashMap<u64, f64> = HashMap::new();
            let base_rank = (1.0 - damping) / n as f64;

            for &node_id in self.nodes.keys() {
                next_ranks.insert(node_id, base_rank);
            }

            for (&node_id, &curr_rank) in &ranks {
                let outgoing_edges = self.outgoing.get(&node_id).map(|v| v.len()).unwrap_or(0);
                if outgoing_edges > 0 {
                    let share = (damping * curr_rank) / outgoing_edges as f64;
                    for &eid in self.outgoing.get(&node_id).unwrap() {
                        if let Some(edge) = self.edges.get(&eid) {
                            *next_ranks.entry(edge.to).or_insert(0.0) += share;
                        }
                    }
                } else {
                    let share = (damping * curr_rank) / n as f64;
                    for next_id in self.nodes.keys() {
                        *next_ranks.entry(*next_id).or_insert(0.0) += share;
                    }
                }
            }

            ranks = next_ranks;
        }

        ranks
    }

    /// Semantic hybrid AI vector search over graph node embeddings
    pub fn semantic_search(&self, query_vec: &[f64], top_k: usize) -> Vec<(u64, f64)> {
        let mut scored = Vec::new();

        for node in self.nodes.values() {
            if let Some(emb) = &node.embedding {
                let sim = cosine_similarity(query_vec, emb);
                scored.push((node.id, sim));
            }
        }

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        if top_k > 0 && top_k < scored.len() {
            scored.truncate(top_k);
        }

        scored
    }
}

fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    let min_len = a.len().min(b.len());
    if min_len == 0 {
        return 0.0;
    }

    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    for i in 0..min_len {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom == 0.0 {
        0.0
    } else {
        dot / denom
    }
}

// ==============================================================================
// 2. Global Graph Registry
// ==============================================================================

static GRAPH_REGISTRY: OnceLock<Mutex<HashMap<u64, Arc<Mutex<PropertyGraph>>>>> = OnceLock::new();
static NEXT_GRAPH_ID: AtomicU64 = AtomicU64::new(1);

fn get_registry() -> &'static Mutex<HashMap<u64, Arc<Mutex<PropertyGraph>>>> {
    GRAPH_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn create_graph(name: &str) -> u64 {
    let id = NEXT_GRAPH_ID.fetch_add(1, AtomicOrdering::SeqCst);
    let graph = PropertyGraph::new(id, name.to_string());
    get_registry().lock().unwrap().insert(id, Arc::new(Mutex::new(graph)));
    id
}

pub fn get_graph(id: u64) -> Result<Arc<Mutex<PropertyGraph>>, String> {
    get_registry()
        .lock()
        .unwrap()
        .get(&id)
        .cloned()
        .ok_or_else(|| format!("AetherGraph: Graph {} not found", id))
}

// ==============================================================================
// 3. Native VM Module Bindings
// ==============================================================================

pub fn register_graph_module(globals: &mut HashMap<String, Value>) {
    let mut mod_map = HashMap::new();

    // 1. Graph.create(name) -> graph_id
    mod_map.insert(
        "create".to_string(),
        Value::Native("Graph.create".into(), |args| {
            let name = if !args.is_empty() { args[0].to_string() } else { "graph".to_string() };
            let id = create_graph(&name);
            Ok(Value::Int(id as i64))
        }),
    );

    // 2. Graph.add_node(graph_id, label, props_opt, embedding_opt) -> node_id
    mod_map.insert(
        "add_node".to_string(),
        Value::Native("Graph.add_node".into(), |args| {
            if args.len() < 2 {
                return Err("Graph.add_node(graph_id, label, [props], [embedding]) requires at least 2 arguments".into());
            }
            let gid = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("graph_id must be integer".into()),
            };
            let label = args[1].to_string();

            let props = if args.len() > 2 {
                match &args[2] {
                    Value::Map(m) => m.lock().clone(),
                    _ => HashMap::new(),
                }
            } else {
                HashMap::new()
            };

            let embedding = if args.len() > 3 {
                match &args[3] {
                    Value::Array(a) => {
                        let nums: Vec<f64> = a.lock().iter().filter_map(|v| match v {
                            Value::Float(f) => Some(*f),
                            Value::Int(i) => Some(*i as f64),
                            _ => None,
                        }).collect();
                        Some(nums)
                    }
                    _ => None,
                }
            } else {
                None
            };

            let g_arc = get_graph(gid)?;
            let mut g = g_arc.lock().unwrap();
            let nid = g.add_node(&label, props, embedding);
            Ok(Value::Int(nid as i64))
        }),
    );

    // 3. Graph.add_edge(graph_id, from, to, label, weight_opt, props_opt) -> edge_id
    mod_map.insert(
        "add_edge".to_string(),
        Value::Native("Graph.add_edge".into(), |args| {
            if args.len() < 4 {
                return Err("Graph.add_edge(graph_id, from, to, label, [weight], [props]) requires at least 4 arguments".into());
            }
            let gid = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("graph_id must be integer".into()),
            };
            let from = match args[1] {
                Value::Int(i) => i as u64,
                _ => return Err("from must be integer".into()),
            };
            let to = match args[2] {
                Value::Int(i) => i as u64,
                _ => return Err("to must be integer".into()),
            };
            let label = args[3].to_string();

            let weight = if args.len() > 4 {
                match args[4] {
                    Value::Float(f) => f,
                    Value::Int(i) => i as f64,
                    _ => 1.0,
                }
            } else {
                1.0
            };

            let props = if args.len() > 5 {
                match &args[5] {
                    Value::Map(m) => m.lock().clone(),
                    _ => HashMap::new(),
                }
            } else {
                HashMap::new()
            };

            let g_arc = get_graph(gid)?;
            let mut g = g_arc.lock().unwrap();
            let eid = g.add_edge(from, to, &label, weight, props)?;
            Ok(Value::Int(eid as i64))
        }),
    );

    // 4. Graph.shortest_path(graph_id, from, to) -> Map { "path": [...], "cost": ... }
    mod_map.insert(
        "shortest_path".to_string(),
        Value::Native("Graph.shortest_path".into(), |args| {
            if args.len() < 3 {
                return Err("Graph.shortest_path(graph_id, from, to) requires 3 arguments".into());
            }
            let gid = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("graph_id must be integer".into()),
            };
            let from = match args[1] {
                Value::Int(i) => i as u64,
                _ => return Err("from must be integer".into()),
            };
            let to = match args[2] {
                Value::Int(i) => i as u64,
                _ => return Err("to must be integer".into()),
            };

            let g_arc = get_graph(gid)?;
            let g = g_arc.lock().unwrap();
            let res = g.dijkstra_shortest_path(from, to);

            if let Some((path, cost)) = res {
                let mut map = HashMap::new();
                let path_vals = path.into_iter().map(|id| Value::Int(id as i64)).collect();
                map.insert("path".to_string(), Value::array(path_vals));
                map.insert("cost".to_string(), Value::Float(cost));
                Ok(Value::map(map))
            } else {
                Ok(Value::Nil)
            }
        }),
    );

    // 5. Graph.pagerank(graph_id, iterations, damping) -> Map { node_id: rank }
    mod_map.insert(
        "pagerank".to_string(),
        Value::Native("Graph.pagerank".into(), |args| {
            if args.is_empty() { return Err("Graph.pagerank(graph_id) requires graph_id".into()); }
            let gid = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("graph_id must be integer".into()),
            };
            let iters = if args.len() > 1 {
                match args[1] { Value::Int(i) => i as usize, _ => 20 }
            } else { 20 };

            let damping = if args.len() > 2 {
                match args[2] { Value::Float(f) => f, Value::Int(i) => i as f64, _ => 0.85 }
            } else { 0.85 };

            let g_arc = get_graph(gid)?;
            let g = g_arc.lock().unwrap();
            let ranks = g.pagerank(iters, damping);

            let mut map = HashMap::new();
            for (nid, r) in ranks {
                map.insert(nid.to_string(), Value::Float(r));
            }
            Ok(Value::map(map))
        }),
    );

    // 6. Graph.bfs(graph_id, start_id, depth) -> Array of node IDs
    mod_map.insert(
        "bfs".to_string(),
        Value::Native("Graph.bfs".into(), |args| {
            if args.len() < 2 { return Err("Graph.bfs(graph_id, start_id, [depth]) requires at least 2 arguments".into()); }
            let gid = match args[0] { Value::Int(i) => i as u64, _ => return Err("graph_id must be integer".into()) };
            let start = match args[1] { Value::Int(i) => i as u64, _ => return Err("start_id must be integer".into()) };
            let depth = if args.len() > 2 { match args[2] { Value::Int(i) => i as usize, _ => 10 } } else { 10 };

            let g_arc = get_graph(gid)?;
            let g = g_arc.lock().unwrap();
            let order = g.bfs(start, depth);
            let vals = order.into_iter().map(|id| Value::Int(id as i64)).collect();
            Ok(Value::array(vals))
        }),
    );

    // 7. Graph.dfs(graph_id, start_id, depth) -> Array of node IDs
    mod_map.insert(
        "dfs".to_string(),
        Value::Native("Graph.dfs".into(), |args| {
            if args.len() < 2 { return Err("Graph.dfs(graph_id, start_id, [depth]) requires at least 2 arguments".into()); }
            let gid = match args[0] { Value::Int(i) => i as u64, _ => return Err("graph_id must be integer".into()) };
            let start = match args[1] { Value::Int(i) => i as u64, _ => return Err("start_id must be integer".into()) };
            let depth = if args.len() > 2 { match args[2] { Value::Int(i) => i as usize, _ => 10 } } else { 10 };

            let g_arc = get_graph(gid)?;
            let g = g_arc.lock().unwrap();
            let order = g.dfs(start, depth);
            let vals = order.into_iter().map(|id| Value::Int(id as i64)).collect();
            Ok(Value::array(vals))
        }),
    );

    // 8. Graph.semantic_search(graph_id, query_vec, top_k) -> Array of maps
    mod_map.insert(
        "semantic_search".to_string(),
        Value::Native("Graph.semantic_search".into(), |args| {
            if args.len() < 2 { return Err("Graph.semantic_search(graph_id, query_vec, [top_k]) requires at least 2 arguments".into()); }
            let gid = match args[0] { Value::Int(i) => i as u64, _ => return Err("graph_id must be integer".into()) };

            let query_vec: Vec<f64> = match &args[1] {
                Value::Array(a) => {
                    a.lock().iter().filter_map(|v| match v {
                        Value::Float(f) => Some(*f),
                        Value::Int(i) => Some(*i as f64),
                        _ => None,
                    }).collect()
                }
                _ => return Err("query_vec must be an array of numbers".into()),
            };

            let top_k = if args.len() > 2 { match args[2] { Value::Int(i) => i as usize, _ => 5 } } else { 5 };

            let g_arc = get_graph(gid)?;
            let g = g_arc.lock().unwrap();
            let results = g.semantic_search(&query_vec, top_k);

            let res_vals: Vec<Value> = results.into_iter().map(|(nid, score)| {
                let mut map = HashMap::new();
                map.insert("node_id".to_string(), Value::Int(nid as i64));
                map.insert("score".to_string(), Value::Float(score));
                if let Some(node) = g.nodes.get(&nid) {
                    map.insert("label".to_string(), Value::string(node.label.clone()));
                }
                Value::map(map)
            }).collect();

            Ok(Value::array(res_vals))
        }),
    );

    let val = Value::map(mod_map);
    globals.insert("__native_graph".to_string(), val.clone());
    globals.insert("graph".to_string(), val.clone());
    globals.insert("aether_graph".to_string(), val.clone());
    globals.insert("Graph".to_string(), val);
}
