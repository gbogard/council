use std::collections::HashMap;

use crate::node::id::NodeId;

#[derive(PartialEq, Eq, Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(transparent)]
pub struct VersionVector {
    pub(crate) versions: HashMap<NodeId, u16>,
}
