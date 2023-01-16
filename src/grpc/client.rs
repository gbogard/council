use std::{collections::HashMap, error::Error};

use tokio::sync::Mutex;
use tonic::{transport::Channel, Request};
use url::Url;

use super::protos;
use crate::{
    cluster::views::{MemberView, PartialClusterView},
    node::NodeId,
};

pub(crate) type HeartbeatMessage = HashMap<NodeId, u64>;

pub(crate) struct CouncilClient {
    pub(crate) tonic_channel_factory: Box<dyn Fn(Url) -> Channel>,
    open_channels: Mutex<HashMap<Url, Channel>>,
}

impl CouncilClient {
    pub(crate) async fn exchange_cluster_views(
        &mut self,
        node_advertised_url: Url,
        cluster_view: PartialClusterView,
    ) -> Result<PartialClusterView, Box<dyn Error>> {
        let mut client = self.get_client_for_url(node_advertised_url).await;
        let request = Request::new(cluster_view.into());
        let response = client.exchange_cluster_views(request).await?;
        Ok(response.into_inner().into())
    }

    pub(crate) async fn exchange_heartbeats(
        &mut self,
        node_advertised_url: Url,
        heartbeat_messsage: HeartbeatMessage,
    ) -> Result<HeartbeatMessage, Box<dyn Error>> {
        let mut client = self.get_client_for_url(node_advertised_url).await;
        let request = Request::new(heartbeat_messsage.into());
        let response = client.exchange_heartbeats(request).await?;
        Ok(response.into_inner().into())
    }

    async fn get_client_for_url(
        &mut self,
        url: Url,
    ) -> protos::gossip_service_client::GossipServiceClient<Channel> {
        let mut guard = self.open_channels.lock().await;
        let channel = guard
            .entry(url.clone())
            .or_insert_with(|| (self.tonic_channel_factory)(url))
            .clone();
        protos::gossip_service_client::GossipServiceClient::new(channel)
    }
}
