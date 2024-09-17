use std::sync::{Arc, RwLock};
use tonic::{
    Status,
    client::GrpcService,
    codegen::{http, StdError},
    transport::{Body},
    body::BoxBody
};
use http::Response;
use std::task::{Context, Poll};
use tower::{Service, Layer};
use futures::future::BoxFuture;
use crate::errors::Error;
use bytes::Bytes;
use crate::proto::auth::{auth_client, auth_request, AuthRequest};

#[derive(Debug, Clone)]
pub enum Credentials {
    None,
    Token(JwtState),
}

#[derive(Debug, Clone)]
pub enum JwtState {
    Initial{
        secret: String
    },
    Authenticated {
        jwt: String,
        refresh: String,
    }
}

impl JwtState {
    pub fn new(secret: String) -> Self {
        JwtState::Initial {
            secret
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
    /// Do nothing
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

pub struct AuthService<S> {
    inner: S,
    credentials: Arc<RwLock<Credentials>>,
}

impl<S> Service<http::Request<BoxBody>> for AuthService<S>
where
    S: GrpcService<BoxBody> + Send + 'static + Clone,
    S::Future: Send + 'static,
    S::Error: Into<StdError> + Send,
    S::ResponseBody: Body<Data = Bytes> + Send + 'static,
    <S::ResponseBody as Body>::Error: Into<StdError> + Send,
{
    type Response = Response<S::ResponseBody>;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: http::Request<BoxBody>) -> Self::Future {
        let credentials_global = self.credentials.clone();

        // This is necessary because tonic internally uses `tower::buffer::Buffer`.
        // See https://github.com/tower-rs/tower/issues/547#issuecomment-767629149
        // for details on why this is necessary
        let inner_clone = self.inner.clone();
        // let auth_client = auth_client::AuthClient::new(clone.clone());
        let mut inner = std::mem::replace(&mut self.inner, inner_clone);

        Box::pin(async move {
            // it makes a copy of the credentials to verify if it can be used or should authenticate
            // but if it needs an authentication (or refresh) it may cause a concurrent request because it's not locked for write
            // usually it needs just one auth request at the start and it's not a big problem it if fetched it twice from the server,
            // but with a heavy-load app it may cause some performance issues
            // TODO use a tokio lock with write lock on update, to avoid this
            let credentials = credentials_global.read().unwrap().clone();
            match credentials {
                Credentials::None => inner.call(req).await,
                Credentials::Token(jwt_state) => {
                    match jwt_state {
                        JwtState::Initial { secret } => {
                            // Authenticate and get JWT token
                            let client = auth_client::AuthClient::new(inner.clone());
                            let jwt = Self::authenticate(&secret, client).await;
                            if let Ok(jwt) = jwt {
                                {
                                    // write the received JWT token to the global credentials to it can be reused by other requests
                                    let mut credentials = credentials_global.write().unwrap();
                                    *credentials = Credentials::Token(jwt.clone());
                                }
                                if let JwtState::Authenticated { jwt, .. } = &jwt {
                                    Self::add_auth_header(&mut req, jwt);
                                } else {
                                    todo!("Handle errors");
                                }
                            } else {
                                eprintln!("Error: {:?}", jwt);
                                todo!("Handle errors");
                            }
                            inner.call(req).await
                        }
                        JwtState::Authenticated { jwt, .. } => {
                            Self::add_auth_header(&mut req, &jwt);
                            inner.call(req).await
                        }
                    }
                }
            }
        })
    }

}

impl<S> AuthService<S>
where
    S: GrpcService<BoxBody> + Send + 'static + Clone,
    S::Future: Send + 'static,
    S::Error: Into<StdError> + Send,
    S::ResponseBody: Body<Data = Bytes> + Send + 'static,
    <S::ResponseBody as Body>::Error: Into<StdError> + Send, {

    fn add_auth_header(req: &mut http::Request<BoxBody>, jwt: &str) {
        req.headers_mut().insert(
            "authorization",
            format!("Bearer {}", jwt).parse().unwrap(),
        );
    }

    async fn authenticate(token: &String, mut client: auth_client::AuthClient<S>) -> Result<JwtState, Status> {
        tracing::trace!("Authenticating...");

        let request = tonic::Request::new(AuthRequest {
            auth_type: Some(auth_request::AuthType::AuthSecret(token.clone())),
            ..Default::default()
        });

        let response = client.authenticate(request).await?;
        let response = response.into_inner();

        if response.status != 0 {
            return Err(Status::unauthenticated(format!("Status: {}", response.status)));
        }

        tracing::trace!("Authenticated with JWT");

        Ok(JwtState::Authenticated {
            jwt: response.access_token.clone(),
            refresh: response.refresh_token.clone(),
        })
    }
}

///
/// An Authentication Layer for the Tokio Tower
pub(crate) struct AuthLayer {
    credentials: Arc<RwLock<Credentials>>,
}

impl AuthLayer {
    pub fn new(credentials: Arc<RwLock<Credentials>>) -> Self {
        AuthLayer {
            credentials
        }
    }
}

impl<S> Layer<S> for AuthLayer
where
    S: GrpcService<BoxBody>,
    S::Error: Into<Error>,
    S::ResponseBody: Body<Data = Bytes> + Send + 'static,
    <S::ResponseBody as Body>::Error: Into<Error> + Send,
{

    type Service = AuthService<S>;

    fn layer(&self, service: S) -> Self::Service {
        AuthService {
            inner: service,
            credentials: self.credentials.clone(),
        }
    }
}

