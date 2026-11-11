use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use tokio::sync::OwnedMutexGuard;
use tonic::{
    Status,
    body::Body,
    client::GrpcService,
    codegen::StdError,
};
use crate::creds::{Credentials, JwtState};
use crate::proto::auth::{auth_client::AuthClient, AuthRequest, RefreshRequest};

/// How long before its expiration a JWT gets renewed.
/// Renewing early keeps a request from carrying a JWT that expires while it's in flight,
/// and absorbs a moderate clock difference with the server.
const RENEW_BEFORE_EXPIRATION: chrono::TimeDelta = chrono::TimeDelta::seconds(60);

/// A call to the Auth API holds back the request that needs it, and maybe others queued behind it,
/// so it gets a bound of its own instead of relying on the (possibly absent) deadline of that request.
const AUTH_CALL_TIMEOUT: Duration = Duration::from_secs(5);

/// After a failed call to the Auth API the next one waits for this long.
/// Otherwise, when the Auth API is down or the token is rejected, every request of a busy client would
/// make its own attempt, one after another.
const RETRY_AFTER_FAILURE: Duration = Duration::from_secs(5);

/// A plain (non-authenticated) channel to call the Auth API through
pub(crate) trait AuthChannel:
    GrpcService<
        Body,
        Error: Into<StdError>,
        Future: Send,
        ResponseBody: http_body::Body<Data = Bytes, Error: Into<StdError> + Send> + Send + 'static,
    > + Clone + Send + 'static
{
}

impl<S> AuthChannel for S where
    S: GrpcService<
        Body,
        Error: Into<StdError>,
        Future: Send,
        ResponseBody: http_body::Body<Data = Bytes, Error: Into<StdError> + Send> + Send + 'static,
    > + Clone + Send + 'static
{
}

/// The credentials state shared by all clients of one connection.
///
/// All of them must share the same JWT, and only one of them may renew it at a time:
/// with each one doing that on its own, a burst of requests at the moment of expiration would each spend
/// the same refresh token, and all but the first of them would fail.
pub(crate) struct AuthSession {
    credentials: RwLock<Credentials>,
    /// Held while calling the Auth API. Keeps the last failure of that call,
    /// so the following attempts can respect [`RETRY_AFTER_FAILURE`].
    renewal: Arc<tokio::sync::Mutex<Option<Failure>>>,
    /// When the server last rejected a JWT that was valid by its expiration time
    last_rejection: Mutex<Option<Instant>>,
}

struct Failure {
    at: Instant,
    status: Status,
}

impl Failure {
    fn new(status: Status) -> Self {
        Self { at: Instant::now(), status }
    }

    fn is_recent(&self) -> bool {
        self.at.elapsed() < RETRY_AFTER_FAILURE
    }
}

/// What a request can do with the current credentials
pub(crate) enum Bearer {
    /// Unauthenticated connection, nothing to attach
    NotRequired,
    Ready(String),
    /// The JWT is still valid but is about to expire, so it's time to renew it
    Expiring(String),
    /// No valid JWT, a request cannot proceed until it gets one
    Missing,
}

impl AuthSession {
    pub(crate) fn new(credentials: Credentials) -> Self {
        Self {
            credentials: RwLock::new(credentials),
            renewal: Arc::new(tokio::sync::Mutex::new(None)),
            last_rejection: Mutex::new(None),
        }
    }

    /// The server rejected `jwt`, though it's not expired (e.g., it was revoked, or the server changed its keys).
    /// The session drops it, so the next request authenticates again instead of sending the same JWT until it expires.
    ///
    /// No more often than [`RETRY_AFTER_FAILURE`], so a server that rejects every new JWT as well
    /// doesn't get an authentication request with each call.
    pub(crate) fn rejected(&self, jwt: &str) {
        {
            let mut last_rejection = self.last_rejection.lock().unwrap();
            if last_rejection.is_some_and(|at| at.elapsed() < RETRY_AFTER_FAILURE) {
                return;
            }
            *last_rejection = Some(Instant::now());
        }
        let mut credentials = self.credentials.write().unwrap();
        let secret = match &*credentials {
            // Another request may have replaced it already
            Credentials::Token(JwtState::Authenticated { secret, jwt: current, .. }) if current == jwt => secret.clone(),
            _ => return,
        };
        tracing::warn!("The server rejected the JWT, authenticating again");
        *credentials = Credentials::Token(JwtState::Initial { secret });
    }

    pub(crate) fn credentials(&self) -> Credentials {
        self.credentials.read().unwrap().clone()
    }

    /// The JWT to attach to a request, authenticating or renewing it first if needed.
    pub(crate) async fn bearer<S: AuthChannel>(self: &Arc<Self>, channel: S) -> Result<Option<String>, Status> {
        match self.current() {
            Bearer::NotRequired => Ok(None),
            Bearer::Ready(jwt) => Ok(Some(jwt)),
            Bearer::Expiring(jwt) => {
                // The current JWT is still good for this request, so it shouldn't wait for a renewal
                // that another request is already doing, or for the Auth API that just failed.
                let Ok(last_failure) = Arc::clone(&self.renewal).try_lock_owned() else {
                    return Ok(Some(jwt));
                };
                if last_failure.as_ref().is_some_and(Failure::is_recent) {
                    return Ok(Some(jwt));
                }
                if let Bearer::Ready(renewed) = self.current() {
                    return Ok(Some(renewed));
                }
                // Even if it failed, the current JWT is valid for a bit longer
                Ok(Some(self.renew(last_failure, channel).await.unwrap_or(jwt)))
            }
            Bearer::Missing => {
                let last_failure = Arc::clone(&self.renewal).lock_owned().await;
                // Most likely, the request was waiting for the lock while another one was getting a JWT
                match self.current() {
                    Bearer::NotRequired => return Ok(None),
                    Bearer::Ready(jwt) => return Ok(Some(jwt)),
                    Bearer::Expiring(_) | Bearer::Missing => {}
                }
                if let Some(failure) = last_failure.as_ref().filter(|f| f.is_recent()) {
                    return Err(failure.status.clone());
                }
                self.renew(last_failure, channel).await.map(Some)
            }
        }
    }

