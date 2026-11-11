#[cfg(all(feature = "client-auth", feature = "server-auth"))]
mod on_mock {
    use chrono::Utc;
    use emerald_api::{
        auth::connect,
        conn::EmeraldConn,
        creds::{Credentials, JwtState},
        proto::auth::{
            auth_server::{Auth, AuthServer},
            AuthRequest, AuthResponse, DeleteTokenRequest, DeleteTokenResponse, IssueTokenRequest,
            IssuedTokenResponse, ListTokensRequest, ListTokensResponse, RefreshRequest,
            UpdateTokenRequest, WhoAmIRequest, WhoAmIResponse,
        },
    };
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tokio_stream::wrappers::TcpListenerStream;
    use tonic::{transport::Server, Code, Request, Response, Status};

    const SECRET: &str = "secret_token";

    /// An Auth API that issues `jwt_N`/`refresh_N` pairs, and records what the client sent to it
    struct MockAuth {
        /// How long an issued JWT lives
        jwt_ttl: chrono::Duration,
        /// Issues the first JWT already expired
        first_expired: bool,
        /// Makes the overlap of concurrent requests certain
        auth_delay: Duration,
        accept_auth: AtomicBool,
        accept_refresh: AtomicBool,
        issued: AtomicUsize,
        auth_calls: AtomicUsize,
        refresh_calls: Mutex<Vec<RefreshRequest>>,
        /// The `authorization` header of each WhoAmI call
        bearers: Mutex<Vec<String>>,
        /// A JWT that WhoAmI rejects, though it's not expired
        revoked: Mutex<Option<String>>,
    }

    impl MockAuth {
        fn new(jwt_ttl: chrono::Duration) -> Self {
            Self {
                jwt_ttl,
                first_expired: false,
                auth_delay: Duration::ZERO,
                accept_auth: AtomicBool::new(true),
                accept_refresh: AtomicBool::new(true),
                issued: AtomicUsize::new(0),
                auth_calls: AtomicUsize::new(0),
                refresh_calls: Mutex::new(Vec::new()),
                bearers: Mutex::new(Vec::new()),
                revoked: Mutex::new(None),
            }
        }

        fn issue(&self) -> AuthResponse {
            let n = self.issued.fetch_add(1, Ordering::SeqCst) + 1;
            let ttl = if n == 1 && self.first_expired {
                chrono::Duration::seconds(-1)
            } else {
                self.jwt_ttl
            };
            AuthResponse {
                status: 0,
                access_token: format!("jwt_{n}"),
                refresh_token: format!("refresh_{n}"),
                expires_at: (Utc::now() + ttl).timestamp_millis() as u64,
                ..Default::default()
            }
        }

        fn auth_calls(&self) -> usize {
            self.auth_calls.load(Ordering::SeqCst)
        }

        fn refresh_calls(&self) -> Vec<RefreshRequest> {
            self.refresh_calls.lock().unwrap().clone()
        }

        fn bearers(&self) -> Vec<String> {
            self.bearers.lock().unwrap().clone()
        }
    }

    #[tonic::async_trait]
    impl Auth for MockAuth {
        async fn authenticate(&self, request: Request<AuthRequest>) -> Result<Response<AuthResponse>, Status> {
            self.auth_calls.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(self.auth_delay).await;
            if !self.accept_auth.load(Ordering::SeqCst) || request.into_inner().auth_secret != SECRET {
                return Ok(Response::new(AuthResponse {
                    status: 401,
                    deny_message: "invalid token".to_string(),
                    ..Default::default()
                }));
            }
            Ok(Response::new(self.issue()))
        }

        async fn refresh(&self, request: Request<RefreshRequest>) -> Result<Response<AuthResponse>, Status> {
            let request = request.into_inner();
            self.refresh_calls.lock().unwrap().push(request.clone());
            tokio::time::sleep(self.auth_delay).await;
            let expected = format!("refresh_{}", self.issued.load(Ordering::SeqCst));
            if !self.accept_refresh.load(Ordering::SeqCst) || request.auth_secret != SECRET || request.refresh_token != expected {
                return Err(Status::unauthenticated("invalid refresh token"));
            }
            Ok(Response::new(self.issue()))
        }

        async fn who_am_i(&self, request: Request<WhoAmIRequest>) -> Result<Response<WhoAmIResponse>, Status> {
            let bearer = request
                .metadata()
                .get("authorization")
                .map(|v| v.to_str().unwrap().to_string())
                .unwrap_or_default();
            let is_authenticated = !bearer.is_empty();
            self.bearers.lock().unwrap().push(bearer.clone());
            if let Some(revoked) = self.revoked.lock().unwrap().as_ref() {
                if bearer == format!("Bearer {revoked}") {
                    return Err(Status::unauthenticated("revoked"));
                }
            }
            Ok(Response::new(WhoAmIResponse {
                is_authenticated,
                ..Default::default()
            }))
        }

