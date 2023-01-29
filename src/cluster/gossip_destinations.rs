use rand::seq::SliceRandom;
use url::Url;

use crate::cluster::{views::PartialClusterView, Cluster};

const GOSSIP_DESTINATIONS_SAMPLE_SIZE: usize = 3;

#[derive(Debug)]
pub(crate) struct GossipDestination {
    pub(crate) destination_url: Url,
    pub(crate) cluster_view: PartialClusterView,
}

impl Cluster {
    /// Selects destinations to gossip with, up to a maximum of three
    pub(crate) fn select_gossip_destinations(&mut self) -> Vec<GossipDestination> {
        let mut destinations = Vec::new();
        let cluster_view = PartialClusterView {
            this_node_id: self.this_node_id,
            members: self.cluster_view.known_members.clone(),
        };

        for url in &self.unknwon_peer_nodes {
            if destinations.len() >= GOSSIP_DESTINATIONS_SAMPLE_SIZE
                || url == &self.this_advertised_url
            {
                break;
            }

            {
                log::debug!(
                    "[Node Id: {}] Exchanging cluster views with unknown peer node {}",
                    self.this_node_id,
                    url
                );
                destinations.push(GossipDestination {
                    destination_url: url.clone(),
                    cluster_view: cluster_view.clone(),
                });
            }
        }

        let urls: Vec<&Url> = self
            .cluster_view
            .known_members
            .iter()
            .filter_map(|(_, m)| {
                if m.advertised_addr == self.this_advertised_url {
                    None
                } else {
                    Some(&m.advertised_addr)
                }
            })
            .collect();

        let remaining_exchanges = std::cmp::min(
            GOSSIP_DESTINATIONS_SAMPLE_SIZE - destinations.len(),
            self.cluster_view.known_members.len(),
        );

        if remaining_exchanges > 0 {
            log::debug!(
                "[Node Id: {}] Exchanging heartbeats with {} random members",
                self.this_node_id,
                remaining_exchanges
            );
            for url in urls.choose_multiple(&mut rand::thread_rng(), remaining_exchanges) {
                destinations.push(GossipDestination {
                    destination_url: (*url).clone(),
                    cluster_view: cluster_view.clone(),
                })
            }
        }

        destinations
    }
}
