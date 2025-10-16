use crate::{
    coordinator::{
        CoordinatorClientBuilder, CoordinatorTransactions, CoordinatorViews, DKGContractError,
    },
    error::ContractClientResult,
    ethers::coordinator::Coordinator::CoordinatorInstance,
    ServiceClient, TransactionCaller, ViewCaller,
};
use ::core::panic;
use alloy::{primitives::Address, rpc::types::TransactionReceipt, sol};
use arpa_core::{
    ChainIdentity, ExponentialBackoffRetryDescriptor, GeneralMainChainIdentity,
    GeneralRelayedChainIdentity, ProviderClientWithSigner,
};
use async_trait::async_trait;
use dkg_core::{
    primitives::{BundledJustification, BundledResponses, BundledShares},
    BoardPublisher,
};
use log::info;
use threshold_bls::group::Curve;

sol! {
    #[sol(ignore_unlinked)]
    #[sol(rpc)]
    Coordinator,
    "abi/Coordinator.json"
}

pub struct CoordinatorClient {
    chain_id: u64,
    coordinator_address: Address,
    client: ProviderClientWithSigner,
    contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
    contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
    max_priority_fee_per_gas: Option<u128>,
}

impl CoordinatorClient {
    pub fn new(
        chain_id: u64,
        coordinator_address: Address,
        identity: &GeneralMainChainIdentity,
        contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
        contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
        max_priority_fee_per_gas: Option<u128>,
    ) -> Self {
        CoordinatorClient {
            chain_id,
            coordinator_address,
            client: identity.get_client(),
            contract_transaction_retry_descriptor,
            contract_view_retry_descriptor,
            max_priority_fee_per_gas,
        }
    }
}

impl<C: Curve + 'static> CoordinatorClientBuilder<C> for GeneralMainChainIdentity {
    type CoordinatorService = CoordinatorClient;

    fn build_coordinator_client(&self, contract_address: Address) -> CoordinatorClient {
        CoordinatorClient::new(
            self.get_chain_id(),
            contract_address,
            self,
            self.get_contract_transaction_retry_descriptor(),
            self.get_contract_view_retry_descriptor(),
            self.get_max_priority_fee_per_gas(),
        )
    }
}

impl<C: Curve + 'static> CoordinatorClientBuilder<C> for GeneralRelayedChainIdentity {
    type CoordinatorService = CoordinatorClient;

    fn build_coordinator_client(&self, _contract_address: Address) -> CoordinatorClient {
        panic!("not implemented")
    }
}

type CoordinatorContract = CoordinatorInstance<ProviderClientWithSigner>;

#[async_trait]
impl ServiceClient<CoordinatorContract> for CoordinatorClient {
    async fn prepare_service_client(&self) -> ContractClientResult<CoordinatorContract> {
        let coordinator_contract = Coordinator::new(self.coordinator_address, self.client.clone());

        Ok(coordinator_contract)
    }
}

#[async_trait]
impl TransactionCaller for CoordinatorClient {}

#[async_trait]
impl ViewCaller for CoordinatorClient {}

#[async_trait]
impl CoordinatorTransactions for CoordinatorClient {
    async fn publish(&self, value: Vec<u8>) -> ContractClientResult<TransactionReceipt> {
        let coordinator_contract =
            ServiceClient::<CoordinatorContract>::prepare_service_client(self).await?;

        let call = coordinator_contract.publish(value.into());

        CoordinatorClient::call_contract_transaction(
            self.chain_id,
            "publish",
            coordinator_contract.provider(),
            call,
            self.contract_transaction_retry_descriptor,
            true,
            self.max_priority_fee_per_gas,
        )
        .await
    }
}

#[async_trait]
impl CoordinatorViews for CoordinatorClient {
    async fn get_shares(&self) -> ContractClientResult<Vec<Vec<u8>>> {
        let coordinator_contract =
            ServiceClient::<CoordinatorContract>::prepare_service_client(self).await?;

        CoordinatorClient::call_contract_view(
            self.chain_id,
            "get_shares",
            coordinator_contract.getShares(),
            self.contract_view_retry_descriptor,
        )
        .await
        .map(|r| r.iter().map(|b| b.to_vec()).collect::<Vec<Vec<u8>>>())
    }

    async fn get_responses(&self) -> ContractClientResult<Vec<Vec<u8>>> {
        let coordinator_contract =
            ServiceClient::<CoordinatorContract>::prepare_service_client(self).await?;

        CoordinatorClient::call_contract_view(
            self.chain_id,
            "get_responses",
            coordinator_contract.getResponses(),
            self.contract_view_retry_descriptor,
        )
        .await
        .map(|r| r.iter().map(|b| b.to_vec()).collect::<Vec<Vec<u8>>>())
    }

