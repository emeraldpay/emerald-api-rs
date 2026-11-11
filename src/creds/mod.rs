//! Credentials for calling Emerald APIs.
//!
//! An API connection authenticates with a secret Auth Token, which it exchanges for a short-lived JWT
//! on the first call, and then attaches that JWT to every call. The JWT is renewed with its refresh token
//! before it expires, so a long-running client keeps working without any action on its side.

mod service;
mod session;

pub use service::AuthService;
pub(crate) use service::AuthLayer;
pub(crate) use session::AuthSession;

use chrono::{DateTime, Utc};
use std::fmt::{Debug, Formatter};
use crate::proto::auth::AuthResponse;

#[derive(Debug, Clone)]
pub enum Credentials {

    /// No credentials, just for plain unauthenticated calls
    None,

    ///
    /// Authenticated with a JWT token
    Token(JwtState),
}

#[derive(Clone)]
pub enum JwtState {
    ///
    /// An initial state of the API connection. Exchanges it to JWT on first use.
    Initial{
        ///
        /// A secret token to exchange for a JWT
        secret: String
    },
    Authenticated {
        /// The secret token the JWT was issued for.
        /// The server requires it with each refresh, and the client needs it to authenticate again
        /// when the refresh token is rejected (e.g., after being idle for longer than the refresh token lives).
        secret: String,
        /// JWT received to authenticate future calls
        jwt: String,
        /// Expiration time of the JWT token; client automatically refreshes the JWT before that moment
        expires_at: DateTime<Utc>,
        /// a secret token that can be exchanged for another JWT before the expiration time
        refresh: String,
    }
}

impl JwtState {
    ///
    /// Create a new initial state of JWT auth
    ///
    /// @param secret - secret token for exchanging for a JWT token, e.g., an API token
    pub fn new(secret: String) -> Self {
        JwtState::Initial {
            secret
        }
    }

    /// The state after the server issued a JWT in exchange for `secret`
    pub(crate) fn issued(secret: String, response: AuthResponse) -> Self {
        JwtState::Authenticated {
            secret,
            jwt: response.access_token,
            refresh: response.refresh_token,
            expires_at: DateTime::from_timestamp_millis(response.expires_at as i64)
                // an invalid ts will not happen unless there is a major bug in the server,
                // but if it happens we consider that the JWT is valid for at least a minute
                .unwrap_or(Utc::now() + chrono::Duration::minutes(1)),
        }
    }
}

/// Doesn't print the tokens, so the state can be logged without leaking them
impl Debug for JwtState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            JwtState::Initial { .. } => f.write_str("Initial"),
            JwtState::Authenticated { expires_at, .. } => f
                .debug_struct("Authenticated")
                .field("expires_at", expires_at)
                .finish_non_exhaustive(),
        }
    }
}

impl Default for Credentials {
    fn default() -> Self {
        Credentials::unauthenticated()
    }
}

impl Credentials {

    ///
    /// Do nothing, for plain unauthenticated calls
    pub fn unauthenticated() -> Self {
        Credentials::None
    }

    ///
    /// Authenticate using a predefined JWT token, i.e., by putting it in the `Authorization` header
    pub fn jwt<S: ToString>(jwt: S) -> Self {
        Credentials::Token(JwtState::new(jwt.to_string()))
    }

    ///
    /// Authenticate using an API token
    pub fn token<S: ToString>(secret_token: S) -> Self {
        Credentials::Token(JwtState::Initial { secret: secret_token.to_string() })
    }
}
