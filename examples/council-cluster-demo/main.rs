use std::{net::SocketAddr, sync::Arc};

use council::{
    grpc::{DefaultTonicChannelFactory},
    Council,
};
use rand::seq::SliceRandom;
use tokio::sync::oneshot;
use tonic::transport::Server;
use url::Url;

struct RunningCouncil {
    council_instance: Council,
    server_shutdown: oneshot::Sender<()>,
}

struct ApplicationState {
    instances: Vec<RunningCouncil>,
}

impl ApplicationState {
    fn start(size: u16) -> Self {
        let mut rng = rand::thread_rng();
        let base_port = 8080;
        let urls: Vec<Url> = (0..size)
            .map(|i| Url::parse(&format!("http://0.0.0.0:{}", base_port + i)).unwrap())
            .collect();
        let tonic_channel_factory = Arc::new(DefaultTonicChannelFactory::new());
        let instances = urls
            .iter()
            .map(|url| {
                let socket_addr = SocketAddr::new("0.0.0.0".parse().unwrap(), url.port().unwrap());
                let peer_nodes: Vec<Url> = urls.choose_multiple(&mut rng, 2).cloned().collect();

                let council = Council::builder(url.clone())
                    .with_peer_nodes(&peer_nodes[..])
                    .with_tonic_channel_factory_arc(Arc::clone(&tonic_channel_factory))
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

                RunningCouncil {
                    council_instance: council,
                    server_shutdown: shutdown_tx,
                }
            })
            .collect();
        Self { instances }
    }
}

pub fn main() {}
