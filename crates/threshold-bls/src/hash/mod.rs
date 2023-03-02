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
    use ark_bn254::Parameters;
    use ark_ec::{bn::BnParameters, models::SWModelParameters, ProjectiveCurve};
    use ark_serialize::CanonicalSerialize;
    use ethers_core::{types::U256, utils::hex};

    #[test]
    fn hash_to_curve_direct_g1() {
        let h = Keccak256Hasher;
        // hash_to_curve_test::<_, <Parameters as BnParameters>::G1Parameters>(h, b"hello");
        hash_to_curve_test::<_, <Parameters as BnParameters>::G2Parameters>(h, b"hello01");
        hash_to_curve_test::<_, <Parameters as BnParameters>::G2Parameters>(h, b"hello02");
        hash_to_curve_test::<_, <Parameters as BnParameters>::G2Parameters>(h, b"hello03");
        hash_to_curve_test::<_, <Parameters as BnParameters>::G2Parameters>(h, b"hello04");
        hash_to_curve_test::<_, <Parameters as BnParameters>::G2Parameters>(h, b"hello05");
    }

    fn hash_to_curve_test<X: Hasher<Error = BLSError>, P: SWModelParameters>(h: X, input: &[u8]) {
        let hasher = TryAndIncrement::<X, P>::new(&h);
        let g = hasher.hash(&[], input).unwrap();

        let mut xbytes = vec![];
        g.into_affine().x.serialize(&mut xbytes).unwrap();
        println!("{}", g);
        let mut ybytes = vec![];
        g.into_affine().y.serialize(&mut ybytes).unwrap();
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
        print!("{:?}", U256::from(&x1 as &[u8]));
        print!(" ");
        println!("{:?}", U256::from(&x2 as &[u8]));
        println!("");
    }
}
