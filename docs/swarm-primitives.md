# AETHER Swarm Intelligence Primitives

> **Status:** Phase 1 — Implemented (classical simulation, single host)  
> **Roadmap:** Distributed multi-host swarm execution and hardware-level stigmergic memory are Phase 2 items.

---

## Overview

AETHER's swarm intelligence primitives model **distributed autonomous agent computing** inspired by biological swarm systems: ant colonies, honeybee swarms, bird flocking (murmuration), and fish schooling. These natural systems achieve remarkable collective intelligence without centralized control — each individual agent follows simple local rules, and global-optimal behavior emerges from their interactions.

AETHER brings this paradigm into software with a formal, compiler-enforced agent model:

- **Agents** are lightweight autonomous execution contexts (goroutine-scale, not thread-scale)
- **State** is shared via a **CRDT (Conflict-free Replicated Data Type) state graph** — agents can read and write shared state without locks or coordination, and the runtime guarantees eventual consistency
- **Communication** is primarily indirect via **stigmergy** (pheromone-like markers on the shared state graph) rather than direct message passing
- **Consensus** is achieved via the `consensus` primitive using PBFT-derived algorithms for Byzantine fault tolerance

```
Swarm topology (Phase 1):

  ┌──────────────────────────────────────────────────────┐
  │               AETHER Runtime (JIT)                   │
  │                                                      │
  │  ┌─────────┐  ┌─────────┐  ┌─────────┐             │
  │  │ Agent 0 │  │ Agent 1 │  │ Agent 2 │  ...        │
  │  └────┬────┘  └────┬────┘  └────┬────┘             │
  │       │            │            │                    │
  │  ─────┴────────────┴────────────┴─────              │
  │            CRDT State Graph (shared)                 │
  └──────────────────────────────────────────────────────┘
```

### Design Principles

| Principle         | Description                                                        |
|-------------------|--------------------------------------------------------------------|
| **Decentralization** | No single agent is a coordinator. Leadership emerges via queen/drone election. |
| **Redundancy**    | Multiple agents pursue the same goal; the first to succeed wins.  |
| **Stigmergy**     | Agents communicate by modifying shared state, not direct messaging.|
| **Fault tolerance**| Agent death is normal. The swarm continues without any single agent. |
| **Emergence**     | Macro-level optimal behavior arises from micro-level local rules.  |

---

## `swarm_spawn`

### Concept

`swarm_spawn` **instantiates N autonomous agents** within the current execution context. Each agent is assigned a unique ID (`agent_id ∈ [0, N)`), a local stack, and a reference to the shared CRDT state graph. Agents begin executing their assigned task function concurrently.

The JIT schedules agents on available CPU threads using a work-stealing scheduler. On a single core, agents are interleaved cooperatively at `yield` points.

### Syntax

```aether
swarm_spawn(N) { agent_task };

// With agent configuration
swarm_spawn(N, role: worker) {
    // agent body — runs in each of the N agents
    let my_id = agent_id();
    // ...
};
```

### JIT Output

```
[AETHER::Swarm] SPAWN  count=8  role=worker  scheduler=work_steal
[AETHER::Swarm] AGENT  id=0  stack=64KB  crdt_ref=0x7f3a2c
[AETHER::Swarm] AGENT  id=1  stack=64KB  crdt_ref=0x7f3a2c
...
[AETHER::Swarm] ALIVE  agents=8  state=RUNNING
```

### Examples

**Example 1 — Spawn 8 worker agents:**
```aether
swarm_spawn(8) {
    let id = agent_id();
    emit("Agent ${id} is alive");
};
```

**Example 2 — Divide work by agent ID:**
```aether
let data = load_large_dataset();   // 10,000 records
int total_agents = 16;

swarm_spawn(total_agents) {
    int id    = agent_id();
    int chunk = data.length / total_agents;
    int start = id * chunk;
    int end   = start + chunk;
    
    let partial = process_slice(data, start, end);
    crdt_write("result_${id}", partial);
    
    // Signal completion
    pheromone("done", intensity: 1.0, agent: id);
};

// Wait for all agents to mark done
stigmergy("done", threshold: total_agents) => final_signal;
```

