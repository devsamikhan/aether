use std::collections::{HashMap, HashSet};
use std::hash::Hash;

/// An Observed-Remove Set (ORSet).
/// State-based CRDT that supports adds and removes.
#[derive(Debug, Clone, Eq)]
pub struct ORSet<T: Hash + Eq> {
    pub node_id: String,
    pub seq: u64,
    pub elements: HashMap<T, HashSet<String>>,
    pub tombstones: HashSet<String>,
}

impl<T: Hash + Eq + Clone> ORSet<T> {
    pub fn new(node_id: &str) -> Self {
        Self {
            node_id: node_id.to_string(),
            seq: 0,
            elements: HashMap::new(),
            tombstones: HashSet::new(),
        }
    }

    /// Add an item to the set with a new unique tag.
    pub fn add(&mut self, item: T) {
        self.seq += 1;
        let tag = format!("{}-{}", self.node_id, self.seq);
        self.elements.entry(item).or_insert_with(HashSet::new).insert(tag);
    }

    /// Remove an item by moving all of its active tags to the tombstones.
    pub fn remove(&mut self, item: &T) {
        if let Some(tags) = self.elements.get(item) {
            for tag in tags {
                self.tombstones.insert(tag.clone());
            }
        }
    }

    /// Check if item is active (has active tags not tombstoned).
    pub fn contains(&self, item: &T) -> bool {
        if let Some(tags) = self.elements.get(item) {
            // Must have at least one tag not present in tombstones
            tags.iter().any(|tag| !self.tombstones.contains(tag))
        } else {
            false
        }
    }

    /// Return all active elements in the set.
    pub fn read(&self) -> HashSet<T> {
        self.elements
            .iter()
            .filter(|(_item, tags)| tags.iter().any(|tag| !self.tombstones.contains(tag)))
            .map(|(item, _)| item.clone())
            .collect()
    }

    /// Merge state with another ORSet.
    pub fn merge(&mut self, other: &Self) {
        // Union tombstones
        for tag in &other.tombstones {
            self.tombstones.insert(tag.clone());
        }

        // Union elements and tags
        for (item, tags) in &other.elements {
            let entry = self.elements.entry(item.clone()).or_insert_with(HashSet::new);
            for tag in tags {
                entry.insert(tag.clone());
            }
        }
    }
}

impl<T: Hash + Eq> PartialEq for ORSet<T> {
    fn eq(&self, other: &Self) -> bool {
        // Compare elements and tombstones, ignoring node_id and local seq sequence counter
        self.elements == other.elements && self.tombstones == other.tombstones
    }
}
