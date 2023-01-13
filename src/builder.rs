use std::{
    collections::HashSet,
    time::{Duration, SystemTime},
};

use tokio::{
    sync::{broadcast, mpsc},
};
use url::Url;

use crate::{
    cluster::{
        convergence_monitor::{ConvergenceMonitor},
        failure_detector::{FailureDetector},
        views::ClusterView,
        Cluster,
    },
    node::NodeId,
    Council,
};

pub struct CouncilBuilder {
    this_node_advertised_url: Url,
    this_node_id: NodeId,
    peer_nodes: HashSet<Url>,
    failure_detector_phi_threshold: f64,
    gossip_interval: Duration,
}

impl CouncilBuilder {
    pub fn new(this_node_advertised_url: Url) -> Self {
        let this_node_id = NodeId::from_url(&this_node_advertised_url, SystemTime::now());
        Self {
            this_node_advertised_url,
            this_node_id,
            peer_nodes: HashSet::new(),
            failure_detector_phi_threshold: 8.0,
            gossip_interval: Duration::from_secs(1),
        }
    }
    pub fn with_peer_nodes(mut self, peer_nodes: &[Url]) -> Self {
        self.peer_nodes.extend(peer_nodes.iter().cloned());
        self
    }
    pub fn with_failure_detector_phi_threshold(mut self, threshold: f64) -> Self {
        self.failure_detector_phi_threshold = threshold;
        self
    }
    pub fn with_gossip_interval(mut self, interval_duration: Duration) -> Self {
        self.gossip_interval = interval_duration;
        self
    }
    pub fn build(self) -> Council {
        let (cluster_events_sender, cluster_events_receiver) = broadcast::channel(10);
        let (incoming_gossip_sender, incoming_gossip_receiver) = mpsc::channel(10);

        let outgoing_gossip_interval = tokio::time::interval(self.gossip_interval);

        let cluster_view = ClusterView::initial(self.this_node_id, self.this_node_advertised_url);
        let peer_nodes = self.peer_nodes;
        let failure_detector = FailureDetector::new();
        let convergence_monitor = ConvergenceMonitor::new(self.this_node_id);

        let cluster = Cluster {
            cluster_view,
            peer_nodes,
            failure_detector,
            convergence_monitor,
        };

        let main_thread = tokio::spawn(Council::main_thread(
            outgoing_gossip_interval,
            cluster,
            incoming_gossip_receiver,
            cluster_events_sender,
        ));

        Council {
            cluster_events_receiver,
            incoming_gossip_sender,
            main_thread,
        }
    }
}
