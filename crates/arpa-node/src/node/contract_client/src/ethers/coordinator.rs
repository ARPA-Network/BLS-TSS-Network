use crate::{
    coordinator::{
        CoordinatorClientBuilder, CoordinatorTransactions, CoordinatorViews, DKGContractError,
    },
    error::{ContractClientError, ContractClientResult},
    ServiceClient,
};

use self::coordinator_stub::Coordinator;
use super::WalletSigner;
use arpa_node_core::{ChainIdentity, GeneralChainIdentity};
use async_trait::async_trait;
use dkg_core::{
    primitives::{BundledJustification, BundledResponses, BundledShares},
    BoardPublisher,
};
use ethers::prelude::*;
use log::info;
use std::{convert::TryFrom, sync::Arc, time::Duration};
use threshold_bls::curve::bls12381::Curve;

#[allow(clippy::useless_conversion)]
pub mod coordinator_stub {
    include!("../../contract_stub/coordinator.rs");
}

pub struct CoordinatorClient {
    coordinator_address: Address,
    signer: Arc<WalletSigner>,
}

impl CoordinatorClient {
    pub fn new(coordinator_address: Address, identity: &GeneralChainIdentity) -> Self {
        let provider = Provider::<Http>::try_from(identity.get_provider_rpc_endpoint())
            .unwrap()
            .interval(Duration::from_millis(10u64));

        // instantiate the client with the wallet
        let signer = Arc::new(SignerMiddleware::new(
            provider,
            identity
                .get_signer()
                .clone()
                .with_chain_id(identity.get_chain_id() as u32),
        ));

        CoordinatorClient {
            coordinator_address,
            signer,
        }
    }
}

impl CoordinatorClientBuilder for GeneralChainIdentity {
    type Service = CoordinatorClient;

    fn build_coordinator_client(&self, contract_address: Address) -> CoordinatorClient {
        CoordinatorClient::new(contract_address, self)
    }
}

type CoordinatorContract = Coordinator<WalletSigner>;

#[async_trait]
impl ServiceClient<CoordinatorContract> for CoordinatorClient {
    async fn prepare_service_client(&self) -> ContractClientResult<CoordinatorContract> {
        let coordinator_contract = Coordinator::new(self.coordinator_address, self.signer.clone());

        Ok(coordinator_contract)
    }
}

#[async_trait]
impl CoordinatorTransactions for CoordinatorClient {
    async fn publish(&self, value: Vec<u8>) -> ContractClientResult<()> {
        let coordinator_contract =
            ServiceClient::<CoordinatorContract>::prepare_service_client(self).await?;

        coordinator_contract
            .publish(value.into())
            .send()
            .await
            .map_err(|e| {
                let e: ContractClientError = e.into();
                e
            })?
            .await
            .map_err(|e| {
                let e: ContractClientError = e.into();
                e
            })?;

        Ok(())
    }
}

#[async_trait]
impl CoordinatorViews for CoordinatorClient {
    async fn get_shares(&self) -> ContractClientResult<Vec<Vec<u8>>> {
        let coordinator_contract =
            ServiceClient::<CoordinatorContract>::prepare_service_client(self).await?;

        let res = coordinator_contract
            .get_shares()
            .call()
            .await
            .map(|r| r.iter().map(|b| b.to_vec()).collect::<Vec<Vec<u8>>>())
            .map_err(|e| {
                let e: ContractClientError = e.into();
                e
            })?;

        Ok(res)
    }

    async fn get_responses(&self) -> ContractClientResult<Vec<Vec<u8>>> {
        let coordinator_contract =
            ServiceClient::<CoordinatorContract>::prepare_service_client(self).await?;

        let res = coordinator_contract
            .get_responses()
            .call()
            .await
            .map(|r| r.iter().map(|b| b.to_vec()).collect::<Vec<Vec<u8>>>())
            .map_err(|e| {
                let e: ContractClientError = e.into();
                e
            })?;

        Ok(res)
    }

    async fn get_justifications(&self) -> ContractClientResult<Vec<Vec<u8>>> {
        let coordinator_contract =
            ServiceClient::<CoordinatorContract>::prepare_service_client(self).await?;

        let res = coordinator_contract
            .get_justifications()
            .call()
            .await
            .map(|r| r.iter().map(|b| b.to_vec()).collect::<Vec<Vec<u8>>>())
            .map_err(|e| {
                let e: ContractClientError = e.into();
                e
            })?;

        Ok(res)
    }

    async fn get_participants(&self) -> ContractClientResult<Vec<Address>> {
        let coordinator_contract =
            ServiceClient::<CoordinatorContract>::prepare_service_client(self).await?;

        let res = coordinator_contract
            .get_participants()
            .call()
            .await
            .map_err(|e| {
                let e: ContractClientError = e.into();
                e
            })?;

        Ok(res)
    }

    async fn get_bls_keys(&self) -> ContractClientResult<(usize, Vec<Vec<u8>>)> {
        let coordinator_contract =
            ServiceClient::<CoordinatorContract>::prepare_service_client(self).await?;

        let res = coordinator_contract
            .get_bls_keys()
            .call()
            .await
            .map(|(t, keys)| {
                (
                    t.as_usize(),
                    keys.iter().map(|b| b.to_vec()).collect::<Vec<Vec<u8>>>(),
                )
            })
            .map_err(|e| {
                let e: ContractClientError = e.into();
                e
            })?;

        Ok(res)
    }

