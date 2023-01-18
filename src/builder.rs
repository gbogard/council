use std::{
    collections::HashSet,
    sync::Arc,
    time::{Duration, SystemTime},
};

use tokio::sync::{broadcast, mpsc};
use url::Url;

use crate::{
    cluster::{
        convergence_monitor::ConvergenceMonitor, failure_detector::FailureDetector,
        views::ClusterView, Cluster,
    },
    grpc::{client::CouncilClient, DefaultTonicChannelFactory, TonicChannelFactory},
    node::NodeId,
    Council,
};

pub struct CouncilBuilder {
    this_node_advertised_url: Url,
    this_node_id: NodeId,
    peer_nodes: HashSet<Url>,
    failure_detector_phi_threshold: f64,
    gossip_interval: Duration,
    tonic_channel_factory: Arc<dyn TonicChannelFactory + Send + Sync>,
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
            tonic_channel_factory: Arc::new(DefaultTonicChannelFactory::new()),
        }
    }
    pub fn with_tonic_channel_factory<F: TonicChannelFactory + Send + Sync + 'static>(
        mut self,
        factory: F,
    ) -> Self {
        self.tonic_channel_factory = Arc::new(factory);
        self
    }
    pub fn with_tonic_channel_factory_arc<F: TonicChannelFactory + Send + Sync + 'static>(
        mut self,
        factory: Arc<F>,
    ) -> Self {
        self.tonic_channel_factory = factory;
        self
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
        let (message_sender, message_receiver) = mpsc::channel(10);

        let outgoing_gossip_interval = tokio::time::interval(self.gossip_interval);

        let cluster_view = ClusterView::initial(self.this_node_id, self.this_node_advertised_url);
        let peer_nodes = self.peer_nodes;
        let failure_detector = FailureDetector::new(self.this_node_id);
        let convergence_monitor = ConvergenceMonitor::new(self.this_node_id);

        let cluster = Cluster {
            this_node_id: self.this_node_id,
            cluster_view,
            unknwon_peer_nodes: peer_nodes.clone(),
            peer_nodes,
            failure_detector,
            convergence_monitor,
        };

        let client = Arc::new(CouncilClient {
            tonic_channel_factory: Arc::clone(&self.tonic_channel_factory),
        });

        let main_thread = tokio::spawn(Council::main_thread(
            outgoing_gossip_interval,
            cluster,
            message_receiver,
            cluster_events_sender,
            client,
        ));

        Council {
            cluster_events_receiver,
            tonic_channel_factory: self.tonic_channel_factory,
            main_thread_message_sender: message_sender,
            main_thread,
        }
    }
}