**Example 3 — Adaptive spawn size (spawn based on workload):**
```aether
int work_units = measure_workload();
int agents     = clamp(work_units / 100, min: 2, max: 256);

swarm_spawn(agents) {
    forage("work_queue") => task;
    let result = execute(task);
    crdt_write("results", result);
};
```

---

## `hive_mind`

### Concept

`hive_mind` **aggregates all active agents** into a collective intelligence block. Inside a `hive_mind` block, all agents synchronize their local state into the CRDT graph and participate in a **consensus round**. The block body runs as a single, unified computation on the emergent collective state — as if all agents were one mind.

This is the mechanism for **collective decision making**: individual agents gather data independently, then converge into `hive_mind` to combine their knowledge and output a unified answer.

Biologically, this models the **waggle dance** of honeybees: scouts return to the hive and collectively signal the best food source through vibration, eventually reaching quorum agreement.

### Syntax

```aether
hive_mind {
    // All agents have synchronized; collective state is available
    let aggregate = hive_collect("key");
    // Make a collective decision
    let decision = hive_vote(aggregate);
    hive_broadcast(decision);
};
```

### Examples

**Example 1 — Collective aggregation:**
```aether
swarm_spawn(16) {
    let local_result = search_local_space(agent_id());
    crdt_write("partial_${agent_id()}", local_result);
};

hive_mind {
    // All 16 agents' partial results are now available
    let all_results = hive_collect("partial_*");
    let best = all_results.min_by(r => r.cost);
    hive_broadcast("global_best", best);
};
```

**Example 2 — Majority vote:**
```aether
hive_mind {
    let votes = hive_collect("agent_vote");
    let consensus_value = hive_majority_vote(votes);
    emit("Collective decision: ${consensus_value}");
};
```

**Example 3 — Hive mind driving next swarm generation:**
```aether
repeat 10 {    // 10 generations of swarm evolution
    swarm_spawn(32) {
        forage("task_queue") => task;
        let fitness = evaluate(task);
        crdt_write("fitness_${agent_id()}", fitness);
    };
    
    hive_mind {
        let fitnesses = hive_collect("fitness_*");
        let next_gen  = evolve(fitnesses);       // Evolutionary selection
        crdt_write("task_queue", next_gen);
        hive_broadcast("generation_complete", true);
    };
}
```

---

## `von_neumann_replicate`

### Concept

`von_neumann_replicate` implements **self-replication** of an agent's computational state, inspired by John von Neumann's universal constructor theory. A replicating agent creates an exact functional copy of itself — same code, same local state — and spawns the copy as a new agent.

This is used for **adaptive swarm scaling**: when an agent encounters more work than it can handle alone, it replicates to create additional agents dynamically, without requiring central orchestration.

> [!WARNING]
> Unconstrained replication can exhaust system resources. Always pair `von_neumann_replicate` with a `colony` size bound or a resource governor.

### Syntax

```aether
von_neumann_replicate(target_agent);
von_neumann_replicate(self, max_copies: N);
```

### Examples

**Example 1 — Agent self-replication when overloaded:**
```aether
swarm_spawn(1) {
    repeat {
        let queue_depth = crdt_read("work_queue").length;
        if queue_depth > OVERLOAD_THRESHOLD {
            von_neumann_replicate(self, max_copies: 4);
        }
        forage("work_queue") => task;
        execute(task);
    }
};
```

**Example 2 — Replication for fault tolerance:**
```aether
// Replicate critical agents to ensure redundancy
fn make_redundant(agent_spec) {
    swarm_spawn(1) agent_spec;
    von_neumann_replicate(last_spawned(), max_copies: 3);
    // Now 4 identical agents run the same job; if any die, 3 remain
}
```

---

## Biological Role Primitives

AETHER maps the role hierarchy of social insect colonies to executable agent archetypes. Each role keyword defines a **behavioral policy** baked into the agent's scheduler.

### `pheromone`

Deposits a **stigmergic marker** on the CRDT state graph at a named location with an intensity value. Other agents can sense pheromone concentration via `forage` and `stigmergy`. Pheromone intensity decays over time at a configurable rate.

```aether
pheromone("food_trail", intensity: 1.0, decay: 0.95);
// JIT: [AETHER::Swarm] PHEROMONE key="food_trail"  strength=1.0  decay=0.95
```

---

