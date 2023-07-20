#[cfg(test)]
#[cfg(feature = "client-market")]
mod on_prod {
    use tonic::Response;
    use emerald_api::conn::EmeraldConn;
    use emerald_api::creds::Credentials;
    use emerald_api::market::connect;
    use emerald_api::proto::market::{GetRatesRequest, GetRatesResponse, Pair};

    #[tokio::test]
    async fn read_usd_rate() {
        let conn = EmeraldConn::new(
            EmeraldConn::standard_api(), Credentials::unauthneticated()
        );
        let mut client = connect(&conn);
        
        let rates: Response<GetRatesResponse> = client.get_rates(
            GetRatesRequest {
                pairs: vec![
                    Pair {
                        base: "ETH".to_string(),
                        target: "USD".to_string(),
                    },
                    Pair {
                        base: "BTC".to_string(),
                        target: "USD".to_string(),
                    },
                ],
            }
        ).await.expect("rates not received");

        let rates = rates.into_inner().rates;

        assert_eq!(rates.len(), 2);

        println!("Rates: {:?}", rates);
    }
}