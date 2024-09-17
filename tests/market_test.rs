#[cfg(test)]
#[cfg(feature = "client-market")]
mod on_prod {
    use tonic::Response;
    use emerald_api::conn::EmeraldConn;
    use emerald_api::creds::Credentials;
    use emerald_api::market::connect;
    use emerald_api::proto::market::{GetRatesRequest, GetRatesResponse, Pair};
    use emerald_api::proto::market::pair::{BaseType, TargetType};

    #[tokio::test]
    async fn read_usd_rate() {
        let conn = EmeraldConn::connect(
            Credentials::unauthenticated()
        );
        let mut client = connect(&conn);
        
        let rates: Response<GetRatesResponse> = client.get_rates(
            GetRatesRequest {
                pairs: vec![
                    Pair {
                        base_type: Some(BaseType::Base("ETH".to_string())),
                        target_type: Some(TargetType::Target("USD".to_string())),
                    },
                    Pair {
                        base_type: Some(BaseType::Base("BTC".to_string())),
                        target_type: Some(TargetType::Target("USD".to_string())),
                    },
                ],
            }
        ).await.expect("rates not received");

        let rates = rates.into_inner().rates;

        assert_eq!(rates.len(), 2);

        println!("Rates: {:?}", rates);
    }
}