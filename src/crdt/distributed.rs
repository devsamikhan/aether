use std::collections::HashMap;

pub trait Mergeable {
    fn merge(&mut self, other: &Self);
}

/// A simulated distributed system containing nodes running a mergeable CRDT.
pub struct DistributedSystem<C> {
    pub nodes: HashMap<String, C>,
}

impl<C: Clone + PartialEq + Mergeable> DistributedSystem<C> {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, id: &str, node: C) {
        self.nodes.insert(id.to_string(), node);
    }

    pub fn update<F>(&mut self, id: &str, f: F)
    where
        F: FnOnce(&mut C),
    {
        if let Some(node) = self.nodes.get_mut(id) {
            f(node);
        }
    }

    /// Synchronize the state of `from` node to `to` node.
    pub fn sync(&mut self, from: &str, to: &str) {
        if let Some(from_node) = self.nodes.get(from).cloned() {
            if let Some(to_node) = self.nodes.get_mut(to) {
                to_node.merge(&from_node);
            }
        }
    }

    /// Check if all nodes in the system have converged to identical state representation.
    pub fn verify_convergence(&self) -> bool {
        if self.nodes.is_empty() {
            return true;
        }
        let mut iter = self.nodes.values();
        let first = iter.next().unwrap();
        for other in iter {
            if first != other {
                return false;
            }
        }
        true
    }
}
