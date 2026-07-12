use super::gcounter::GCounter;

/// A Positive-Negative Counter (PNCounter).
/// State-based CRDT that supports increments and decrements.
#[derive(Debug, Clone, Eq)]
pub struct PNCounter {
    pub positive: GCounter,
    pub negative: GCounter,
}

impl PNCounter {
    pub fn new(node_id: &str) -> Self {
        Self {
            positive: GCounter::new(node_id),
            negative: GCounter::new(node_id),
        }
    }

    /// Increment counter.
    pub fn increment(&mut self) {
        self.positive.increment();
    }

    /// Decrement counter.
    pub fn decrement(&mut self) {
        self.negative.increment();
    }

    /// Returns the active value (positive value minus negative value).
    pub fn value(&self) -> i64 {
        (self.positive.value() as i64) - (self.negative.value() as i64)
    }

    /// Merge state with another PNCounter.
    pub fn merge(&mut self, other: &Self) {
        self.positive.merge(&other.positive);
        self.negative.merge(&other.negative);
    }
}

impl PartialEq for PNCounter {
    fn eq(&self, other: &Self) -> bool {
        self.positive == other.positive && self.negative == other.negative
    }
}
