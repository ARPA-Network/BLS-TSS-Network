use crate::node::error::NodeResult;
use threshold_bls::{
    curve::bls12381::{PairingCurve as BLS12_381, Scalar, G1},
    poly::Eval,
    sig::{G1Scheme, Share, SignatureScheme, ThresholdScheme},
};

pub(crate) struct SimpleBLSCore {}

pub(crate) trait BLSCore {
    /// Partially signs a message with a share of the private key
    fn partial_sign(&self, private: &Share<Scalar>, msg: &[u8]) -> NodeResult<Vec<u8>>;

    /// Verifies a partial signature on a message against the public polynomial
    fn partial_verify(&self, partial_public_key: &G1, msg: &[u8], partial: &[u8])
        -> NodeResult<()>;

    /// Aggregates all partials signature together. Note that this method does
    /// not verify if the partial signatures are correct or not; it only
    /// aggregates them.
    fn aggregate(&self, threshold: usize, partials: &[Vec<u8>]) -> NodeResult<Vec<u8>>;

    /// Verifies that the signature on the provided message was produced by the public key
    fn verify(&self, public: &G1, msg: &[u8], sig: &[u8]) -> NodeResult<()>;

    fn verify_partial_sigs(
        &self,
        publics: &[G1],
        msg: &[u8],
        partial_sigs: &[&[u8]],
    ) -> NodeResult<()>;
}

impl BLSCore for SimpleBLSCore {
    fn partial_sign(&self, private: &Share<Scalar>, msg: &[u8]) -> NodeResult<Vec<u8>> {
        let partial_signature = G1Scheme::<BLS12_381>::partial_sign(private, msg)?;
        Ok(partial_signature)
    }

    fn partial_verify(
        &self,
        partial_public_key: &G1,
        msg: &[u8],
        partial: &[u8],
    ) -> NodeResult<()> {
        let partial: Eval<Vec<u8>> = bincode::deserialize(partial)?;
        self.verify(partial_public_key, msg, &partial.value)?;
        Ok(())
    }

    fn aggregate(&self, threshold: usize, partials: &[Vec<u8>]) -> NodeResult<Vec<u8>> {
        let signature = G1Scheme::<BLS12_381>::aggregate(threshold, partials)?;
        Ok(signature)
    }

    fn verify(&self, public: &G1, msg: &[u8], sig: &[u8]) -> NodeResult<()> {
        G1Scheme::<BLS12_381>::verify(public, msg, sig)?;
        Ok(())
    }

    fn verify_partial_sigs(
        &self,
        publics: &[G1],
        msg: &[u8],
        partial_sigs: &[&[u8]],
    ) -> NodeResult<()> {
        G1Scheme::<BLS12_381>::aggregation_verify_on_the_same_msg(publics, msg, partial_sigs)?;
        Ok(())
    }
}