### `scout`

Declares the current agent as a **scout agent**. Scouts have elevated exploration priority: they randomly sample the search space with high variance, depositing pheromones on promising regions. Worker agents follow scout pheromone trails.

```aether
scout {
    // Random exploration body
    let candidate = random_sample(search_space);
    let quality   = evaluate(candidate);
    pheromone("good_region", intensity: quality);
};
```

---

### `worker`

Declares the current agent as a **worker agent**. Workers exploit known-good regions by following the strongest pheromone gradient. They do the bulk of computation in a swarm.

```aether
worker {
    forage("good_region") => target;
    let result = execute_task(target);
    crdt_write("results", result);
};
```

---

### `queen`

Declares the current agent as the **queen agent** — a singleton coordinator responsible for spawning new agents, issuing broadcast directives, and managing colony lifecycle. There can only be one queen per colony. If the queen dies, a drone is automatically promoted via election.

```aether
queen {
    swarm_spawn(32, role: worker) { ... };
    repeat {
        let health = hive_collect("agent_health");
        if health.dead_count > 4 { swarm_spawn(health.dead_count, role: worker) { ... }; }
    }
};
```

---

### `drone`

A **drone agent** monitors colony health and is eligible for queen promotion. Drones maintain redundant metadata about the swarm state and run the queen-election protocol if the current queen's heartbeat is lost.

```aether
drone {
    repeat {
        let q_alive = ping("queen", timeout: 500ms);
        if !q_alive { elect_queen(); }
        sleep(1s);
    }
};
```

---

### `colony`

Declares a **bounded colony context** with a maximum agent count and a named CRDT namespace. All agents spawned inside a colony share the same namespace and are limited by the `max_agents` cap.

```aether
colony "search_swarm" max_agents: 128 {
    queen { ... };
    swarm_spawn(64, role: scout) { ... };
    swarm_spawn(64, role: worker) { ... };
};
```

---

### `forage`

Retrieves a work item from a named pheromone-gradient queue. Agents are automatically directed toward the highest-intensity pheromone region. If no pheromone is present, `forage` returns a random item (exploration fallback).

```aether
forage("task_queue") => task;
forage("good_region", bias: exploitation) => target;
forage("food_trail",  timeout: 2s) => food;
```

---

### `stigmergy`

Waits until the cumulative pheromone concentration at a named location exceeds a threshold. Used for **indirect synchronization**: agents signal completion by depositing pheromone, and `stigmergy` blocks until enough agents have signaled.

```aether
stigmergy("done", threshold: 16) => signal;
// Blocks until 16 agents have deposited "done" pheromone
emit("All 16 agents finished");
```

---

## Communication Primitives

### `consensus`

Runs a **PBFT-derived consensus round** across all live agents. Each agent proposes a value; the round guarantees agreement on a single value even if up to `f` agents are Byzantine (malicious or faulty), where `f < N/3`.

```aether
consensus("block_hash", value: my_block_hash, fault_tolerance: 1/3) => agreed_hash;
emit("Consensus reached: ${agreed_hash}");
```

---

### `broadcast`

Sends a message to **all agents** in the current colony simultaneously. Agents receive the broadcast on their next `recv` call. Delivery is best-effort (not guaranteed if an agent dies mid-delivery).

```aether
broadcast("shutdown", payload: "graceful");
broadcast("new_target", payload: best_candidate);
```

---

### `multicast`

Sends a message to a **specific group** of agents identified by role or ID list.

```aether
multicast(to: role(scout), message: "reset_exploration");
multicast(to: agents([0, 4, 7, 15]), message: "priority_task");
```

---

## Use Cases

### GroverSwarmSearch

Combines AETHER's quantum and swarm primitives. Each swarm agent maintains an independent qubit register and applies a partial Grover search over its assigned subspace. At a `hive_mind` barrier, agents exchange phase information via CRDT, achieving interference across the distributed quantum states — collapsing to the global solution.

