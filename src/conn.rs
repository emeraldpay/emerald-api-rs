use std::sync::{Arc, RwLock};
use tonic::transport::{Channel, Uri};
use crate::creds::{AuthLayer, AuthService, Credentials};
use tonic::transport::ClientTlsConfig;
use tower::ServiceBuilder;
use crate::errors::Error;

pub struct EmeraldConn {
    channel: Channel,
    pub(crate) credentials: Arc<RwLock<Credentials>>
}

impl EmeraldConn {
    pub fn new(channel: Channel, cred: Credentials) -> Self {
        Self {
            channel,
            credentials: Arc::new(RwLock::new(cred)),
        }
    }

    pub fn channel(&self) -> AuthService<Channel> {
        let auth_layer = AuthLayer::new(self.credentials.clone());

        ServiceBuilder::new()
            .layer(auth_layer)
            .service(self.channel.clone())
    }

    pub fn connect(cred: Credentials) -> Self {
        Self::connect_endpoint("https://api.emrld.io", cred).unwrap()
    }

    pub fn connect_endpoint<S: TryInto<Uri>>(uri: S, cred: Credentials) -> Result<Self, Error> {
        let tls = ClientTlsConfig::new().with_native_roots();
        let uri = uri.try_into().map_err(|_| Error::Transport("Invalid URI".to_string()))?;
        let channel = Channel::builder(uri)
            .tls_config(tls).expect("TLS cannot be configured")
            .connect_lazy();
        Ok(EmeraldConn::new(channel, cred))
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