        async fn issue_token(&self, _: Request<IssueTokenRequest>) -> Result<Response<IssuedTokenResponse>, Status> {
            Err(Status::unimplemented("mock"))
        }

        async fn update_token(&self, _: Request<UpdateTokenRequest>) -> Result<Response<IssuedTokenResponse>, Status> {
            Err(Status::unimplemented("mock"))
        }

        async fn list_tokens(&self, _: Request<ListTokensRequest>) -> Result<Response<ListTokensResponse>, Status> {
            Err(Status::unimplemented("mock"))
        }

        async fn delete_token(&self, _: Request<DeleteTokenRequest>) -> Result<Response<DeleteTokenResponse>, Status> {
            Err(Status::unimplemented("mock"))
        }
    }

    async fn start(mock: Arc<MockAuth>, credentials: Credentials) -> EmeraldConn {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(
            Server::builder()
                .add_service(AuthServer::from_arc(mock))
                .serve_with_incoming(TcpListenerStream::new(listener)),
        );
        let channel = tonic::transport::Channel::from_shared(format!("http://{addr}"))
            .unwrap()
            .connect_lazy();
        EmeraldConn::new(channel, credentials)
    }

    fn current_jwt(conn: &EmeraldConn) -> String {
        match conn.get_credentials() {
            Credentials::Token(JwtState::Authenticated { jwt, .. }) => jwt,
            other => panic!("Not authenticated: {:?}", other),
        }
    }

    #[tokio::test]
    async fn authenticates_once_and_reuses_the_jwt() {
        let mock = Arc::new(MockAuth::new(chrono::Duration::hours(1)));
        let conn = start(mock.clone(), Credentials::token(SECRET)).await;
        let mut client = connect(&conn);

        client.who_am_i(WhoAmIRequest {}).await.unwrap();
        client.who_am_i(WhoAmIRequest {}).await.unwrap();

        assert_eq!(mock.auth_calls(), 1);
        assert_eq!(mock.bearers(), vec!["Bearer jwt_1", "Bearer jwt_1"]);
        assert_eq!(current_jwt(&conn), "jwt_1");
    }

    #[tokio::test]
    async fn unauthenticated_connection_sends_no_header() {
        let mock = Arc::new(MockAuth::new(chrono::Duration::hours(1)));
        let conn = start(mock.clone(), Credentials::unauthenticated()).await;

        connect(&conn).who_am_i(WhoAmIRequest {}).await.unwrap();

        assert_eq!(mock.auth_calls(), 0);
        assert_eq!(mock.bearers(), vec![""]);
    }

    #[tokio::test]
    async fn refreshes_the_jwt_before_it_expires() {
        // Valid for a while, but already within the renewal margin
        let mock = Arc::new(MockAuth::new(chrono::Duration::seconds(30)));
        let conn = start(mock.clone(), Credentials::token(SECRET)).await;
        let mut client = connect(&conn);

        client.who_am_i(WhoAmIRequest {}).await.unwrap();
        client.who_am_i(WhoAmIRequest {}).await.unwrap();

        let refreshes = mock.refresh_calls();
        assert_eq!(refreshes.len(), 1);
        assert_eq!(refreshes[0].auth_secret, SECRET);
        assert_eq!(refreshes[0].refresh_token, "refresh_1");
        assert_eq!(mock.auth_calls(), 1);
        assert_eq!(mock.bearers(), vec!["Bearer jwt_1", "Bearer jwt_2"]);
    }

    #[tokio::test]
    async fn authenticates_again_when_refresh_is_rejected() {
        let mock = Arc::new(MockAuth::new(chrono::Duration::seconds(30)));
        let conn = start(mock.clone(), Credentials::token(SECRET)).await;
        let mut client = connect(&conn);
        client.who_am_i(WhoAmIRequest {}).await.unwrap();

        mock.accept_refresh.store(false, Ordering::SeqCst);
        client.who_am_i(WhoAmIRequest {}).await.unwrap();

        assert_eq!(mock.refresh_calls().len(), 1);
        assert_eq!(mock.auth_calls(), 2);
        assert_eq!(mock.bearers(), vec!["Bearer jwt_1", "Bearer jwt_2"]);
    }

