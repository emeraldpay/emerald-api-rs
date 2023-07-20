use tonic::transport::Channel;
use crate::creds::Credentials;
use tonic::transport::ClientTlsConfig;

pub struct EmeraldConn {
    channel: Channel,
    pub credentials: Credentials
}

impl EmeraldConn {
    pub fn new(channel: Channel, cred: Credentials) -> Self {
        Self {
            channel,
            credentials: cred,
        }
    }

    pub fn channel(&self) -> Channel {
        self.channel.clone()
    }

    pub fn standard_api() -> Channel {
        let tls = ClientTlsConfig::new();
        Channel::builder("https://api.emrld.io".parse().unwrap())
            .tls_config(tls).expect("TLS cannot be configured")
            .connect_lazy()
    }

}

impl Into<Channel> for &EmeraldConn {
     fn into(self) -> Channel {
          self.channel.clone()
     }
}