    async fn get_justifications(&self) -> ContractClientResult<Vec<Vec<u8>>> {
        let coordinator_contract =
            ServiceClient::<CoordinatorContract>::prepare_service_client(self).await?;

        CoordinatorClient::call_contract_view(
            self.chain_id,
            "get_justifications",
            coordinator_contract.getJustifications(),
            self.contract_view_retry_descriptor,
        )
        .await
        .map(|r| r.iter().map(|b| b.to_vec()).collect::<Vec<Vec<u8>>>())
    }

    async fn get_participants(&self) -> ContractClientResult<Vec<Address>> {
        let coordinator_contract =
            ServiceClient::<CoordinatorContract>::prepare_service_client(self).await?;

        CoordinatorClient::call_contract_view(
            self.chain_id,
            "get_participants",
            coordinator_contract.getParticipants(),
            self.contract_view_retry_descriptor,
        )
        .await
    }

    async fn get_dkg_keys(&self) -> ContractClientResult<(usize, Vec<Vec<u8>>)> {
        let coordinator_contract =
            ServiceClient::<CoordinatorContract>::prepare_service_client(self).await?;

        CoordinatorClient::call_contract_view(
            self.chain_id,
            "get_dkg_keys",
            coordinator_contract.getDkgKeys(),
            self.contract_view_retry_descriptor,
        )
        .await
        .map(|get_dkg_keys_return| {
            (
                get_dkg_keys_return._0.to::<usize>(),
                get_dkg_keys_return
                    ._1
                    .iter()
                    .map(|b| b.to_vec())
                    .collect::<Vec<Vec<u8>>>(), // TODO: check if this is correct
            )
        })
    }

    async fn in_phase(&self) -> ContractClientResult<i8> {
        let coordinator_contract =
            ServiceClient::<CoordinatorContract>::prepare_service_client(self).await?;

        CoordinatorClient::call_contract_view(
            self.chain_id,
            "in_phase",
            coordinator_contract.inPhase(),
            self.contract_view_retry_descriptor,
        )
        .await
    }
}

#[async_trait]
impl<C: Curve + 'static> BoardPublisher<C> for CoordinatorClient {
    type Error = DKGContractError;

    async fn publish_shares(&mut self, shares: BundledShares<C>) -> Result<(), Self::Error> {
        info!("called publish_shares");
        let serialized = bincode::serialize(&shares)?;
        self.publish(serialized).await?;
        Ok(())
    }

    async fn publish_responses(&mut self, responses: BundledResponses) -> Result<(), Self::Error> {
        info!("called publish_responses");
        let serialized = bincode::serialize(&responses)?;
        self.publish(serialized).await?;
        Ok(())
    }

    async fn publish_justifications(
        &mut self,
        justifications: BundledJustification<C>,
    ) -> Result<(), Self::Error> {
        let serialized = bincode::serialize(&justifications)?;
        self.publish(serialized).await?;
        Ok(())
    }
}

#[cfg(test)]
pub mod coordinator_tests {
    use super::{CoordinatorClient, ProviderClientWithSigner};
    use crate::coordinator::CoordinatorTransactions;
    use crate::error::ContractClientError;
    use crate::ethers::coordinator::Coordinator;
    use crate::ethers::coordinator::Coordinator::CoordinatorInstance;
    use alloy::node_bindings::Anvil;
    use alloy::node_bindings::AnvilInstance;
    use alloy::primitives::Address;
    use alloy::primitives::U256;
    use alloy::providers::WsConnect;
    use alloy::signers::local::coins_bip39::English;
    use alloy::signers::local::MnemonicBuilder;
    use alloy::signers::local::PrivateKeySigner;
    use arpa_core::build_client;
    use arpa_core::Config;
    use arpa_core::GeneralMainChainIdentity;
    use simple_logger::SimpleLogger;
    use std::env;
    use std::path::PathBuf;
    use std::time::Duration;
    use threshold_bls::schemes::bn254::G2Scheme;

    #[test]
    fn test_cargo_manifest_parent_dir() {
        let dir = env!("CARGO_MANIFEST_DIR");
        println!("{:?}", PathBuf::new().join(dir).parent());
    }

    const PHRASE: &str =
        "work man father plunge mystery proud hollow address reunion sauce theory bonus";
    const INDEX: u32 = 0;

    fn start_chain() -> AnvilInstance {
        Anvil::new().chain_id(1u64).mnemonic(PHRASE).spawn()
    }

    async fn deploy_contract(
        anvil: &AnvilInstance,
    ) -> CoordinatorInstance<ProviderClientWithSigner> {
        // 2. instantiate our wallet
        let wallet: PrivateKeySigner = anvil.keys()[0].clone().into();

        // 3. connect to the network
        let ws =
            WsConnect::new(anvil.ws_endpoint()).with_retry_interval(Duration::from_millis(3000));

        // 4. instantiate the client with the wallet
        let client = build_client(wallet, anvil.chain_id(), ws).await.unwrap();

        // let client = build_client(wallet, anvil.chain_id() as usize, provider);

        // 5. deploy contract
        let instance = Coordinator::deploy(client.clone(), U256::from(3u8), U256::from(30u8))
            .await
            .unwrap();

        // if let Some(tx) = call.deployer.tx.as_eip1559_mut() {
        //     let (max_fee, max_priority_fee) = client
        //         .estimate_eip1559_fees(Some(eip1559_gas_price_estimator))
        //         .await
        //         .unwrap();
        //     tx.max_fee_per_gas = Some(max_fee);
        //     tx.max_priority_fee_per_gas = Some(max_priority_fee);
        // }

        // let coordinator_contract = call.send().await.unwrap();

        instance
    }

