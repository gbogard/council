use cluster::Cluster;
use tokio::{
    select,
    sync::{broadcast, mpsc},
    task::JoinHandle,
    time::Interval,
};
use url::Url;

#[cfg(test)]
extern crate quickcheck;
#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

mod builder;
pub mod cluster;
pub mod grpc;
pub mod node;

pub use self::builder::*;

pub struct Council {
    cluster_events_receiver: broadcast::Receiver<Cluster>,
    incoming_gossip_sender: mpsc::Sender<IncomingGossipMessage>,
    main_thread: JoinHandle<()>,
}

impl Council {
    pub fn builder(this_node_advertised_url: Url) -> CouncilBuilder {
        CouncilBuilder::new(this_node_advertised_url)
    }

    pub(crate) async fn main_thread(
        mut outgoing_gossip_interval: Interval,
        _cluster: Cluster,
        mut incoming_gossip_receiver: mpsc::Receiver<IncomingGossipMessage>,
        _cluster_events_sender: broadcast::Sender<Cluster>,
    ) {
        loop {
            select! {
                Some(_incoming_gossip) = incoming_gossip_receiver.recv() => {},
                _ = outgoing_gossip_interval.tick() => {

                }
            }
        }
    }
}

struct IncomingGossipMessage {}
