mod protos {
    tonic::include_proto!("council");
}

mod client;
mod server;

pub use server::*;
