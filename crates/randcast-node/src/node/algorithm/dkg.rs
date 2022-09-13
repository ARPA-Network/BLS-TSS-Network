use crate::node::{
    contract_client::coordinator::{CoordinatorTransactions, CoordinatorViews},
    error::NodeResult,
};
use async_trait::async_trait;
use dkg_core::{
    primitives::{joint_feldman::*, *},
    BoardPublisher, DKGPhase, Phase2Result,
};
use log::info;
use rand::RngCore;
use rustc_hex::ToHex;
use std::io::{self, Write};
use threshold_bls::{
    curve::bls12381::{Curve, Scalar, G1},
    poly::Idx,
};

#[async_trait]
pub(crate) trait DKGCore<F, R> {
    async fn run_dkg(
        &mut self,
        dkg_private_key: Scalar,
        node_rpc_endpoint: String,
        rng: F,
    ) -> NodeResult<DKGOutput<Curve>>
    where
        R: RngCore,
        F: Fn() -> R + Send + 'async_trait;
}

pub(crate) struct AllPhasesDKGCore<
    P: CoordinatorTransactions + CoordinatorViews + BoardPublisher<Curve>,
> {
    coordinator_client: P,
}

impl<P: CoordinatorTransactions + CoordinatorViews + BoardPublisher<Curve>> AllPhasesDKGCore<P> {
    pub fn new(coordinator_client: P) -> Self {
        AllPhasesDKGCore { coordinator_client }
    }
}

#[async_trait]
impl<F, R, P> DKGCore<F, R> for AllPhasesDKGCore<P>
where
    R: RngCore,
    F: Fn() -> R,
    P: CoordinatorTransactions + CoordinatorViews + BoardPublisher<Curve> + Sync + Send,
{
    async fn run_dkg(
        &mut self,
        dkg_private_key: Scalar,
        node_rpc_endpoint: String,
        rng: F,
    ) -> NodeResult<DKGOutput<Curve>>
    where
        F: Send + 'async_trait,
    {
        // TODO error handling and retry

        // Wait for Phase 0
        wait_for_phase(&self.coordinator_client, 0).await?;

        // Get the group info
        let group = self.coordinator_client.get_bls_keys().await?;
        let participants = self.coordinator_client.get_participants().await?;

        // print some debug info
        info!(
            "Will run DKG with the group listed below and threshold {}",
            group.0
        );
        for (bls_pubkey, address) in group.1.iter().zip(participants) {
            let key = bls_pubkey.to_hex::<String>();
            info!("{:?} -> {}", address, key)
        }

        let nodes = group
            .1
            .into_iter()
            .filter(|pubkey| !pubkey.is_empty()) // skip users that did not register
            .enumerate()
            .map(|(i, pubkey)| {
                let pubkey: G1 = bincode::deserialize(&pubkey)?;
                Ok(Node::<Curve>::new(i as Idx, pubkey))
            })
            .collect::<NodeResult<_>>()?;

        let group = Group {
            threshold: group.0,
            nodes,
        };

        // Instantiate the DKG with the group info
        info!("Calculating and broadcasting our shares... Running Phase 0.");
        let phase0 = DKG::new(dkg_private_key, node_rpc_endpoint, group)?;

        // Run Phase 0 and publish to the chain
        let phase1 = phase0.run(&mut self.coordinator_client, rng).await?;

        // Wait for Phase 1
        wait_for_phase(&self.coordinator_client, 1).await?;

        // Get the shares
        let shares = self.coordinator_client.get_shares().await?;
        info!("Got {} shares...", shares.len());
        let shares = parse_bundle(&shares)?;
        info!("Parsed {} shares. Running Phase 1.", shares.len());

        // Run Phase 1
        let phase2 = phase1.run(&mut self.coordinator_client, &shares).await?;

        // Wait for Phase 2
        wait_for_phase(&self.coordinator_client, 2).await?;

        // Get the responses
        let responses = self.coordinator_client.get_responses().await?;
        info!("Got {} responses...", responses.len());
        let responses = parse_bundle(&responses)?;
        info!("Parsed {} responses. Running Phase 2.", responses.len());

        // Run Phase 2
        let result = match phase2.run(&mut self.coordinator_client, &responses).await? {
            Phase2Result::Output(out) => Ok(out),
            // Run Phase 3 if Phase 2 errored
            Phase2Result::GoToPhase3(phase3) => {
                info!("There were complaints. Running Phase 3.");
                // Wait for Phase 3
                wait_for_phase(&self.coordinator_client, 3).await?;

                let justifications = self.coordinator_client.get_justifications().await?;
                let justifications = parse_bundle(&justifications)?;

                // Run Phase 3
                phase3
                    .run(&mut self.coordinator_client, &justifications)
                    .await
            }
        };

        match result {
            Ok(output) => {
                info!("Success. Your share and threshold pubkey are ready.");

                write_output(&output)?;

                // info!("{:#?}", output.qual.nodes);

                // info!("public key: {}", output.public.public_key());

                Ok(output)
            }
            Err(err) => Err(err.into()),
        }
    }
}

async fn wait_for_phase(dkg: &impl CoordinatorViews, num: usize) -> NodeResult<()> {
    info!("Waiting for Phase {} to start", num);

    loop {
        let phase = dkg.in_phase().await?;

        if phase >= num {
            break;
        }

        print!(".");
        io::stdout().flush().unwrap();

        // 1s for demonstration, should be changed to block mining interval
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
    }

    info!("In Phase {}. Moving to the next step.", num);

    Ok(())
}

fn parse_bundle<D: serde::de::DeserializeOwned>(bundle: &[Vec<u8>]) -> NodeResult<Vec<D>> {
    bundle
        .iter()
        .filter(|item| !item.is_empty()) // filter out empty items
        .map(|item| Ok(bincode::deserialize::<D>(item)?))
        .collect()
}

fn write_output(out: &DKGOutput<Curve>) -> NodeResult<()> {
    let output = OutputJson {
        public_key: hex::encode(&bincode::serialize(&out.public.public_key())?),
        public_polynomial: hex::encode(&bincode::serialize(&out.public)?),
        share: hex::encode(&bincode::serialize(&out.share)?),
    };

    info!("{:?}", output);

    Ok(())
}

#[derive(serde::Serialize, Debug)]
struct OutputJson {
    #[serde(rename = "publicKey")]
    public_key: String,
    #[serde(rename = "publicPolynomial")]
    public_polynomial: String,
    #[serde(rename = "share")]
    share: String,
}