```aether
int agents    = 8;
int qubits    = 4;           // Each agent searches 2^4 = 16 states
int total_space = agents * (2 ** qubits);  // 128 total states

colony "grover_swarm" max_agents: agents {
    swarm_spawn(agents) {
        // Each agent runs an independent Grover circuit
        qubit[qubits] reg;
        for i in 0..qubits { superpose(reg[i]); }
        
        // Local oracle marks the target value in this agent's subspace
        let my_subspace = agent_id() * (2 ** qubits);
        grover_oracle(reg) {
            if (reg + my_subspace) == global_target { phase_flip(reg); }
        };
        
        grover_diffuse(reg);
        measure(reg) => local_candidate;
        crdt_write("candidate_${agent_id()}", local_candidate + my_subspace);
        pheromone("found_candidate", intensity: grover_probability(reg));
    };
    
    hive_mind {
        let candidates = hive_collect("candidate_*");
        let best       = candidates.max_by(c => c.probability);
        emit("GroverSwarmSearch result: ${best.value}  P=${best.probability}");
    };
};
```

---

### ConsensusLedger

A Byzantine fault-tolerant distributed ledger. Each agent is a validator node. Blocks are proposed, broadcast, and accepted only after `consensus` guarantees 2f+1 matching votes.

```aether
colony "ledger" max_agents: 21 {
    swarm_spawn(21) {
        let block = build_block(crdt_read("mempool"));
        
        // Phase 1: Pre-prepare
        broadcast("pre_prepare", payload: block.hash);
        
        // Phase 2: Prepare — wait for 2f+1 matching pre-prepare messages
        stigmergy("pre_prepare_${block.hash}", threshold: 14) => _;
        broadcast("prepare", payload: block.hash);
        
        // Phase 3: Commit — wait for 2f+1 prepare messages
        stigmergy("prepare_${block.hash}", threshold: 14) => _;
        broadcast("commit", payload: block.hash);
        
        // Finalize
        stigmergy("commit_${block.hash}", threshold: 14) => _;
        crdt_write("ledger", block);
        emit("Block ${block.hash} committed. Height: ${ledger.height}");
    };
};
```

**Fault tolerance:** With 21 validators, tolerates up to 6 Byzantine (malicious) nodes (`f = 6`, `3f+1 = 19 ≤ 21`).

---

### ByzantineConsensus — Theoretical Proposal

A theoretical extension combining `entangle` (quantum primitives) with `consensus` (swarm primitives). Validator qubits are pre-entangled in a GHZ state before the consensus round. A Byzantine validator cannot provide a divergent vote without the entanglement being detected as a Bell inequality violation.

> [!NOTE]
> This is a theoretical research proposal. Quantum network infrastructure for distributing entangled qubit pairs between validator nodes is a Phase 3 roadmap item.

```aether
// Entangle all validator qubits (conceptual Phase 3 code)
qubit[21] validator_qubits;
// Prepare GHZ state: 1/√2(|000...0⟩ + |111...1⟩)
hadamard(validator_qubits[0]);
for i in 1..21 { cnot(validator_qubits[0], validator_qubits[i]); }

// Now run consensus — Byzantine votes produce detectable state violations
consensus("block_hash_quantum", value: my_hash, fault_tolerance: 1/3) => agreed;
```

---

## Performance Characteristics

### Swarm Scaling

| Metric              | Phase 1 (single host) | Phase 2 (distributed) |
|---------------------|----------------------|----------------------|
| Max agents          | ~10,000 (goroutine limit) | Unlimited (network nodes) |
| CRDT sync latency   | <1ms (shared memory) | 5–100ms (network RTT) |
| Pheromone decay     | Software timer (ms resolution) | Hardware timestamped |
| Fault recovery      | Agent restart (<1ms) | Node rejoin (~500ms) |

> **Note:** These metrics represent simulated complexity analysis and theoretical bounds, not runs on physical distributed hardware.

### Consensus Complexity

PBFT consensus has the following communication complexity per round:

```
Messages per round:  O(N²)  where N = number of validators
Rounds to finality:  3 (pre-prepare → prepare → commit)
Total messages:      O(3N²)
Byzantine tolerance: f < N/3
```

For large `N`, this limits PBFT to approximately 100–200 validators before network overhead dominates. AETHER Phase 2 will include a **HotStuff** consensus variant with O(N) communication complexity for larger validator sets.

> [!TIP]
> For applications requiring thousands of validators, use the `hotstuff_consensus` variant (planned Phase 2 primitive) which achieves O(N) message complexity with linear-threshold signatures.
