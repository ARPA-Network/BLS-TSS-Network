use async_trait::async_trait;
use dkg_core::board::BoardPublisher;
use dkg_core::primitives::types::{BundledJustification, BundledResponses, BundledShares};
use dkg_core::primitives::{
    group::{Group, Node},
    joint_feldman,
    types::DKGOutput,
};
use dkg_core::{DKGPhase, Phase2Result};
use threshold_bls::group::Curve;
use threshold_bls::{poly::Idx, sig::Scheme};

/// An in-memory board used for testing
#[derive(Default)]
pub struct InMemoryBoard<C: Curve> {
    pub shares: Vec<BundledShares<C>>,
    pub responses: Vec<BundledResponses>,
    pub justifs: Vec<BundledJustification<C>>,
}

impl<C: Curve> InMemoryBoard<C> {
    #[allow(unused)]
    pub fn new() -> Self {
        Self {
            shares: vec![],
            responses: vec![],
            justifs: vec![],
        }
    }
}

#[async_trait]
impl<C: Curve> BoardPublisher<C> for InMemoryBoard<C> {
    type Error = ();

    async fn publish_shares(&mut self, bundle: BundledShares<C>) -> Result<(), Self::Error>
    where
        C: 'async_trait,
    {
        self.shares.push(bundle);
        Ok(())
    }

    async fn publish_responses(&mut self, bundle: BundledResponses) -> Result<(), Self::Error>
    where
        C: 'async_trait,
    {
        self.responses.push(bundle);
        Ok(())
    }

    async fn publish_justifications(
        &mut self,
        bundle: BundledJustification<C>,
    ) -> Result<(), Self::Error>
    where
        C: 'async_trait,
    {
        self.justifs.push(bundle);
        Ok(())
    }
}

#[allow(unused)]
pub async fn run_dkg<C, S>(
    board: &mut InMemoryBoard<C>,
    phase0s: Vec<joint_feldman::DKG<C>>,
) -> Vec<DKGOutput<C>>
where
    C: Curve,
    // We need to bind the Curve's Point and Scalars to the Scheme
    S: Scheme<Public = <C as Curve>::Point, Private = <C as Curve>::Scalar>,
{
    // Phase 1: Publishes shares
    let mut phase1s = Vec::new();
    for phase0 in phase0s {
        phase1s.push(phase0.run(board, rand::thread_rng).await.unwrap());
    }

    // Get the shares from the board
    let shares = board.shares.clone();

    // Phase2
    let mut phase2s = Vec::new();
    for phase1 in phase1s {
        phase2s.push(phase1.run(board, &shares).await.unwrap());
    }

    // Get the responses from the board
    let responses = board.responses.clone();

    let mut results = Vec::new();
    for phase2 in phase2s {
        results.push(phase2.run(board, &responses).await.unwrap());
    }

    // The distributed public key must be the same
    let outputs = results
        .into_iter()
        .map(|res| match res {
            Phase2Result::Output(out) => out,
            Phase2Result::GoToPhase3(_) => unreachable!("should not get here"),
        })
        .collect::<Vec<_>>();
    assert!(is_all_same(outputs.iter().map(|output| {
        // println!("{:?}", output.public);
        &output.public
    })));

    outputs
}

#[allow(unused)]
pub fn setup<C, S, R: rand::RngCore>(
    n: usize,
    t: usize,
    rng: &mut R,
) -> (InMemoryBoard<C>, Vec<joint_feldman::DKG<C>>)
where
    C: Curve,
    // We need to bind the Curve's Point and Scalars to the Scheme
    S: Scheme<Public = C::Point, Private = <C as Curve>::Scalar>,
{
    // generate a keypair per participant
    let keypairs = (0..n).map(|_| S::keypair(rng)).collect::<Vec<_>>();

    let nodes = keypairs
        .iter()
        .enumerate()
        .map(|(i, (_, public))| {
            // println!("{}", i);
            Node::<C>::new(i as Idx, public.clone())
        })
        .collect::<Vec<_>>();

    // This is setup phase during which publickeys and indexes must be exchanged
    // across participants
    let group = Group::new(nodes, t).unwrap();

    // Create the Phase 0 for each participant
    let phase0s = keypairs
        .iter()
        .map(|(private, _)| {
            joint_feldman::DKG::new(private.clone(), String::from(""), group.clone()).unwrap()
        })
        .collect::<Vec<_>>();

    // Create the board
    let board = InMemoryBoard::<C>::new();

    (board, phase0s)
}

pub fn is_all_same<T: PartialEq>(mut arr: impl Iterator<Item = T>) -> bool {
    let first = arr.next().unwrap();
    arr.all(|item| item == first)
}
