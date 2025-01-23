extern crate prost;

#[cfg(feature = "auth")]
pub mod auth {
    #[cfg(feature = "client-auth")]
    use crate::creds::AuthService;
    #[cfg(feature = "client-auth")]
    use crate::proto::auth::auth_client;

    #[cfg(feature = "client-auth")]
    pub fn connect(conn: &crate::conn::EmeraldConn) ->  auth_client::AuthClient<AuthService<tonic::transport::Channel>> {
        auth_client::AuthClient::new(conn.channel())
    }
}

#[cfg(feature = "blockchain")]
pub mod blockchain {
    #[cfg(feature = "client-blockchain")]
    use crate::creds::AuthService;
    #[cfg(feature = "client-blockchain")]
    use crate::proto::blockchain::blockchain_client;

    #[cfg(feature = "client-blockchain")]
    pub fn connect(conn: &crate::conn::EmeraldConn) ->  blockchain_client::BlockchainClient<AuthService<tonic::transport::Channel>> {
        blockchain_client::BlockchainClient::new(conn.channel())
    }
}

#[cfg(feature = "market")]
pub mod market {
    #[cfg(feature = "client-market")]
    use crate::creds::AuthService;
    #[cfg(feature = "client-market")]
    use crate::proto::market::market_client;
    #[cfg(feature = "client-market")]
    pub fn connect(conn: &crate::conn::EmeraldConn) -> market_client::MarketClient<AuthService<tonic::transport::Channel>> {
        market_client::MarketClient::new(conn.channel())
    }
}
#[cfg(feature = "monitoring")]
pub mod monitoring {
    #[cfg(feature = "client-monitoring")]
    use crate::creds::AuthService;
    #[cfg(feature = "client-monitoring")]
    use crate::proto::monitoring::monitoring_client;
    #[cfg(feature = "client-monitoring")]
    pub fn connect(conn: &crate::conn::EmeraldConn) -> monitoring_client::MonitoringClient<AuthService<tonic::transport::Channel>> {
        monitoring_client::MonitoringClient::new(conn.channel())
    }
}
#[cfg(feature = "transaction")]
pub mod transaction {
    #[cfg(feature = "client-transaction")]
    use crate::creds::AuthService;
    #[cfg(feature = "client-transaction")]
    use crate::proto::transaction::transaction_client;
    #[cfg(feature = "client-transaction")]
    pub fn connect(conn: &crate::conn::EmeraldConn) -> transaction_client::TransactionClient<AuthService<tonic::transport::Channel>> {
        transaction_client::TransactionClient::new(conn.channel())
    }
}

#[cfg(feature = "token")]
pub mod token {
    #[cfg(feature = "client-token")]
    use crate::creds::AuthService;
    #[cfg(feature = "client-token")]
    use crate::proto::token::token_client;
    #[cfg(feature = "client-token")]
    pub fn connect(conn: &crate::conn::EmeraldConn) -> token_client::TokenClient<AuthService<tonic::transport::Channel>> {
        token_client::TokenClient::new(conn.channel())
    }
}

pub mod proto {
    pub mod common {
        tonic::include_proto!("emerald");
    }

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

    #[cfg(feature = "token")]
    pub mod token {
        tonic::include_proto!("token/emerald");

        // re-export token types from submodule (also called `token`)
        // because otherwise you have to repeat the module name twice when using them
        pub use token::*;

        // added as a submodule too because that's how Tonic generates dependencies between proto files
        mod token {
            tonic::include_proto!("token/emerald.token");
        }
    }
}

pub mod errors;
#[cfg(feature = "client")]
pub mod conn;
#[cfg(feature = "client")]
pub mod creds;
pub mod common;
