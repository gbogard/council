use std::{collections::HashMap, error::Error, ops::Deref, str::FromStr, sync::Arc};

use tokio::sync::Mutex;
use tonic::{
    async_trait,
    transport::{Channel, Endpoint},
};
use url::Url;

/// Describes types that can turn URLs into Tonic [channels](Channel).
/// The default implementation creates a simple channel from the provided URL, with no extra option.
/// By implementing this trait and uing your implementation in your [Council](crate::Council),
/// you can pass extra options to Tonic, like TLS certificates.
#[async_trait]
pub trait TonicChannelFactory {
    async fn channel_for_url(
        &self,
        url: Url,
    ) -> Result<Channel, Box<dyn Error + Send + Sync + 'static>>;
}

pub struct DefaultTonicChannelFactory {}

impl DefaultTonicChannelFactory {
    /// Builds a cached [TonicChannelFactory] using the default implementation
    pub fn new() -> impl TonicChannelFactory {
        TonicChannelFactoryCache::new(DefaultTonicChannelFactory {})
    }
}

#[async_trait]
impl TonicChannelFactory for DefaultTonicChannelFactory {
    async fn channel_for_url(
        &self,
        url: Url,
    ) -> Result<Channel, Box<dyn Error + Send + Sync + 'static>> {
        let endpoint = Endpoint::from_str(url.as_str())?;
        Ok(endpoint.connect().await?)
    }
}

/// Wraps another struct inplementing [TonicChannelFactory] and caches
/// channels so they are reused between calls to [TonicChannelFactory::channel_for_url].
pub struct TonicChannelFactoryCache<F: TonicChannelFactory> {
    factory: F,
    open_channels: Mutex<HashMap<Url, Channel>>,
}
impl<F: TonicChannelFactory + Send + Sync> TonicChannelFactoryCache<F> {
    /// Turns an existing [TonicChannelFactory] into a cached factory that reuses open
    /// channels between calls to [TonicChannelFactory::channel_for_url].
    pub fn new(underlying_factory: F) -> Self {
        Self {
            factory: underlying_factory,
            open_channels: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl<F: TonicChannelFactory + Send + Sync> TonicChannelFactory for TonicChannelFactoryCache<F> {
    async fn channel_for_url(
        &self,
        url: Url,
    ) -> Result<Channel, Box<dyn Error + Send + Sync + 'static>> {
        let mut guard = self.open_channels.lock().await;
        if let Some(channel) = guard.get(&url) {
            Ok(channel.clone())
        } else {
            let channel = self.factory.channel_for_url(url.clone()).await?;
            guard.insert(url, channel.clone());
            Ok(channel)
        }
    }
}
