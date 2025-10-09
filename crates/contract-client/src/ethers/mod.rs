use crate::ethers::controller::IController::{
    Group as ControllerContractGroup, Member as ControllerContractMember,
};
use crate::ethers::controller_oracle::IControllerOracle::{
    Group as ControllerOracleContractGroup, Member as ControllerOracleContractMember,
};
use alloy::primitives::{Address, U256};
use arpa_core::{u256_to_vec, Group, Member};
use std::collections::BTreeMap;
use std::marker::PhantomData;
use threshold_bls::group::Curve;

pub mod adapter;
pub mod avs_directory;
pub mod controller;
pub mod controller_oracle;
pub mod controller_relayer;
pub mod coordinator;
pub mod ierc20;
pub mod node_registry;
pub mod provider;
pub mod service_manager;
pub mod staking;

pub fn parse_controller_oracle_contract_member<C: Curve>(
    cm: ControllerOracleContractMember,
    index: usize,
) -> Member<C> {
    let partial_public_key =
        if cm.partialPublicKey.is_empty() || cm.partialPublicKey[0] == U256::ZERO {
            None
        } else {
            let bytes = cm
                .partialPublicKey
                .iter()
                .map(u256_to_vec)
                .reduce(|mut acc, mut e| {
                    acc.append(&mut e);
                    acc
                })
                .unwrap();
            Some(bincode::deserialize(&bytes).unwrap())
        };

    Member {
        index,
        dkg_index: Some(0),
        id_address: cm.nodeIdAddress,
        rpc_endpoint: None,
        partial_public_key,
    }
}

pub fn parse_controller_contract_member<C: Curve>(
    cm: ControllerContractMember,
    index: usize,
) -> Member<C> {
    let partial_public_key =
        if cm.partialPublicKey.is_empty() || cm.partialPublicKey[0] == U256::ZERO {
            None
        } else {
            let bytes = cm
                .partialPublicKey
                .iter()
                .map(u256_to_vec)
                .reduce(|mut acc, mut e| {
                    acc.append(&mut e);
                    acc
                })
                .unwrap();
            Some(bincode::deserialize(&bytes).unwrap())
        };

    Member {
        index,
        dkg_index: Some(0),
        id_address: cm.nodeIdAddress,
        rpc_endpoint: None,
        partial_public_key,
    }
}

pub fn parse_controller_oracle_contract_group<C: Curve>(
    cg: ControllerOracleContractGroup,
) -> Group<C> {
    let ControllerOracleContractGroup {
        index,
        epoch,
        size,
        threshold,
        publicKey,
        members,
        committers,
        commitCacheList: _,
        isStrictlyMajorityConsensusReached,
    } = cg;

    let members: BTreeMap<Address, Member<C>> = members
        .into_iter()
        .enumerate()
        .map(|(index, cm)| {
            (
                cm.nodeIdAddress,
                parse_controller_oracle_contract_member(cm, index),
            )
        })
        .collect();

    let public_key = if publicKey.is_empty() || publicKey[0] == U256::ZERO {
        None
    } else {
        let bytes = publicKey
            .iter()
            .map(u256_to_vec)
            .reduce(|mut acc, mut e| {
                acc.append(&mut e);
                acc
            })
            .unwrap();
        Some(bincode::deserialize(&bytes).unwrap())
    };

    Group {
        index: index.to::<usize>(),
        epoch: epoch.to::<usize>(),
        size: size.to::<usize>(),
        threshold: threshold.to::<usize>(),
        state: isStrictlyMajorityConsensusReached,
        public_key,
        members,
        committers,
        c: PhantomData,
    }
}

pub fn parse_controller_contract_group<C: Curve>(cg: ControllerContractGroup) -> Group<C> {
    let ControllerContractGroup {
        index,
        epoch,
        size,
        threshold,
        publicKey,
        members,
        committers,
        commitCacheList: _,
        isStrictlyMajorityConsensusReached,
    } = cg;

    let members: BTreeMap<Address, Member<C>> = members
        .into_iter()
        .enumerate()
        .map(|(index, cm)| {
            (
                cm.nodeIdAddress,
                parse_controller_contract_member(cm, index),
            )
        })
        .collect();

    let public_key = if publicKey.is_empty() || publicKey[0] == U256::ZERO {
        None
    } else {
        let bytes = publicKey
            .iter()
            .map(u256_to_vec)
            .reduce(|mut acc, mut e| {
                acc.append(&mut e);
                acc
            })
            .unwrap();
        Some(bincode::deserialize(&bytes).unwrap())
    };

    Group {
        index: index.to::<usize>(),
        epoch: epoch.to::<usize>(),
        size: size.to::<usize>(),
        threshold: threshold.to::<usize>(),
        state: isStrictlyMajorityConsensusReached,
        public_key,
        members,
        committers,
        c: PhantomData,
    }
}

#[cfg(test)]
pub mod contract_interaction_tests {

    use alloy::eips::eip1559::Eip1559Estimation;
    use alloy::providers::utils::Eip1559Estimator;
    use alloy::providers::Provider;
    use alloy::providers::ProviderBuilder;
    use arpa_core::eip1559_gas_price_estimator;

    #[tokio::test]
    async fn test_estimate_eip1559_fees() -> Result<(), anyhow::Error> {
        let provider = ProviderBuilder::new().connect_http("https://eth.llamarpc.com".parse()?);

        let Eip1559Estimation {
            max_fee_per_gas,
            max_priority_fee_per_gas,
        } = provider
            .estimate_eip1559_fees_with(Eip1559Estimator::Custom(Box::new(
                eip1559_gas_price_estimator,
            )))
            .await?;
        println!("max_fee: {:?}", max_fee_per_gas);
        println!("max_priority_fee: {:?}", max_priority_fee_per_gas);

        Ok(())
    }
}
