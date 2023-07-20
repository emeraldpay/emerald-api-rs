extern crate prost;

#[cfg(feature = "blockchain")]
pub mod blockchain {
    #[cfg(feature = "client-blockchain")]
    use crate::proto::blockchain::blockchain_client;

    #[cfg(feature = "client-blockchain")]
    pub fn connect(conn: &crate::conn::EmeraldConn) ->  blockchain_client::BlockchainClient<tonic::service::interceptor::InterceptedService<tonic::transport::Channel, crate::creds::Credentials>> {
        let interceptor: crate::creds::Credentials = conn.credentials.clone();
        blockchain_client::BlockchainClient::<tonic::transport::Channel>::with_interceptor(conn.into(), interceptor)
    }
}

#[cfg(feature = "market")]
pub mod market {
    #[cfg(feature = "client-market")]
    use crate::proto::market::market_client;

    #[cfg(feature = "client-market")]
    pub fn connect(conn: &crate::conn::EmeraldConn) ->  market_client::MarketClient<tonic::service::interceptor::InterceptedService<tonic::transport::Channel, crate::creds::Credentials>> {
        let interceptor: crate::creds::Credentials = conn.credentials.clone();
        market_client::MarketClient::<tonic::transport::Channel>::with_interceptor(conn.into(), interceptor)
    }
}

#[cfg(feature = "monitoring")]
pub mod monitoring {
    #[cfg(feature = "client-monitoring")]
    use crate::proto::monitoring::monitoring_client;

    #[cfg(feature = "client-monitoring")]
    pub fn connect(conn: &crate::conn::EmeraldConn) ->  monitoring_client::MonitoringClient<tonic::service::interceptor::InterceptedService<tonic::transport::Channel, crate::creds::Credentials>> {
        let interceptor: crate::creds::Credentials = conn.credentials.clone();
        monitoring_client::MonitoringClient::<tonic::transport::Channel>::with_interceptor(conn.into(), interceptor)
    }
}

#[cfg(feature = "transaction")]
pub mod transaction {
    #[cfg(feature = "client-transaction")]
    use crate::proto::transaction::transaction_client;

    #[cfg(feature = "client-transaction")]
    pub fn connect(conn: &crate::conn::EmeraldConn) ->  transaction_client::TransactionClient<tonic::service::interceptor::InterceptedService<tonic::transport::Channel, crate::creds::Credentials>> {
        let interceptor: crate::creds::Credentials = conn.credentials.clone();
        transaction_client::TransactionClient::<tonic::transport::Channel>::with_interceptor(conn.into(), interceptor)
    }
}

pub mod proto {
    tonic::include_proto!("emerald");

    #[cfg(feature = "auth")]
    pub mod auth {
        tonic::include_proto!("auth/emerald");
    }

    #[cfg(feature = "blockchain")]
    pub mod blockchain {
        tonic::include_proto!("blockchain/emerald");
    }

    #[cfg(feature = "market")]
    pub mod market {
        tonic::include_proto!("market/emerald");
    }

    #[cfg(feature = "monitoring")]
    pub mod monitoring {
        tonic::include_proto!("monitoring/emerald");
    }

    #[cfg(feature = "transaction")]
    pub mod transaction {
        tonic::include_proto!("transaction/emerald");

        // re-export transaction types from submodule (also called `transaction`)
        // because otherwise you have to repeat the module name twice when using them
        pub use transaction::*;

        // added as a submodule too because that's how Tonic generates dependencies between proto files
        mod transaction {
            tonic::include_proto!("transaction/emerald.transaction");
        }
    }
}

pub mod errors;
#[cfg(feature = "client")]
pub mod conn;
#[cfg(feature = "client")]
pub mod creds;