    #[tokio::test]
    async fn test_coordinator_in_phase() {
        let anvil = start_chain();
        let coordinator_contract = deploy_contract(&anvil).await;
        let res = coordinator_contract.inPhase().call().await.unwrap();

        println!("{:?}", res);
    }

    #[tokio::test]
    async fn test_publish_to_coordinator() {
        SimpleLogger::new()
            .with_level(log::LevelFilter::Info)
            .init()
            .unwrap();

        let config = Config::default();

        let anvil = start_chain();
        let coordinator_contract = deploy_contract(&anvil).await;

        let wallet = MnemonicBuilder::<English>::default()
            .phrase(PHRASE)
            .index(INDEX)
            .unwrap()
            .build()
            .unwrap();

        // mock dkg key pair
        let (_, dkg_public_key) = dkg_core::generate_keypair::<G2Scheme>();

        let nodes = vec![wallet.address()];
        let public_keys = vec![bincode::serialize(&dkg_public_key).unwrap().into()];

        let pending_tx = coordinator_contract
            .initialize(nodes, public_keys)
            .send()
            .await
            .unwrap();
        pending_tx.get_receipt().await.unwrap();

        let ws_connect =
            WsConnect::new(anvil.ws_endpoint()).with_retry_interval(Duration::from_millis(3000));

        let client = build_client(wallet.clone(), anvil.chain_id(), ws_connect.clone())
            .await
            .unwrap();

        let main_chain_identity = GeneralMainChainIdentity::new(
            anvil.chain_id(),
            wallet,
            ws_connect,
            client,
            anvil.ws_endpoint(),
            Address::ZERO,
            Address::ZERO,
            Address::ZERO,
            config
                .get_time_limits()
                .contract_transaction_retry_descriptor,
            config.get_time_limits().contract_view_retry_descriptor,
            config.get_max_priority_fee_per_gas(),
        );

        let client = CoordinatorClient::new(
            anvil.chain_id(),
            *coordinator_contract.address(),
            &main_chain_identity,
            config
                .get_time_limits()
                .contract_transaction_retry_descriptor,
            config.get_time_limits().contract_view_retry_descriptor,
            config.get_max_priority_fee_per_gas(),
        );

        let mock_value = vec![1, 2, 3, 4];
        let res = client.publish(mock_value.clone()).await;
        assert!(res.is_ok());

        let res = client.publish(mock_value.clone()).await;
        assert!(res.is_err());
        if let ContractClientError::TransportError(error) = res.unwrap_err() {
            if error.is_error_resp() {
                let error_msg = error.as_error_resp().unwrap().to_string();
                assert!(error_msg.contains("share existed"));
            } else {
                panic!("should be revert error")
            }
        }
    }
    // if let ContractClientError::TransportError(error if error.is_error_resp()) =
    //     res.unwrap_err()
    // {
    //     let error_msg = error.as_error_resp().unwrap().to_string();
    //     assert!(error_msg.contains("share existed"));
    // } else {
    //     panic!("should be revert error")
    // }
    // }

    #[test]
    fn test_three_ways_to_provide_wallet() {
        //1. mnemonic

        // Access mnemonic phrase with password
        // Child key at derivation path: m/44'/60'/0'/0/{index}
        let password = "TREZOR123";

        let wallet1 = MnemonicBuilder::<English>::default()
            .phrase(PHRASE)
            .index(INDEX)
            .unwrap()
            // Use this if your mnemonic is encrypted
            .password(password)
            .build()
            .unwrap();

        // 2.private key in plaintext
        let wallet2: PrivateKeySigner =
            "4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318"
                .parse()
                .unwrap();

        // 3. private key in keystore(protected by password)
        let path = PathBuf::new().join(env!("CARGO_MANIFEST_DIR"));
        let mut rng = rand::thread_rng();
        let (_key, _uuid) =
            PrivateKeySigner::new_keystore(&path, &mut rng, "randpsswd", Some("passwd")).unwrap();

        // read from the encrypted JSON keystore and decrypt it, while validating that the
        // signatures produced by both the keys should match

        let wallet3 =
            PrivateKeySigner::decrypt_keystore(&path.join("passwd"), "randpsswd").unwrap();
        // let signature2 = key2.sign_message(message).await.unwrap();

        println!("{:?}", wallet1);
        println!("{:?}", wallet2);
        println!("{:?}", wallet3);
    }
}
