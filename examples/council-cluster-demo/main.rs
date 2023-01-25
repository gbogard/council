use std::sync::Arc;

use application::Application;
use log::LevelFilter;
use simplelog::{Config, ConfigBuilder, SimpleLogger};

mod application;
mod http_server;
mod logs;
mod templates;

#[tokio::main]
pub async fn main() {
    let _ = SimpleLogger::init(
        LevelFilter::Debug,
        ConfigBuilder::new()
            .add_filter_ignore_str("h2::codec")
            .add_filter_ignore_str("tower::buffer")
            .build(),
    );

    let number_of_nodes_to_start: u64 = std::env::args()
        .find_map(|arg| arg.parse::<u64>().ok().filter(|n| n > &0 && n <= &100))
        .unwrap_or(6);

    log::info!("Starting {} Council nodes", number_of_nodes_to_start);

    let app = Arc::new(Application::start(number_of_nodes_to_start).await);
    let server = tokio::spawn(http_server::start_http_server(Arc::clone(&app)));

    log::info!("Demo is available on http://localhost:8080 !");

    tokio::select! {
        signal = tokio::signal::ctrl_c() => {
            signal.unwrap()
        },
        _ = server => {
            panic!()
        }
    }
}
