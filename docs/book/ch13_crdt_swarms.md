# Chapter 13: Distributed Swarms & Conflict-Free Replicated Data Types (CRDTs)

AETHER solves distributed consensus through mathematical **join-semilattices** (CRDTs), guaranteeing eventual consistency across network partitions without distributed locks.

### 13.1 GCounter (Grow-Only Counter)

```aether
import crdt

let cluster_node_1 = GCounter.new("datacenter_us_east")
let cluster_node_2 = GCounter.new("datacenter_eu_west")

# Asynchronous operations across datacenters
cluster_node_1.increment(25)
cluster_node_2.increment(40)

# Autonomous state merge across network partition
cluster_node_1.merge(cluster_node_2)
println("Converged Cluster State:", cluster_node_1.read()) # 65
```

---
