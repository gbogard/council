mod protos {
    tonic::include_proto!("council");
}

pub(crate) mod channel_factory;
pub(crate) mod client;
pub(crate) mod dtos_conversions;
pub(crate) mod server;

pub use channel_factory::*;
pub use server::*;
