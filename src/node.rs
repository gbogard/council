#[cfg(test)]
use quickcheck::Arbitrary;

pub mod id {
    use std::hash::{Hash, Hasher};

    #[cfg(test)]
    use quickcheck::Arbitrary;
    use url::Url;

    /// A [NodeId] is a 64-bit number uniquely identifying a single node in a cluster
    #[repr(transparent)]
    #[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct NodeId(pub(crate) u64);

    impl NodeId {
        /// Builds a [NodeId] whose value is a hash of the provided URL
        pub fn from_url(url: &Url) -> Self {
            let mut hasher = crate::hasher::deterministic_hasher();
            url.hash(&mut hasher);
            Self(hasher.finish())
        }
    }

    #[cfg(test)]
    impl Arbitrary for NodeId {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            NodeId(u64::arbitrary(g))
        }
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum NodeStatus {
    // A node attempts to join the cluster
    // Nodes apply this status to themselves and then reach out to peer nodes
    // to join a cluster.
    Joining = 1,
    // Upon convergence, a joining node was marked up by the cluster leader
    // Nodes are not allowed to apply this status to themselves
    Up = 2,
    // A node is about to leave gracefully
    // Nodes apply this status to themselves, and then wait for the cluster to acknowledge
    // their departure by applying the Exiting status to them.
    Leaving = 3,
    // Upon convergence, a leaving node was marked as exiting by the cluster leader
    // Nodes are not allowed to apply this status to themselves
    Exiting = 4,
    // Upon failure, a downing strategy marks an unreachable node as down
    // The Down status is final.
    //A down node can never be marked up again unless the node is entirely restarted.
    Down = 5,
}

#[cfg(test)]
impl Arbitrary for NodeStatus {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        *g.choose(&[
            Self::Down,
            Self::Joining,
            Self::Up,
            Self::Leaving,
            Self::Exiting,
            Self::Down,
        ])
        .unwrap()
    }
}
