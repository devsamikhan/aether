use aether::vm::{run_source, Value};

#[test]
fn test_vector_similarity_metrics() {
    let code = r#"
from aether_vector import cosine_similarity, euclidean_distance, dot_product, normalize

# 1. Cosine similarity
# Identical vectors: cos = 1.0
cos_identical = cosine_similarity([1.0, 2.0, 3.0], [1.0, 2.0, 3.0])

# Orthogonal vectors: cos = 0.0
cos_ortho = cosine_similarity([1.0, 0.0], [0.0, 1.0])

# Opposite vectors: cos = -1.0
cos_opposite = cosine_similarity([1.0, 0.0], [-1.0, 0.0])

# 2. Euclidean distance: 3-4-5 triangle
dist = euclidean_distance([0.0, 0.0], [3.0, 4.0])

# 3. Dot product: 2*4 + 3*5 = 8 + 15 = 23
dot = dot_product([2.0, 3.0], [4.0, 5.0])

# 4. Normalization: [3.0, 4.0] / 5.0 = [0.6, 0.8]
norm = normalize([3.0, 4.0])

[
    Math.round(cos_identical, 4),
    Math.round(cos_ortho, 4),
    Math.round(cos_opposite, 4),
    Math.round(dist, 4),
    Math.round(dot, 4),
    Math.round(norm[0], 2),
    Math.round(norm[1], 2)
]
"#;
    let res = run_source(code).expect("Vector similarity metrics test failed");
    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        assert_eq!(items[0].to_string(), "1");
        assert_eq!(items[1].to_string(), "0");
        assert_eq!(items[2].to_string(), "-1");
        assert_eq!(items[3].to_string(), "5");
        assert_eq!(items[4].to_string(), "23");
        assert_eq!(items[5].to_string(), "0.6");
        assert_eq!(items[6].to_string(), "0.8");
    } else {
        panic!("Expected Array, got {:?}", res);
    }
}

#[test]
fn test_vector_store_top_k_search_and_metadata_filter() {
    let code = r#"
from aether_vector import VectorStore

store = VectorStore("cosine")

# Index 4 documents with 3-dimensional embeddings
store.add("doc_rust", [0.9, 0.2, 0.1], {"lang": "rust", "topic": "systems"})
store.add("doc_python", [0.2, 0.9, 0.3], {"lang": "python", "topic": "ai"})
store.add("doc_aether", [0.85, 0.3, 0.2], {"lang": "aether", "topic": "systems"})
store.add("doc_deep_learning", [0.1, 0.85, 0.5], {"lang": "python", "topic": "ai"})

# Query close to systems/low-level: [1.0, 0.0, 0.0]
results_all = store.search([1.0, 0.0, 0.0], 2)

# Query with metadata filter: only topic == "ai"
results_filtered = store.search([1.0, 0.0, 0.0], 2, "cosine", "topic", "ai")

[
    store.count(),
    results_all[0]["id"],
    results_all[1]["id"],
    results_filtered[0]["id"]
]
"#;
    let res = run_source(code).expect("Vector store search test failed");
    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        assert_eq!(items[0].to_string(), "4"); // total count
        assert_eq!(items[1].to_string(), "doc_rust"); // closest to [1, 0, 0]
        assert_eq!(items[2].to_string(), "doc_aether"); // second closest
        assert_eq!(items[3].to_string(), "doc_python"); // best among topic=="ai"
    } else {
        panic!("Expected Array, got {:?}", res);
    }
}

#[test]
fn test_agent_tool_registration_and_execution() {
    let code = r#"
from aether_agent import Tool, Agent

fn add_numbers(args):
    return args[0] + args[1]

fn multiply_numbers(args):
    return args[0] * args[1]

t1 = Tool("add", "Adds two numbers", add_numbers)
t2 = Tool("mul", "Multiplies two numbers", multiply_numbers)

agent = Agent("MathAgent", "Specialized in calculations", [t1, t2])

r1 = agent.get_tool("add").execute([15, 27])
r2 = agent.get_tool("mul").execute([6, 7])

[r1, r2, len(agent.tools.keys())]
"#;
    let res = run_source(code).expect("Agent tool test failed");
    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        assert_eq!(items[0].to_string(), "42");
        assert_eq!(items[1].to_string(), "42");
        assert_eq!(items[2].to_string(), "2");
    } else {
        panic!("Expected Array, got {:?}", res);
    }
}

#[test]
fn test_autonomous_agent_reasoning_and_memory_retrieval() {
    let code = r#"
from aether_agent import Tool, Agent, AgentMemory

memory = AgentMemory()

# Store historical knowledge in long-term vector memory
memory.remember("fact_capital", "Islamabad is the capital of Pakistan", [0.95, 0.1, 0.0], {"domain": "geography"})
memory.remember("fact_quantum", "Quantum qubits utilize superposition", [0.05, 0.95, 0.2], {"domain": "physics"})

fn search_memory_tool(query):
    # Query vector close to geography
    let hits = memory.recall([0.9, 0.05, 0.0], 1)
    if len(hits) > 0:
        return hits[0]["metadata"]["text"]
    return "No memory found"

t_memory = Tool("recall_memory", "Searches long-term semantic memory", search_memory_tool)

agent = Agent("ResearcherAgent", "Autonomous research assistant", [t_memory], memory)

# Run autonomous agent with goal
res = agent.run("What is the capital of Pakistan?")

[
    res["status"],
    res["steps"],
    res["result"],
    len(agent.memory.get_history())
]
"#;
    let res = run_source(code).expect("Autonomous agent test failed");
    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        assert_eq!(items[0].to_string(), "completed");
        assert_eq!(items[1].to_string(), "2");
        assert!(items[2].to_string().contains("Islamabad is the capital of Pakistan"));
        assert_eq!(items[3].to_string(), "3"); // 1 user + 1 tool + 1 assistant
    } else {
        panic!("Expected Array, got {:?}", res);
    }
}
