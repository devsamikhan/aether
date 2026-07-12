use std::collections::HashMap;

/// A Grow-only Counter (GCounter).
/// State-based CRDT where value only increments.
#[derive(Debug, Clone, Eq)]
pub struct GCounter {
    pub node_id: String,
    pub counts: HashMap<String, u64>,
}

impl GCounter {
    pub fn new(node_id: &str) -> Self {
        Self {
            node_id: node_id.to_string(),
            counts: HashMap::new(),
        }
    }

    /// Increment the local node's counter.
    pub fn increment(&mut self) {
        let entry = self.counts.entry(self.node_id.clone()).or_insert(0);
        *entry += 1;
    }

    /// Increment the local node's counter by a specific value.
    pub fn increment_by(&mut self, val: u64) {
        let entry = self.counts.entry(self.node_id.clone()).or_insert(0);
        *entry += val;
    }

    /// Returns the aggregated value across all nodes.
    pub fn value(&self) -> u64 {
        self.counts.values().sum()
    }

    /// Merge state with another GCounter.
    /// LUB (Least Upper Bound) is obtained by taking the element-wise maximum.
    pub fn merge(&mut self, other: &Self) {
        for (node, count) in &other.counts {
            let entry = self.counts.entry(node.clone()).or_insert(0);
            *entry = std::cmp::max(*entry, *count);
        }
    }
}

impl PartialEq for GCounter {
    fn eq(&self, other: &Self) -> bool {
        // Compare only semantic counts, ignoring node_id metadata
        self.counts == other.counts
    }
}
