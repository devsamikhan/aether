pub mod gcounter;
pub mod gset;
pub mod pncounter;
pub mod orset;
pub mod distributed;

pub use gcounter::GCounter;
pub use gset::GSet;
pub use pncounter::PNCounter;
pub use orset::ORSet;
pub use distributed::{DistributedSystem, Mergeable};

impl Mergeable for GCounter {
    fn merge(&mut self, other: &Self) {
        self.merge(other);
    }
}

impl<T: std::hash::Hash + Eq + Clone> Mergeable for GSet<T> {
    fn merge(&mut self, other: &Self) {
        self.merge(other);
    }
}

impl Mergeable for PNCounter {
    fn merge(&mut self, other: &Self) {
        self.merge(other);
    }
}

impl<T: std::hash::Hash + Eq + Clone> Mergeable for ORSet<T> {
    fn merge(&mut self, other: &Self) {
        self.merge(other);
    }
}