    pub(crate) fn current(&self) -> Bearer {
        match &*self.credentials.read().unwrap() {
            Credentials::None => Bearer::NotRequired,
            Credentials::Token(JwtState::Initial { .. }) => Bearer::Missing,
            Credentials::Token(JwtState::Authenticated { jwt, expires_at, .. }) => {
                let now = Utc::now();
                if *expires_at <= now {
                    Bearer::Missing
                } else if *expires_at - RENEW_BEFORE_EXPIRATION <= now {
                    Bearer::Expiring(jwt.clone())
                } else {
                    Bearer::Ready(jwt.clone())
                }
            }
        }
    }

    /// Gets a new JWT and stores it for all following requests, or the failure to get it for the
    /// following attempts. The `renewal` lock is released only after that.
    ///
    /// Runs as a separate task, so the request that started it may give up waiting for it (e.g., by
    /// its deadline) without leaving the renewal unfinished for the others.
    async fn renew<S: AuthChannel>(
        self: &Arc<Self>,
        mut last_failure: OwnedMutexGuard<Option<Failure>>,
        channel: S,
    ) -> Result<String, Status> {
        let session = Arc::clone(self);
        tokio::spawn(async move {
            let result = session.exchange(channel).await;
            *last_failure = match &result {
                Ok(_) => None,
                Err(status) => {
                    tracing::warn!("Failed to get a JWT: {}", status);
                    Some(Failure::new(status.clone()))
                }
            };
            result
        })
        .await
        .map_err(|e| Status::internal(format!("JWT renewal failed: {}", e)))?
    }

    /// Gets a new JWT, by its refresh token if there is one.
    async fn exchange<S: AuthChannel>(&self, channel: S) -> Result<String, Status> {
        let state = match self.credentials() {
            Credentials::Token(state) => state,
            Credentials::None => return Err(Status::internal("No credentials to authenticate with")),
        };
        let renewed = match state {
            JwtState::Initial { secret } => authenticate(channel, secret).await?,
            JwtState::Authenticated { secret, refresh, .. } => {
                match refresh_jwt(channel.clone(), secret.clone(), refresh).await {
                    Ok(renewed) => renewed,
                    Err(status) => {
                        // The refresh token may have expired, or been revoked,
                        // while the secret token is still valid
                        tracing::debug!("Failed to refresh the JWT, authenticating again: {}", status);
                        authenticate(channel, secret).await?
                    }
                }
            }
        };
        let jwt = match &renewed {
            JwtState::Authenticated { jwt, .. } => jwt.clone(),
            JwtState::Initial { .. } => return Err(Status::unauthenticated("Invalid JWT received from the server")),
        };
        *self.credentials.write().unwrap() = Credentials::Token(renewed);
        Ok(jwt)
    }
}

async fn authenticate<S: AuthChannel>(channel: S, secret: String) -> Result<JwtState, Status> {
    tracing::trace!("Authenticating...");
    let mut client = AuthClient::new(channel);
    let request = AuthRequest {
        auth_secret: secret.clone(),
        ..Default::default()
    };
    let response = tokio::time::timeout(AUTH_CALL_TIMEOUT, client.authenticate(request))
        .await
        .map_err(|_| auth_timeout())??
        .into_inner();
    if response.status != 0 {
        return Err(Status::unauthenticated(format!("Authentication denied ({}): {}", response.status, response.deny_message)));
    }
    tracing::debug!("Authenticated with a new JWT, valid until {:?}", expiration(response.expires_at));
    Ok(JwtState::issued(secret, response))
}

async fn refresh_jwt<S: AuthChannel>(channel: S, secret: String, refresh_token: String) -> Result<JwtState, Status> {
    tracing::trace!("Refreshing the JWT...");
    let mut client = AuthClient::new(channel);
    let request = RefreshRequest {
        auth_secret: secret.clone(),
        refresh_token,
    };
    let response = tokio::time::timeout(AUTH_CALL_TIMEOUT, client.refresh(request))
        .await
        .map_err(|_| auth_timeout())??
        .into_inner();
    if response.status != 0 {
        return Err(Status::unauthenticated(format!("Refresh denied ({}): {}", response.status, response.deny_message)));
    }
    tracing::debug!("Refreshed the JWT, valid until {:?}", expiration(response.expires_at));
    Ok(JwtState::issued(secret, response))
}

fn auth_timeout() -> Status {
    Status::deadline_exceeded(format!("Auth API didn't respond in {}s", AUTH_CALL_TIMEOUT.as_secs()))
}

fn expiration(expires_at: u64) -> Option<DateTime<Utc>> {
    DateTime::from_timestamp_millis(expires_at as i64)
}
