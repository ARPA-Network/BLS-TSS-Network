use super::{hasher::Hasher, HashToCurve};
use crate::curve::BLSError;
use ark_ec::models::{
    short_weierstrass_jacobian::{GroupAffine, GroupProjective},
    SWModelParameters,
};
use ark_ff::{Field, PrimeField, Zero};
use ethers_core::utils::hex;
use log::debug;
use std::marker::PhantomData;

const NUM_TRIES: u8 = 255;

/// A try-and-increment method for hashing to G1 and G2.
#[derive(Clone)]
pub struct TryAndIncrement<'a, H, P> {
    hasher: &'a H,
    curve_params: PhantomData<P>,
}

impl<'a, H, P> TryAndIncrement<'a, H, P>
where
    H: Hasher<Error = BLSError>,
    P: SWModelParameters,
{
    /// Instantiates a new Try-and-increment hasher with the provided hashing method
    /// and curve parameters based on the type
    pub fn new(h: &'a H) -> Self {
        TryAndIncrement {
            hasher: h,
            curve_params: PhantomData,
        }
    }
}

impl<'a, H, P> HashToCurve for TryAndIncrement<'a, H, P>
where
    H: Hasher<Error = BLSError>,
    P: SWModelParameters,
{
    type Output = GroupProjective<P>;

    fn hash(&self, domain: &[u8], message: &[u8]) -> Result<Self::Output, BLSError> {
        self.hash_with_attempt(domain, message).map(|res| res.0)
    }
}

impl<'a, H, P> TryAndIncrement<'a, H, P>
where
    H: Hasher<Error = BLSError>,
    P: SWModelParameters,
{
    pub fn hash_with_attempt(
        &self,
        domain: &[u8],
        message: &[u8],
    ) -> Result<(GroupProjective<P>, usize), BLSError> {
        let mut candidate_hash = self.hasher.hash(domain, message)?;

        for c in 0..NUM_TRIES {
            let xfield = if P::BaseField::extension_degree() == 1 {
                // TODO BN254 G1 curve, currently we have the same simple implementation in contract
                let f = <P::BaseField as Field>::BasePrimeField::from_be_bytes_mod_order(
                    &candidate_hash,
                );
                P::BaseField::from_base_prime_field_elems(&[f])
            } else {
                P::BaseField::from_random_bytes(&candidate_hash)
            };

            if let Some(x) = xfield {
                if let Some(p) = GroupAffine::get_point_from_x(x, false) {
                    debug!(
                        "succeeded hashing \"{}\" to curve in {} tries",
                        hex::encode(message),
                        c + 1
                    );

                    let scaled = p.scale_by_cofactor();
                    if scaled.is_zero() {
                        continue;
                    }
                    return Ok((scaled, (c + 1) as usize));
                }
            }

            candidate_hash = self.hasher.hash(domain, &candidate_hash)?;
        }

        Err(BLSError::HashToCurveError)
    }
}
