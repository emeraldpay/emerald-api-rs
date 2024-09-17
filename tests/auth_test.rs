#[cfg(all(feature = "client-auth", feature = "server-auth"))]
mod on_mock {
    use emerald_api::{
        conn::EmeraldConn,
        auth::connect,
        creds::{Credentials, JwtState},
        proto::auth::{
            auth_server::Auth, AuthRequest, AuthResponse,
            IssueTokenRequest, IssuedTokenResponse, ListTokensRequest, ListTokensResponse, RefreshRequest, WhoAmIRequest, WhoAmIResponse
        },
    };
    use tonic::{transport::Server, Request, Response, Status};
    use std::net::SocketAddr;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct MockAuthService {
        responses: Vec<AuthResponse>,
        request_count: Arc<AtomicUsize>,
    }

    #[tonic::async_trait]
    impl Auth for MockAuthService {
        async fn authenticate(&self, request: Request<AuthRequest>) -> Result<Response<AuthResponse>, Status> {
            self.request_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            println!("Received request: {:?}", request);
            let auth_request = request.into_inner();
            if let Some(auth_type) = auth_request.auth_type {
                if let emerald_api::proto::auth::auth_request::AuthType::AuthSecret(secret) = auth_type {
                    if secret == "secret_token" {
                        Ok(Response::new(self.responses[0].clone()))
                    } else {
                        Err(Status::unauthenticated("Invalid secret token"))
                    }
                } else {
                    Err(Status::invalid_argument("Invalid auth type"))
                }
            } else {
                Err(Status::invalid_argument("Missing auth type"))
            }
        }

        async fn refresh(&self, _request: Request<RefreshRequest>) -> Result<Response<AuthResponse>, Status> {
            self.request_count.fetch_add(1, Ordering::Relaxed);
            todo!()
        }

        async fn issue_token(&self, _request: Request<IssueTokenRequest>) -> Result<Response<IssuedTokenResponse>, Status> {
            self.request_count.fetch_add(1, Ordering::Relaxed);
            todo!()
        }

        async fn who_am_i(&self, request: Request<WhoAmIRequest>) -> Result<Response<WhoAmIResponse>, Status> {
            self.request_count.fetch_add(1, Ordering::Relaxed);
            println!("Received request: {:?}", request);
            Ok(Response::new(WhoAmIResponse {
                is_authenticated: true,
                user_id: "user_001".to_string(),
                ..Default::default()
            }))
        }

        async fn list_tokens(&self, _request: Request<ListTokensRequest>) -> Result<Response<ListTokensResponse>, Status> {
            self.request_count.fetch_add(1, Ordering::Relaxed);
            todo!()
        }
    }

    #[tokio::test]
    async fn test_authentication() {
        // Setup mock server
        let addr: SocketAddr = "127.0.0.1:9091".parse().unwrap();
        let request_count = Arc::new(AtomicUsize::new(0));
        let mock_service = MockAuthService {
            request_count: request_count.clone(),
            responses: vec![AuthResponse {
                status: 0,
                access_token: "jwt_001".to_string(),
                refresh_token: "refresh_001".to_string(),
                expires_at: 1620000000000, // Some fixed timestamp
                ..Default::default()
            }],
        };


        tokio::spawn(async move {
            let serve_future = Server::builder()
                .add_service(emerald_api::proto::auth::auth_server::AuthServer::new(mock_service))
                .serve(addr)
                .await;
            if let Err(e) = serve_future {
                eprintln!("Failed to start server: {}", e);
            } else {
                eprintln!("Server stopped");
            }
        });

        println!("Waiting for server to start...");

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        let channel = tonic::transport::Channel::from_static("http://127.0.0.1:9091")
            .connect()
            .await
            .unwrap();

        let credentials = Credentials::token("secret_token");
        let conn = EmeraldConn::new(channel, credentials);

        let mut auth_client = connect(&conn);

        let me = auth_client.who_am_i(WhoAmIRequest {}).await.unwrap();

        match conn.get_credentials() {
            Credentials::None => {
                panic!("No credential");
            }
            Credentials::Token(jwt_state) => {
                match jwt_state {
                    JwtState::Initial { .. } => {
                        panic!("Still the initial state");
                    }
                    JwtState::Authenticated { jwt, refresh } => {
                        assert_eq!(jwt, "jwt_001");
                        assert_eq!(refresh, "refresh_001");
                    }
                }
            }
        }

        // auth + who_am_i
        assert_eq!(request_count.load(Ordering::Relaxed), 2);

        let me = me.into_inner();
        assert_eq!(me.is_authenticated, true);
        assert_eq!(me.user_id, "user_001");

        let me_2 = auth_client.who_am_i(WhoAmIRequest {}).await.unwrap();
        // + another who_am_i, but no auth at this time
        assert_eq!(request_count.load(Ordering::Relaxed), 3);

        let me_2 = me_2.into_inner();
        assert_eq!(me_2.is_authenticated, true);
        assert_eq!(me_2.user_id, "user_001");

    }
}

