const HASHER_SEED: u64 = 23492387589239525;

pub mod id {
    use std::hash::{Hash, Hasher};

    use twox_hash::xxh3::Hash64;
    use url::Url;

    use super::HASHER_SEED;

    /// A [NodeId] is a 64-bit number uniquely identifying a single node in a cluster
    #[repr(transparent)]
    #[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
    pub struct NodeId(u64);

    impl NodeId {
        /// Builds a [NodeId] whose value is a hash of the provided URL
        pub fn from_url(url: &Url) -> Self {
            let mut hasher = Hash64::with_seed(HASHER_SEED);
            url.hash(&mut hasher);
            Self(hasher.finish())
        }

        pub fn to_u64(&self) -> u64 {
            self.0
        }
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum NodeStatus {
    // A node attempts to join the cluster
    Joining,
    // Upon convergence, a joining node was marked up by the cluster leader
    Up,
    // A node is about to leave gracefully
    Leaving,
    // Upon convergence, a leaving node was marked as exiting by the cluster leader
    Exiting,
    // Upon failure, a downing strategy marks an unreachable node as down
    Down,
}
