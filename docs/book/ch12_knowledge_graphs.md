# Chapter 12: AetherGraph: In-Memory Property Graphs & Knowledge Traversal

AETHER includes a native in-memory graph database engine for artificial intelligence and knowledge graphs.

### 12.1 Building a Knowledge Graph & Dijkstra Pathfinding

```aether
from aether_graph import Graph

let kg = Graph("AI_Reasoning_Engine")

let n_query = kg.add_node("User_Query", {"text": "Compute optimal path"})
let n_rag   = kg.add_node("Knowledge_Base", {"entries": 50000})
let n_model = kg.add_node("Reasoning_LLM", {"params": "70B"})
let n_exec  = kg.add_node("Action_Executor", {"sandbox": true})

# Directed weighted edges
kg.add_edge(n_query, n_rag, "retrieves", 1.2)
kg.add_edge(n_rag, n_model, "contextualizes", 0.8)
kg.add_edge(n_model, n_exec, "dispatches", 1.5)
kg.add_edge(n_query, n_exec, "direct_bypass", 5.0)

# Find optimal shortest path via Dijkstra's algorithm
let route = kg.shortest_path(n_query, n_exec)
println("Optimal Traversal Node Path:", route["path"])
println("Total Cost:", route["cost"]) # 3.5

# Calculate PageRank authority across all nodes
let ranks = kg.pagerank(25, 0.85)
println("PageRank Distribution:", ranks)
```

---
