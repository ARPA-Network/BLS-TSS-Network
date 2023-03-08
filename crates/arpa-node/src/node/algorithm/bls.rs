use anyhow::Result;
use std::marker::PhantomData;
use threshold_bls::{
    group::PairingCurve,
    poly::Eval,
    sig::{G2Scheme, Scheme, Share, SignatureScheme, ThresholdScheme},
};

pub(crate) struct SimpleBLSCore<C: PairingCurve> {
    c: PhantomData<C>,
}

pub(crate) trait BLSCore<C: PairingCurve> {
    /// Partially signs a message with a share of the private key
    fn partial_sign(
        private: &Share<<G2Scheme<C> as Scheme>::Private>,
        msg: &[u8],
    ) -> Result<Vec<u8>>;

    /// Verifies a partial signature on a message against the public polynomial
    fn partial_verify(
        partial_public_key: &<G2Scheme<C> as Scheme>::Public,
        msg: &[u8],
        partial: &[u8],
    ) -> Result<()>;

    /// Aggregates all partials signature together. Note that this method does
    /// not verify if the partial signatures are correct or not; it only
    /// aggregates them.
    fn aggregate(threshold: usize, partials: &[Vec<u8>]) -> Result<Vec<u8>>;

    /// Verifies that the signature on the provided message was produced by the public key
    fn verify(public: &<G2Scheme<C> as Scheme>::Public, msg: &[u8], sig: &[u8]) -> Result<()>;

    fn verify_partial_sigs(
        publics: &[<G2Scheme<C> as Scheme>::Public],
        msg: &[u8],
        partial_sigs: &[&[u8]],
    ) -> Result<()>;
}

impl<C: PairingCurve + 'static> BLSCore<C> for SimpleBLSCore<C> {
    fn partial_sign(
        private: &Share<<G2Scheme<C> as Scheme>::Private>,
        msg: &[u8],
    ) -> Result<Vec<u8>> {
        let partial_signature = G2Scheme::<C>::partial_sign(private, msg)?;
        Ok(partial_signature)
    }

    fn partial_verify(
        partial_public_key: &<G2Scheme<C> as Scheme>::Public,
        msg: &[u8],
        partial: &[u8],
    ) -> Result<()> {
        let partial: Eval<Vec<u8>> = bincode::deserialize(partial)?;
        G2Scheme::<C>::verify(partial_public_key, msg, &partial.value)?;
        Ok(())
    }

    fn aggregate(threshold: usize, partials: &[Vec<u8>]) -> Result<Vec<u8>> {
        let signature = G2Scheme::<C>::aggregate(threshold, partials)?;
        Ok(signature)
    }

    fn verify(public: &<G2Scheme<C> as Scheme>::Public, msg: &[u8], sig: &[u8]) -> Result<()> {
        G2Scheme::<C>::verify(public, msg, sig)?;
        Ok(())
    }

    fn verify_partial_sigs(
        publics: &[<G2Scheme<C> as Scheme>::Public],
        msg: &[u8],
        partial_sigs: &[&[u8]],
    ) -> Result<()> {
        G2Scheme::<C>::aggregation_verify_on_the_same_msg(publics, msg, partial_sigs)?;
        Ok(())
    }
}