    #[tokio::test]
    async fn keeps_the_valid_jwt_when_renewal_fails() {
        let mock = Arc::new(MockAuth::new(chrono::Duration::seconds(30)));
        let conn = start(mock.clone(), Credentials::token(SECRET)).await;
        let mut client = connect(&conn);
        client.who_am_i(WhoAmIRequest {}).await.unwrap();

        mock.accept_refresh.store(false, Ordering::SeqCst);
        mock.accept_auth.store(false, Ordering::SeqCst);
        client.who_am_i(WhoAmIRequest {}).await.unwrap();
        // Doesn't retry the Auth API right after a failure
        client.who_am_i(WhoAmIRequest {}).await.unwrap();

        assert_eq!(mock.refresh_calls().len(), 1);
        assert_eq!(mock.auth_calls(), 2);
        assert_eq!(mock.bearers(), vec!["Bearer jwt_1", "Bearer jwt_1", "Bearer jwt_1"]);
    }

    #[tokio::test]
    async fn concurrent_calls_authenticate_once() {
        let mock = Arc::new(MockAuth {
            auth_delay: Duration::from_millis(200),
            ..MockAuth::new(chrono::Duration::hours(1))
        });
        let conn = start(mock.clone(), Credentials::token(SECRET)).await;

        let calls = (0..20).map(|_| {
            let mut client = connect(&conn);
            tokio::spawn(async move { client.who_am_i(WhoAmIRequest {}).await })
        });
        for call in futures::future::join_all(calls).await {
            call.unwrap().unwrap();
        }

        assert_eq!(mock.auth_calls(), 1);
        assert!(mock.bearers().iter().all(|b| b == "Bearer jwt_1"));
    }

    #[tokio::test]
    async fn waiting_calls_leave_the_channel_to_the_auth() {
        let mock = Arc::new(MockAuth {
            first_expired: true,
            auth_delay: Duration::from_millis(200),
            ..MockAuth::new(chrono::Duration::hours(1))
        });
        let conn = start(mock.clone(), Credentials::token(SECRET)).await;
        connect(&conn).who_am_i(WhoAmIRequest {}).await.unwrap();
        mock.accept_refresh.store(false, Ordering::SeqCst);

        // While the refresh is in progress, the calls fill the channel buffer (1024 by default),
        // and the following login must still be able to go through it.
        let calls = (0..2000).map(|_| {
            let mut client = connect(&conn);
            tokio::spawn(async move { client.who_am_i(WhoAmIRequest {}).await })
        });
        for call in futures::future::join_all(calls).await {
            call.unwrap().unwrap();
        }

        assert_eq!(mock.refresh_calls().len(), 1);
        assert_eq!(mock.auth_calls(), 2);
    }

    #[tokio::test]
    async fn authenticates_again_when_server_rejects_a_valid_jwt() {
        let mock = Arc::new(MockAuth::new(chrono::Duration::hours(1)));
        let conn = start(mock.clone(), Credentials::token(SECRET)).await;
        let mut client = connect(&conn);
        client.who_am_i(WhoAmIRequest {}).await.unwrap();

        *mock.revoked.lock().unwrap() = Some("jwt_1".to_string());
        let status = client.who_am_i(WhoAmIRequest {}).await.unwrap_err();
        assert_eq!(status.code(), Code::Unauthenticated);
        client.who_am_i(WhoAmIRequest {}).await.unwrap();

        assert_eq!(mock.auth_calls(), 2);
        assert_eq!(mock.bearers(), vec!["Bearer jwt_1", "Bearer jwt_1", "Bearer jwt_2"]);
    }

    #[tokio::test]
    async fn rejected_token_fails_the_call_as_unauthenticated() {
        let mock = Arc::new(MockAuth::new(chrono::Duration::hours(1)));
        let conn = start(mock.clone(), Credentials::token("wrong_token")).await;
        let mut client = connect(&conn);

        let status = client.who_am_i(WhoAmIRequest {}).await.unwrap_err();
        assert_eq!(status.code(), Code::Unauthenticated);
        assert!(status.message().contains("invalid token"), "{}", status.message());

        // The next call fails right away, without asking the Auth API again
        let status = client.who_am_i(WhoAmIRequest {}).await.unwrap_err();
        assert_eq!(status.code(), Code::Unauthenticated);
        assert_eq!(mock.auth_calls(), 1);
        assert!(mock.bearers().is_empty());
    }

    #[tokio::test]
    async fn lost_connection_fails_the_call_as_unavailable() {
        // Nothing listens on the port
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        let channel = tonic::transport::Channel::from_shared(format!("http://{addr}"))
            .unwrap()
            .connect_lazy();
        let conn = EmeraldConn::new(channel, Credentials::unauthenticated());

        let status = connect(&conn).who_am_i(WhoAmIRequest {}).await.unwrap_err();
        assert_eq!(status.code(), Code::Unavailable);
    }
}
