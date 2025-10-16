pub mod hasher;
pub mod try_and_increment;
use crate::curve::BLSError;

/// Trait for hashing arbitrary data to a group element on an elliptic curve
pub trait HashToCurve {
    /// The type of the curve being used.
    type Output;

    /// Given a domain separator and a message, produces
    /// a hash of them which is a curve point.
    fn hash(&self, domain: &[u8], message: &[u8]) -> Result<Self::Output, BLSError>;
}

#[cfg(test)]
mod test {

    use super::{
        hasher::{Hasher, Keccak256Hasher},
        try_and_increment::TryAndIncrement,
        *,
    };
    use alloy::{hex, primitives::U256};
    use ark_bn254::g2::Config as G2Config;
    use ark_ec::short_weierstrass::SWCurveConfig;
    // use ark_ec::{
    //     bn::BnParameters, models::short_weierstrass::SWModelParameters,
    //     short_weierstrass::SWCurveConfig, ProjectiveCurve,
    // };
    use ark_serialize::CanonicalSerialize;

    #[test]
    fn hash_to_curve_direct_g1() {
        let h = Keccak256Hasher;
        // hash_to_curve_test::<_, <Parameters as BnParameters>::G1Parameters>(h, b"hello");
        hash_to_curve_test::<_, G2Config>(h, b"hello01");
        hash_to_curve_test::<_, G2Config>(h, b"hello02");
        hash_to_curve_test::<_, G2Config>(h, b"hello03");
        hash_to_curve_test::<_, G2Config>(h, b"hello04");
        hash_to_curve_test::<_, G2Config>(h, b"hello05");
    }

    fn hash_to_curve_test<X: Hasher<Error = BLSError>, P: SWCurveConfig>(h: X, input: &[u8]) {
        let hasher = TryAndIncrement::<X, P>::new(&h);
        let g = hasher.hash(&[], input).unwrap();

        let mut xbytes = vec![];
        g.x.serialize_compressed(&mut xbytes).unwrap();
        println!("{}", g);
        let mut ybytes = vec![];
        g.y.serialize_compressed(&mut ybytes).unwrap();
        print_point("x", &xbytes);
        print_point("y", &ybytes);
    }

    fn print_point(xy: &str, bytes: &[u8]) {
        let x1 = &mut bytes[..32].to_vec();
        let x2 = &mut bytes[32..].to_vec();

        x1.reverse();
        x2.reverse();

        println!("{}", xy);
        // Hex
        print!("{:?}", hex::encode(x1.clone()));
        print!(" ");
        println!("{:?}", hex::encode(x2.clone()));
        // Dec
        print!("{:?}", U256::from_be_slice(&x1));
        print!(" ");
        println!("{:?}", U256::from_be_slice(&x2));
        println!("");
    }
}
