use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use std::task::{Context, Poll};
use futures::future::BoxFuture;
use tonic::{
    body::Body,
    codegen::{http, StdError},
};
use tower::{Layer, Service};
use crate::creds::session::{AuthChannel, Bearer};
use crate::creds::AuthSession;

/// A gRPC channel that authenticates each call with the credentials of its connection.
///
/// Cheap to clone; all the clones share the credentials of the connection they were made for.
pub struct AuthService<S> {
    inner: S,
    session: Arc<AuthSession>,
}

impl<S: Clone> Clone for AuthService<S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            session: Arc::clone(&self.session),
        }
    }
}

/// Keeps the clients (which are `Debug`) printable, without printing the credentials
impl<S: Debug> Debug for AuthService<S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthService")
            .field("inner", &self.inner)
            .finish_non_exhaustive()
    }
}

impl<S> Service<http::Request<Body>> for AuthService<S>
where
    S: AuthChannel,
    S::Future: 'static,
{
    type Response = http::Response<S::ResponseBody>;
    // Passes the original error as it is, because Tonic finds out the gRPC code of a failed call
    // by its type. I.e., a rejected token fails the call as `Unauthenticated`,
    // and a lost connection as `Unavailable`, instead of an `Unknown` for both.
    type Error = StdError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, mut req: http::Request<Body>) -> Self::Future {
        let session = Arc::clone(&self.session);

        // This is necessary because tonic internally uses `tower::buffer::Buffer`.
        // See https://github.com/tower-rs/tower/issues/547#issuecomment-767629149
        // for details on why this is necessary
        let inner_clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, inner_clone);

        Box::pin(async move {
            let jwt = match session.current() {
                Bearer::NotRequired => None,
                Bearer::Ready(jwt) => Some(jwt),
                Bearer::Expiring(_) | Bearer::Missing => {
                    // Getting a JWT may wait for the Auth API, or for another request calling it.
                    // Meanwhile, this request must not keep the slot reserved for it in the channel buffer by `poll_ready`,
                    // otherwise enough waiting requests take all of them, and the Auth API call is left without one.
                    let channel = inner.clone();
                    drop(inner);
                    let jwt = session.bearer(channel.clone()).await?;
                    inner = channel;
                    futures::future::poll_fn(|cx| inner.poll_ready(cx)).await.map_err(Into::into)?;
                    jwt
                }
            };
            if let Some(jwt) = &jwt {
                let value = format!("Bearer {}", jwt)
                    .parse()
                    .map_err(|_| tonic::Status::unauthenticated("JWT is not a valid header value"))?;
                req.headers_mut().insert(http::header::AUTHORIZATION, value);
            }
            let response = inner.call(req).await.map_err(Into::into)?;
            if let Some(jwt) = &jwt {
                if is_unauthenticated(&response) {
                    session.rejected(jwt);
                }
            }
            Ok(response)
        })
    }
}

/// A server rejects a call before answering anything, so the gRPC status comes in the headers
/// (a "trailers-only" response), rather than in the trailers after the body.
fn is_unauthenticated<B>(response: &http::Response<B>) -> bool {
    response
        .headers()
        .get("grpc-status")
        .and_then(|status| status.to_str().ok()?.parse::<i32>().ok())
        .is_some_and(|code| code == tonic::Code::Unauthenticated as i32)
}

///
/// An Authentication Layer for the Tokio Tower
pub(crate) struct AuthLayer {
    session: Arc<AuthSession>,
}

impl AuthLayer {
    pub fn new(session: Arc<AuthSession>) -> Self {
        AuthLayer {
            session
        }
    }
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthService<S>;

    fn layer(&self, service: S) -> Self::Service {
        AuthService {
            inner: service,
            session: Arc::clone(&self.session),
        }
    }
}
