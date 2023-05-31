use std::collections::HashMap;

use super::node::{NodeId, Node};

pub struct Cluster<NodeData> {
    state: ClusterState<NodeData>
}

pub struct ClusterState<NodeData> {
    nodes: HashMap<NodeId, Node<NodeData>> 
}

pub struct Gossip<NodeData> {
    node: Node<NodeData>
}