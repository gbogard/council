mod protos {
    tonic::include_proto!("council");
}

mod client;
mod dtos_conversions;
mod server;

pub use server::*;
