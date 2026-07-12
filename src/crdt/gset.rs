use std::collections::HashSet;
use std::hash::Hash;

/// A Grow-only Set (GSet).
/// State-based CRDT where items can only be added.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GSet<T: Hash + Eq> {
    pub elements: HashSet<T>,
}

impl<T: Hash + Eq + Clone> GSet<T> {
    pub fn new() -> Self {
        Self {
            elements: HashSet::new(),
        }
    }

    /// Add an item to the set.
    pub fn add(&mut self, item: T) {
        self.elements.insert(item);
    }

    /// Check if item exists in the set.
    pub fn contains(&self, item: &T) -> bool {
        self.elements.contains(item)
    }

    /// Merge state with another GSet (Set Union).
    pub fn merge(&mut self, other: &Self) {
        for item in &other.elements {
            self.elements.insert(item.clone());
        }
    }
}
