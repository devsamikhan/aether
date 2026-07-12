pub mod distributed;
pub mod gcounter;
pub mod gset;
pub mod orset;
pub mod pncounter;

pub use distributed::{DistributedSystem, Mergeable};
pub use gcounter::GCounter;
pub use gset::GSet;
pub use orset::ORSet;
pub use pncounter::PNCounter;

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
