pub mod adapter;
pub mod controller;
pub mod controller_oracle;
pub mod controller_relayer;
pub mod coordinator;
pub mod provider;

#[cfg(test)]
pub mod contract_interaction_tests {

    use arpa_core::eip1559_gas_price_estimator;
    use ethers::providers::{Http, Middleware, Provider};

    #[tokio::test]
    async fn test_estimate_eip1559_fees() -> Result<(), anyhow::Error> {
        let provider = Provider::<Http>::try_from("https://eth.llamarpc.com")
            .expect("could not instantiate HTTP Provider");

        let (max_fee, max_priority_fee) = provider
            .estimate_eip1559_fees(Some(eip1559_gas_price_estimator))
            .await?;
        println!("max_fee: {:?}", max_fee);
        println!("max_priority_fee: {:?}", max_priority_fee);

        Ok(())
    }
}
