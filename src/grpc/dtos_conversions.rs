use url::Url;

use super::{client::HeartbeatMessage, protos};
use crate::{
    cluster::views::{MemberView, MemberViewState, PartialClusterView},
    node::{NodeId, NodeStatus},
};

impl From<protos::PartialClusterView> for PartialClusterView {
    fn from(value: protos::PartialClusterView) -> Self {
        PartialClusterView {
            this_node_id: value.this_node_id.unwrap().into(),
            members: value
                .members
                .into_iter()
                .map(|m| (m.node_id.unwrap().into(), m.member.unwrap().into()))
                .collect(),
        }
    }
}

impl From<PartialClusterView> for protos::PartialClusterView {
    fn from(value: PartialClusterView) -> Self {
        protos::PartialClusterView {
            this_node_id: Some(value.this_node_id.into()),
            members: value
                .members
                .into_iter()
                .map(|(_, m)| protos::PartialClusterViewEntry {
                    node_id: Some(m.id.into()),
                    member: Some(m.into()),
                })
                .collect(),
        }
    }
}

impl From<protos::MemberView> for MemberView {
    fn from(value: protos::MemberView) -> Self {
        MemberView {
            id: value.id.unwrap().into(),
            advertised_addr: Url::parse(&value.advertised_addr).unwrap(),
            state: value.state.map(MemberViewState::from),
        }
    }
}

impl From<MemberView> for protos::MemberView {
    fn from(value: MemberView) -> Self {
        Self {
            id: Some(value.id.into()),
            advertised_addr: value.advertised_addr.to_string(),
            state: value.state.map(protos::MemberViewState::from),
        }
    }
}

impl From<protos::MemberViewState> for MemberViewState {
    fn from(value: protos::MemberViewState) -> Self {
        MemberViewState {
            node_status: NodeStatus::try_from(value.node_status as u8).unwrap(),
            version: value.version as u16,
            heartbeat: value.heartbeat,
        }
    }
}

impl From<MemberViewState> for protos::MemberViewState {
    fn from(value: MemberViewState) -> Self {
        Self {
            node_status: value.node_status as u32,
            version: value.version as u32,
            heartbeat: value.heartbeat,
        }
    }
}

impl From<protos::HeartbeatMessage> for HeartbeatMessage {
    fn from(value: protos::HeartbeatMessage) -> Self {
        value
            .members
            .into_iter()
            .map(|m| (m.node_id.unwrap().into(), m.heartbeat))
            .collect()
    }
}

impl From<HeartbeatMessage> for protos::HeartbeatMessage {
    fn from(value: HeartbeatMessage) -> Self {
        Self {
            members: value
                .into_iter()
                .map(|(id, heartbeat)| protos::HeartbeatMessageEntry {
                    node_id: Some(id.into()),
                    heartbeat,
                })
                .collect(),
        }
    }
}

impl From<protos::NodeId> for NodeId {
    fn from(value: protos::NodeId) -> Self {
        Self {
            unique_id: value.unique_id,
            generation: value.generation,
        }
    }
}

impl From<NodeId> for protos::NodeId {
    fn from(value: NodeId) -> Self {
        Self {
            unique_id: value.unique_id,
            generation: value.generation,
        }
    }
}
