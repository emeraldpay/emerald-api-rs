use std::sync::{Arc, RwLock};
use tonic::transport::{Channel, Uri};
use crate::creds::{AuthLayer, AuthService, Credentials};
use tonic::transport::ClientTlsConfig;
use tower::ServiceBuilder;
use crate::errors::Error;

#[derive(Clone)]
pub struct EmeraldConn {
    channel: Channel,
    pub(crate) credentials: Arc<RwLock<Credentials>>
}

impl EmeraldConn {

    ///
    /// Build a connection from provided gRPC channel and credentials.
    /// NOTE: in most cases you want to use `#connect` instead, and this method is more useful for unit-testing with a channel mock.
    ///
    /// @param channel - channel to use
    /// @param cred - credentials to use
    pub fn new(channel: Channel, cred: Credentials) -> Self {
        Self {
            channel,
            credentials: Arc::new(RwLock::new(cred)),
        }
    }

    ///
    /// Get gRPC channel tp use for API call, with the credentials layer.
    ///
    pub fn channel(&self) -> AuthService<Channel> {
        let auth_layer = AuthLayer::new(self.credentials.clone());

        ServiceBuilder::new()
            .layer(auth_layer)
            .service(self.channel.clone())
    }

    ///
    /// Lazily connect using the provided credentials
    ///
    /// @param cred - credentials to use
    pub fn connect(cred: Credentials) -> Self {
        Self::connect_endpoint("https://api.emrld.io", cred).unwrap()
    }

    ///
    /// Lazily connect using the provided credentials to a non-default URI
    ///
    /// @param uri - URI to connect to. Must be a valid URI, e.g., "https://api.emrld.io" or "http://localhost:8080"
    /// @param cred - credentials to use
    pub fn connect_endpoint<S: TryInto<Uri>>(uri: S, cred: Credentials) -> Result<Self, Error> {
        let tls = ClientTlsConfig::new().with_native_roots();
        let uri = uri.try_into().map_err(|_| Error::Transport("Invalid URI".to_string()))?;
        let channel = Channel::builder(uri)
            .tls_config(tls).expect("TLS cannot be configured")
            .connect_lazy();
        Ok(Self::new(channel, cred))
    }

    ///
    /// Set the credentials for this connection
    /// NOTE: this must be called before trying to connect to an API. I.e., before `emerald_api::API_SERVICE::connect(emerald_conn)`
    ///
    /// @param cred - credentials to use
    pub fn with_credentials(self, cred: Credentials) -> Self {
        Self::new(self.channel, cred)
    }

    pub fn get_credentials(&self) -> Credentials {
        self.credentials.read().unwrap().clone()
    }
}

impl Into<Channel> for &EmeraldConn {
     fn into(self) -> Channel {
          self.channel.clone()
     }
}