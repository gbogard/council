use std::{collections::HashMap, error::Error, sync::Arc};

use tonic::{transport::Channel, Request};
use url::Url;

use super::{protos, TonicChannelFactory};
use crate::{cluster::views::PartialClusterView, node::NodeId};

pub(crate) type HeartbeatMessage = HashMap<NodeId, u64>;

pub(crate) struct CouncilClient {
    pub(crate) tonic_channel_factory: Arc<dyn TonicChannelFactory + Send + Sync>,
}

impl CouncilClient {
    pub(crate) async fn exchange_cluster_views(
        &self,
        node_advertised_url: Url,
        cluster_view: PartialClusterView,
    ) -> Result<PartialClusterView, Box<dyn Error + Send + Sync + 'static>> {
        let mut client = self.get_client_for_url(node_advertised_url).await?;
        let request = Request::new(cluster_view.into());
        let response = client.exchange_cluster_views(request).await?;
        Ok(response.into_inner().into())
    }

    pub(crate) async fn exchange_heartbeats(
        &self,
        node_advertised_url: Url,
        heartbeat_messsage: HeartbeatMessage,
    ) -> Result<HeartbeatMessage, Box<dyn Error + Send + Sync + 'static>> {
        let mut client = self.get_client_for_url(node_advertised_url).await?;
        let request = Request::new(heartbeat_messsage.into());
        let response = client.exchange_heartbeats(request).await?;
        Ok(response.into_inner().into())
    }

    async fn get_client_for_url(
        &self,
        url: Url,
    ) -> Result<
        protos::gossip_service_client::GossipServiceClient<Channel>,
        Box<dyn Error + Send + Sync + 'static>,
    > {
        let channel = self.tonic_channel_factory.channel_for_url(url).await?;
        Ok(protos::gossip_service_client::GossipServiceClient::new(
            channel,
        ))
    }
}
