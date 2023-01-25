use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use council::{
    grpc::{DefaultTonicChannelFactory, TonicChannelFactory},
    node::NodeId,
    ClusterEvent, Council,
};
use rand::seq::SliceRandom;
use tokio::sync::{oneshot, Mutex};
use tokio_stream::StreamExt;
use tonic::transport::Server;
use url::Url;

pub(crate) struct RunningCouncil {
    pub council_instance: Council,
    last_event: Arc<Mutex<Option<ClusterEvent>>>,
    server_shutdown: tokio::sync::oneshot::Sender<()>,
}

impl RunningCouncil {
    pub async fn last_event_clone(&self) -> Option<ClusterEvent> {
        (&*self.last_event.lock().await).clone()
    }

    fn new(
        url: &Url,
        peer_nodes: Vec<Url>,
        tonic_channel_factory: Arc<impl TonicChannelFactory + Send + Sync + 'static>,
    ) -> Self {
        let socket_addr = SocketAddr::new("0.0.0.0".parse().unwrap(), url.port().unwrap());

        let council = Council::builder(url.clone())
            .with_peer_nodes(&peer_nodes[..])
            .with_tonic_channel_factory_arc(tonic_channel_factory)
            .build();

        let gossip_service = council.gossip_grpc_service();

        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        // Start the gRPC server in a seperate task
        tokio::spawn(async move {
            Server::builder()
                .add_service(gossip_service)
                .serve_with_shutdown(socket_addr, async {
                    let _ = shutdown_rx.await;
                })
                .await
        });

        // Listen to events in a separate task
        let last_event = Arc::new(Mutex::new(None));
        let mut events = council.events();
        tokio::spawn({
            let last_event = Arc::clone(&last_event);
            async move {
                while let Some(event) = events.next().await {
                    let mut last_event = last_event.lock().await;
                    *last_event = Some(event);
                    drop(last_event);
                }
            }
        });

        RunningCouncil {
            council_instance: council,
            server_shutdown: shutdown_tx,
            last_event,
        }
    }
}

pub struct Application {
    pub(crate) instances: HashMap<NodeId, RunningCouncil>,
}

impl Application {
    pub async fn start(size: u64) -> Self {
        let base_port = 8080;
        let urls: Vec<Url> = (0..size)
            .map(|i| Url::parse(&format!("http://localhost:{}", base_port + i)).unwrap())
            .collect();

        let peer_nodes: Vec<Url> = urls
            .choose_multiple(&mut rand::thread_rng(), 3)
            .cloned()
            .collect();

        let tonic_channel_factory = Arc::new(DefaultTonicChannelFactory::new());

        let instances: HashMap<NodeId, RunningCouncil> = urls
            .iter()
            .map(|url| {
                let instance = RunningCouncil::new(
                    url,
                    peer_nodes.clone(),
                    Arc::clone(&tonic_channel_factory),
                );

                (instance.council_instance.this_node_id, instance)
            })
            .collect();

        Self { instances }
    }
}
