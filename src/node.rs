#[cfg(test)]
use quickcheck::Arbitrary;

use self::clock::Clock;

pub mod id {
    use std::hash::{Hash, Hasher};

    #[cfg(test)]
    use quickcheck::Arbitrary;
    use url::Url;

    /// A [NodeId] is a 64-bit number uniquely identifying a single node in a cluster
    #[repr(transparent)]
    #[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
    pub struct NodeId(u64);

    impl NodeId {
        /// Builds a [NodeId] whose value is a hash of the provided URL
        pub fn from_url(url: &Url) -> Self {
            let mut hasher = crate::hasher::deterministic_hasher();
            url.hash(&mut hasher);
            Self(hasher.finish())
        }

        pub fn to_u64(&self) -> u64 {
            self.0
        }
    }

    #[cfg(test)]
    impl Arbitrary for NodeId {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            NodeId(u64::arbitrary(g))
        }
    }
}

pub mod clock {
    #[cfg(test)]
    use quickcheck::Arbitrary;
    use time::OffsetDateTime;

    #[derive(PartialEq, Eq, Debug, Clone, Copy, Hash)]
    pub struct Clock {
        pub started_at: OffsetDateTime,
        pub heartbeat: u64,
    }

    impl Clock {
        pub fn new() -> Self {
            Self {
                started_at: OffsetDateTime::now_utc(),
                heartbeat: 0,
            }
        }
    }

    impl PartialOrd for Clock {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            if self.started_at == other.started_at {
                self.heartbeat.partial_cmp(&other.heartbeat)
            } else {
                self.started_at.partial_cmp(&other.started_at)
            }
        }
    }

    impl Ord for Clock {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            self.partial_cmp(&other)
                .unwrap_or(std::cmp::Ordering::Equal)
        }
    }

    #[cfg(test)]
    impl Arbitrary for Clock {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            Self {
                heartbeat: u64::arbitrary(g),
                started_at: OffsetDateTime::arbitrary(g),
            }
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
    // The Down status is final. A down node can never be marked up again unless the node is
    // entirely restarted.
    Down,
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

impl NodeStatus {
    pub(crate) fn preceding_status(
        a: NodeStatus,
        a_clock: Clock,
        b: NodeStatus,
        b_clock: Clock,
    ) -> Self {
        let same_generation = a_clock.started_at == b_clock.started_at;

        // The Down status always takes precedence within a generation (until the node is completely restarted)
        // Only the cluster leader is allowed to take a node down, so if receive data about a down node in a gossip,
        // we trust that this information comes from the leader.
        // Moreover, the down status is final, so we know that the node will never have another status, unless it is restarted
        if (a == NodeStatus::Down || b == NodeStatus::Down) && same_generation {
            NodeStatus::Down
        }
        // Similarly, since only the cluster leader is allowed to mark a node as Exiting, the Exiting status
        // takes precedence over other statuses
        else if (a == NodeStatus::Exiting || b == NodeStatus::Exiting) && same_generation {
            NodeStatus::Exiting
        }
        // Similarly, since only the cluster leader is allows to mark a node as Up, the Up status takes precedence over the others
        else if (a == NodeStatus::Up || b == NodeStatus::Up) && same_generation {
            NodeStatus::Up
        }
        // If none of this rules apply, the status that takes precedence is the one associated with the highest logical clock
        else if a_clock > b_clock {
            a
        } else {
            b
        }
    }
}
