use anyhow::Result;
use std::marker::PhantomData;
use threshold_bls::{
    group::Curve,
    poly::Eval,
    sig::{Share, SignatureScheme, ThresholdScheme},
};

pub(crate) struct SimpleBLSCore<
    C: Curve,
    S: SignatureScheme + ThresholdScheme<Public = C::Point, Private = C::Scalar>,
> {
    c: PhantomData<C>,
    s: PhantomData<S>,
}

pub(crate) trait BLSCore<C: Curve> {
    /// Partially signs a message with a share of the private key
    fn partial_sign(private: &Share<C::Scalar>, msg: &[u8]) -> Result<Vec<u8>>;

    /// Verifies a partial signature on a message against the public polynomial
    fn partial_verify(partial_public_key: &C::Point, msg: &[u8], partial: &[u8]) -> Result<()>;

    /// Aggregates all partials signature together. Note that this method does
    /// not verify if the partial signatures are correct or not; it only
    /// aggregates them.
    fn aggregate(threshold: usize, partials: &[Vec<u8>]) -> Result<Vec<u8>>;

    /// Verifies that the signature on the provided message was produced by the public key
    fn verify(public: &C::Point, msg: &[u8], sig: &[u8]) -> Result<()>;

    fn verify_partial_sigs(publics: &[C::Point], msg: &[u8], partial_sigs: &[&[u8]]) -> Result<()>;
}

impl<
        C: Curve + 'static,
        S: SignatureScheme + ThresholdScheme<Public = C::Point, Private = C::Scalar> + 'static,
    > BLSCore<C> for SimpleBLSCore<C, S>
where
    <S as ThresholdScheme>::Error: Sync + Send,
    <S as SignatureScheme>::Error: Sync + Send,
{
    fn partial_sign(private: &Share<C::Scalar>, msg: &[u8]) -> Result<Vec<u8>> {
        let partial_signature = S::partial_sign(private, msg)?;
        Ok(partial_signature)
    }

    fn partial_verify(partial_public_key: &C::Point, msg: &[u8], partial: &[u8]) -> Result<()> {
        let partial: Eval<Vec<u8>> = bincode::deserialize(partial)?;
        S::verify(partial_public_key, msg, &partial.value)?;
        Ok(())
    }

    fn aggregate(threshold: usize, partials: &[Vec<u8>]) -> Result<Vec<u8>> {
        let signature = S::aggregate(threshold, partials)?;
        Ok(signature)
    }

    fn verify(public: &C::Point, msg: &[u8], sig: &[u8]) -> Result<()> {
        S::verify(public, msg, sig)?;
        Ok(())
    }

    fn verify_partial_sigs(publics: &[C::Point], msg: &[u8], partial_sigs: &[&[u8]]) -> Result<()> {
        S::aggregation_verify_on_the_same_msg(publics, msg, partial_sigs)?;
        Ok(())
    }
}
