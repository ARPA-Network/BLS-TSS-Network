use crate::node::{
    contract_client::controller_client::{CoordinatorViews, MockCoordinatorClient},
    error::errors::NodeResult,
};
use async_trait::async_trait;
use dkg_core::{
    primitives::{joint_feldman::*, *},
    DKGPhase, Phase2Result,
};
use rand::RngCore;
use rustc_hex::ToHex;
use std::io::{self, Write};
use threshold_bls::{
    curve::bls12381::{Curve, Scalar, G1},
    poly::Idx,
};

#[async_trait]
pub trait DKGCore<F, R> {
    async fn run_dkg(
        &mut self,
        dkg_private_key: Scalar,
        node_rpc_endpoint: String,
        rng: F,
        coordinator_client: MockCoordinatorClient,
    ) -> NodeResult<DKGOutput<Curve>>
    where
        R: RngCore,
        F: Fn() -> R + Send + 'async_trait;
}

pub struct MockDKGCore {}

#[async_trait]
impl<F, R> DKGCore<F, R> for MockDKGCore
where
    R: RngCore,
    F: Fn() -> R,
{
    async fn run_dkg(
        &mut self,
        dkg_private_key: Scalar,
        node_rpc_endpoint: String,
        rng: F,
        mut coordinator_client: MockCoordinatorClient,
    ) -> NodeResult<DKGOutput<Curve>>
    where
        F: Send + 'async_trait,
    {
        // Wait for Phase 0
        wait_for_phase(&mut coordinator_client, 0).await?;

        // Get the group info
        let group = coordinator_client.get_bls_keys().await?;
        let participants = coordinator_client.get_participants().await?;

        // print some debug info
        println!(
            "Will run DKG with the group listed below and threshold {}",
            group.0
        );
        for (bls_pubkey, address) in group.1.iter().zip(&participants) {
            let key = bls_pubkey.to_hex::<String>();
            println!("{:?} -> {}", address, key)
        }

        // if !clt::confirm(
        //     "\nDoes the above group look good to you?",
        //     false,
        //     "\n",
        //     true,
        // ) {
        //     return Err(anyhow::anyhow!("User rejected group choice."));
        // }

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
        println!("Calculating and broadcasting our shares...");
        let phase0 = DKG::new(dkg_private_key, node_rpc_endpoint, group)?;

        // Run Phase 1 and publish to the chain
        let phase1 = phase0.run(&mut coordinator_client, rng).await?;

        // Wait for Phase 1
        wait_for_phase(&mut coordinator_client, 1).await?;

        // Get the shares
        let shares = coordinator_client.get_shares().await?;
        println!("Got {} shares...", shares.len());
        let shares = parse_bundle(&shares)?;
        println!("Parsed {} shares. Running Phase 2", shares.len());

        let phase2 = phase1.run(&mut coordinator_client, &shares).await?;

        // Wait for Phase 2
        wait_for_phase(&mut coordinator_client, 2).await?;

        // Get the responses
        let responses = coordinator_client.get_responses().await?;
        println!("Got {} responses...", responses.len());
        let responses = parse_bundle(&responses)?;
        println!("Parsed the responses. Getting result.");

        // Run Phase 2
        let result = match phase2.run(&mut coordinator_client, &responses).await? {
            Phase2Result::Output(out) => Ok(out),
            // Run Phase 3 if Phase 2 errored
            Phase2Result::GoToPhase3(phase3) => {
                println!("There were complaints. Running Phase 3.");
                // Wait for Phase 3
                wait_for_phase(&mut coordinator_client, 3).await?;

                let justifications = coordinator_client.get_justifications().await?;
                let justifications = parse_bundle(&justifications)?;

                phase3.run(&mut coordinator_client, &justifications).await
            }
        };

        match result {
            Ok(output) => {
                println!("Success. Your share and threshold pubkey are ready.");

                write_output(std::io::stdout(), &output)?;
                println!();

                // println!("{:#?}", output.qual.nodes);

                // println!("public key: {}", output.public.public_key());

                Ok(output)
            }
            Err(err) => Err(err.into()),
        }
    }
}

async fn wait_for_phase(dkg: &mut impl CoordinatorViews, num: usize) -> NodeResult<()> {
    println!("Waiting for Phase {} to start", num);

    loop {
        let phase = dkg.in_phase().await?;

        if phase >= num {
            break;
        }

        print!(".");
        io::stdout().flush().unwrap();

        // 1s for demonstration
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
    }

    println!("\nIn Phase {}. Moving to the next step.", num);

    Ok(())
}

fn parse_bundle<D: serde::de::DeserializeOwned>(bundle: &[Vec<u8>]) -> NodeResult<Vec<D>> {
    bundle
        .iter()
        .filter(|item| !item.is_empty()) // filter out empty items
        .map(|item| Ok(bincode::deserialize::<D>(item)?))
        .collect()
}

fn write_output<W: Write>(writer: W, out: &DKGOutput<Curve>) -> NodeResult<()> {
    let output = OutputJson {
        public_key: hex::encode(&bincode::serialize(&out.public.public_key())?),
        public_polynomial: hex::encode(&bincode::serialize(&out.public)?),
        share: hex::encode(&bincode::serialize(&out.share)?),
    };
    serde_json::to_writer(writer, &output)?;

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
