//! # DKG Core
//!
//! The DKG is based on the [Secure Distributed Key Generation for Discrete-Log Based Cryptosystems
//! ](https://link.springer.com/article/10.1007/s00145-006-0347-3) paper.
//!
//! The implementation is a state machine which has Phases 0 to 3. Phase 3 is only reachable if any of the
//! n parties does not publish its shares in the first phase. If less than t parties participate in any stage,
//! the DKG fails.

/// Board trait and implementations for publishing data from each DKG phase
pub mod board;
pub use board::BoardPublisher;

/// Higher level objects for running a JF-DKG
pub mod node;
pub use node::{DKGPhase, NodeError, Phase2Result};
use threshold_bls::group::Element;
use threshold_bls::sig::Scheme;

/// Low level primitives and datatypes for implementing DKGs
pub mod primitives;

#[cfg(test)]
pub mod test_helpers;

pub fn generate_keypair<S: Scheme>() -> (S::Private, S::Public) {
    let rng = &mut rand::thread_rng();

    let private = S::Private::rand(rng);

    let mut public = S::Public::one();
    public.mul(&private);

    (private, public)
}
