use std::{
    fmt::Display,
    hash::{Hash, Hasher},
    str::FromStr,
    time::SystemTime,
};

use num_enum::{IntoPrimitive, TryFromPrimitive};
#[cfg(test)]
use quickcheck::Arbitrary;
use url::Url;

/// A [NodeId] uniquely indentifies a node and its specific execution.
/// When a node crashses or leaves the cluster, it cannot have the same [NodeId] as the previous run,
/// even if it has the same URL.
#[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
#[cfg_attr(feature = "serde", derive(SerializeDisplay, DeserializeFromStr))]
pub struct NodeId {
    /// This field uniquely is obtained by hashing the node's advertised URL, which
    /// must be unique troughout the entire cluster
    pub unique_id: u64,
    /// This field is used to disambiguate successive restarts of the same node.
    /// It is simply the number of seconds elapsed since the UNIX epoch at the time the node was started.
    /// When a node restarts after exiting the cluster, it will have a different generation value.
    /// When reconciling two views of the same node, the most recent generation always wins.
    /// If the two views have an identical generation field, then we use the version field to reconcile them.
    pub generation: u64,
}

impl NodeId {
    /// Builds a [NodeId] whose value is a hash of the provided URL
    pub fn from_url(url: &Url, node_started_at: SystemTime) -> Self {
        let generation = node_started_at
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let mut hasher = siphasher::sip::SipHasher::new();
        url.hash(&mut hasher);
        Self {
            unique_id: hasher.finish(),
            generation,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseNodeIdError;

impl Display for ParseNodeIdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Cannot parse NodeId")
    }
}

impl Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}.{}", self.unique_id, self.generation))
    }
}

impl FromStr for NodeId {
    type Err = ParseNodeIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (unique_id, generation) = s.split_once(".").ok_or(ParseNodeIdError)?;
        let unique_id = unique_id.parse::<u64>().map_err(|_| ParseNodeIdError)?;
        let generation = generation.parse::<u64>().map_err(|_| ParseNodeIdError)?;
        Ok(NodeId {
            unique_id,
            generation,
        })
    }
}

#[cfg(test)]
impl Arbitrary for NodeId {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        NodeId {
            unique_id: u64::arbitrary(g),
            generation: u64::arbitrary(g),
        }
    }
}

#[repr(transparent)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Version {
    number: u64,
}

impl Version {
    pub(crate) fn new() -> Version {
        Version { number: 0 }
    }

    pub(crate) fn incr(&mut self) {
        self.number += 1;
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord, TryFromPrimitive, IntoPrimitive)]
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

impl Display for NodeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeStatus::Joining => f.write_str("Joining"),
            NodeStatus::Up => f.write_str("Up"),
            NodeStatus::Leaving => f.write_str("Leaving"),
            NodeStatus::Exiting => f.write_str("Exiting"),
            NodeStatus::Down => f.write_str("Down"),
        }
    }
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
