use std::fmt::Display;

use internment::Intern;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeId {
    inner: Intern<String>,
}

impl NodeId {
    pub fn new(str: impl ToString) -> Self {
        Self {
            inner: Intern::new(str.to_string()),
        }
    }
}

impl Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

pub enum NodeStatus {
    Joining,
    Up {
        activated_by: NodeId,
        activated_at_hearbeat: u64
    },
    Leaving {
        leaving_at_heartbeat: u64
    },
    Down {
        deactivated_by: NodeId,
        deactivated_at_heartbeat: u64
    }
}

pub struct Node<NodeData> {
    pub id: NodeId,
    pub heartbeat: u64,
    pub data: NodeData,
    pub seed_nodes: Vec<NodeId>
}

impl<NodeData> Node<NodeData> {
    /// Increments this node's heartbeat and returns it
    pub(crate) fn incr_heartbeat(&mut self) -> u64 {
        self.heartbeat += 1;
        self.heartbeat
    }
}