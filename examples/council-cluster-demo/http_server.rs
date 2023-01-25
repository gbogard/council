use std::{collections::HashMap, convert::Infallible, net::SocketAddr, sync::Arc};

use council::node::NodeId;
use rust_embed::RustEmbed;
use warp::{Filter, Rejection, Reply};

use super::templates;
use crate::application::Application;

#[derive(RustEmbed)]
#[folder = "examples/council-cluster-demo/assets"]
struct Assets;

fn extract_node_id() -> impl Filter<Extract = (Option<NodeId>,), Error = Rejection> + Clone {
    warp::query::query::<HashMap<String, String>>().map(|mut query: HashMap<String, String>| {
        query
            .remove("node_id")
            .and_then(|param| param.parse::<NodeId>().ok())
    })
}

async fn render_node(
    application: Arc<Application>,
    node_id: Option<NodeId>,
) -> Result<impl Reply, Infallible> {
    let html = templates::home_page(&*&application, node_id).await;
    Ok(warp::reply::html(html.0))
}

pub async fn start_http_server(application: Arc<Application>) {
    let index = warp::path::end()
        .and(extract_node_id())
        .and_then(move |node_id| render_node(Arc::clone(&application), node_id));
    let serve_assets = warp_embed::embed(&Assets);

    let routes = index.or(serve_assets);

    warp::serve(routes)
        .bind("0.0.0.0:9000".parse::<SocketAddr>().unwrap())
        .await
}