    async fn in_phase(&self) -> ContractClientResult<usize> {
        let coordinator_contract =
            ServiceClient::<CoordinatorContract>::prepare_service_client(self).await?;

        let res = coordinator_contract.in_phase().call().await.map_err(|e| {
            let e: ContractClientError = e.into();
            e
        })?;

        Ok(res.as_usize())
    }
}

#[async_trait]
impl BoardPublisher<Curve> for CoordinatorClient {
    type Error = DKGContractError;

    async fn publish_shares(&mut self, shares: BundledShares<Curve>) -> Result<(), Self::Error> {
        info!("called publish_shares");
        let serialized = bincode::serialize(&shares)?;
        self.publish(serialized).await.map_err(|e| e.into())
    }

    async fn publish_responses(&mut self, responses: BundledResponses) -> Result<(), Self::Error> {
        info!("called publish_responses");
        let serialized = bincode::serialize(&responses)?;
        self.publish(serialized).await.map_err(|e| e.into())
    }

    async fn publish_justifications(
        &mut self,
        justifications: BundledJustification<Curve>,
    ) -> Result<(), Self::Error> {
        let serialized = bincode::serialize(&justifications)?;
        self.publish(serialized).await.map_err(|e| e.into())
    }
}

#[cfg(test)]
pub mod coordinator_tests {
    use crate::coordinator::CoordinatorTransactions;

    use super::{CoordinatorClient, WalletSigner};
    use arpa_node_core::GeneralChainIdentity;
    use ethers::abi::Tokenize;
    use ethers::prelude::*;
    use ethers::signers::coins_bip39::English;
    use ethers::utils::Anvil;
    use ethers::utils::AnvilInstance;
    use std::env;
    use std::path::PathBuf;
    use std::{convert::TryFrom, sync::Arc, time::Duration};
    use threshold_bls::schemes::bls12_381::G1Scheme;

    include!("../../contract_stub/coordinator.rs");

    #[test]
    fn test_cargo_manifest_parent_dir() {
        let dir = env!("CARGO_MANIFEST_DIR");
        println!("{:?}", PathBuf::new().join(dir).parent());
        println!("{:?}", (3u8, 10u8).into_tokens());
    }

    const PHRASE: &str =
        "work man father plunge mystery proud hollow address reunion sauce theory bonus";
    const INDEX: u32 = 0;

    fn start_chain() -> AnvilInstance {
        Anvil::new().chain_id(1u64).mnemonic(PHRASE).spawn()
    }

    async fn deploy_contract(anvil: &AnvilInstance) -> Coordinator<WalletSigner> {
        // 2. instantiate our wallet
        let wallet: LocalWallet = anvil.keys()[0].clone().into();

        // 3. connect to the network
        let provider = Provider::<Http>::try_from(anvil.endpoint())
            .unwrap()
            .interval(Duration::from_millis(10u64));

        // 4. instantiate the client with the wallet
        let client = Arc::new(SignerMiddleware::new(
            provider,
            wallet.with_chain_id(anvil.chain_id()),
        ));

        // 5. deploy contract
        let coordinator_contract = Coordinator::deploy(client, (3u8, 30u8))
            .unwrap()
            .send()
            .await
            .unwrap();

        coordinator_contract
    }

    #[tokio::test]
    async fn test_coordinator_in_phase() {
        let anvil = start_chain();
        let coordinator_contract = deploy_contract(&anvil).await;
        let res = coordinator_contract.in_phase().call().await.unwrap();

        println!("{:?}", res);
    }

    #[tokio::test]
    async fn test_publish_to_coordinator() {
        let anvil = start_chain();
        let coordinator_contract = deploy_contract(&anvil).await;

        let wallet = MnemonicBuilder::<English>::default()
            .phrase(PHRASE)
            .index(INDEX)
            .unwrap()
            .build()
            .unwrap();

        // mock dkg key pair
        let (_, dkg_public_key) = dkg_core::generate_keypair::<G1Scheme>();

        let nodes = vec![wallet.address()];
        let public_keys = vec![bincode::serialize(&dkg_public_key).unwrap().into()];

        coordinator_contract
            .initialize(nodes, public_keys)
            .send()
            .await
            .unwrap();

        let main_chain_identity = GeneralChainIdentity::new(
            0,
            anvil.chain_id() as usize,
            wallet,
            anvil.endpoint(),
            Address::random(),
        );

        let client = CoordinatorClient::new(coordinator_contract.address(), &main_chain_identity);

        let mock_value = vec![1, 2, 3, 4];
        let res = client.publish(mock_value.clone()).await;
        assert!(res.is_ok());

        let res = client.publish(mock_value.clone()).await;
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(err
            .to_string()
            .contains("you have already published your shares"));
    }

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
        let wallet2: LocalWallet =
            "4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318"
                .parse()
                .unwrap();

        // 3. private key in keystore(protected by password)
        let path = PathBuf::new().join(env!("CARGO_MANIFEST_DIR"));
        let mut rng = rand::thread_rng();
        let (_key, _uuid) =
            LocalWallet::new_keystore(&path, &mut rng, "randpsswd", Some("passwd")).unwrap();

        // read from the encrypted JSON keystore and decrypt it, while validating that the
        // signatures produced by both the keys should match

        let wallet3 = LocalWallet::decrypt_keystore(&path.join("passwd"), "randpsswd").unwrap();
        // let signature2 = key2.sign_message(message).await.unwrap();

        println!("{:?}", wallet1);
        println!("{:?}", wallet2);
        println!("{:?}", wallet3);
    }
}
