#[cfg(test)]
pub mod tests {
    use ark_ec::ProjectiveCurve;
    use ark_serialize::CanonicalSerialize;
    use ethers_core::{types::U256, utils::hex};

    use crate::curve::bn254::PairingCurve;
    use crate::group::{Element, Scalar};
    use crate::{
        curve::bn254::{G2Curve, G1, G2},
        group::Curve,
        poly::{Idx, Poly},
        schemes::bn254::G2Scheme as SigScheme,
        sig::{G2Scheme, Scheme, Share, SignatureScheme, ThresholdScheme},
    };

    #[test]
    fn test_dkg_bls_over_bn254() {
        let (n, t) = (5, 3);
        // create the private key polynomial
        let private_poly = Poly::<<SigScheme as Scheme>::Private>::new(t - 1);

        // Evaluate it at `n` points to generate the shares
        let shares = (0..n)
            .map(|i| {
                let eval = private_poly.eval(i as Idx);
                Share {
                    index: eval.index,
                    private: eval.value,
                }
            })
            .collect::<Vec<_>>();

        // Get the public polynomial
        let public_poly = private_poly.commit();
        let threshold_public_key = public_poly.public_key();

        // Generate the partial signatures
        let msg = hex::decode("de7c516e867a427226866fa41566ad0eb0ff69f54e4408babd2f59c54238fa860000000000000000000000000000000000000000000000000000000000000001")
            .unwrap();

        let partials = shares
            .iter()
            .map(|s| SigScheme::partial_sign(s, &msg[..]).unwrap())
            .collect::<Vec<_>>();

        // each partial sig can be partially verified against the public polynomial
        partials.iter().for_each(|partial| {
            SigScheme::partial_verify(&public_poly, &msg[..], &partial).unwrap();
        });

        // generate the threshold sig
        let threshold_sig = SigScheme::aggregate(t, &partials).unwrap();

        SigScheme::verify(&threshold_public_key, &msg[..], &threshold_sig).unwrap();

        println!("threshold_public_key: {}", threshold_public_key.0);
        print_g2_point(threshold_public_key);
        println!("signature: {:?}", hex::encode(&threshold_sig));
        let sig_point: G1 = bincode::deserialize(&threshold_sig).unwrap();
        print_g1_point(&sig_point);
        println!("finish.");
    }

    #[test]
    fn test_sign() {
        let (private, public) = keypair::<G2Curve>();
        let msg = hex::decode("de7c516e867a427226866fa41566ad0eb0ff69f54e4408babd2f59c54238fa860000000000000000000000000000000000000000000000000000000000000001")
            .unwrap();
        let sig = G2Scheme::<PairingCurve>::sign(&private, &msg).unwrap();
        SigScheme::verify(&public, &msg[..], &sig).unwrap();

        println!("public_key: {}", public.0);
        print_g2_point(&public);
        println!("signature: {:?}", hex::encode(&sig));
        let sig_point: G1 = bincode::deserialize(&sig).unwrap();
        print_g1_point(&sig_point);
    }

    fn keypair<C: Curve>() -> (C::Scalar, C::Point) {
        let mut private = C::Scalar::new();
        private.set_int(42);
        let mut public = C::Point::one();
        public.mul(&private);
        (private, public)
    }

    fn print_g2_affine(xy: &str, bytes: &[u8]) {
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

    fn print_g1_affine(xy: &str, bytes: &[u8]) {
        let x1 = &mut bytes[..].to_vec();

        x1.reverse();

        println!("{}", xy);
        // Hex
        println!("{:?}", hex::encode(x1.clone()));
        // Dec
        println!("{:?}", U256::from(&x1 as &[u8]));
        println!("");
    }

    fn print_g2_point(p: &G2) {
        let mut xbytes = vec![];
        p.0.into_affine().x.serialize(&mut xbytes).unwrap();
        let mut ybytes = vec![];
        p.0.into_affine().y.serialize(&mut ybytes).unwrap();
        print_g2_affine("x", &xbytes);
        print_g2_affine("y", &ybytes);
    }

    fn print_g1_point(p: &G1) {
        let mut xbytes = vec![];
        p.0.into_affine().x.serialize(&mut xbytes).unwrap();
        let mut ybytes = vec![];
        p.0.into_affine().y.serialize(&mut ybytes).unwrap();
        print_g1_affine("x", &xbytes);
        print_g1_affine("y", &ybytes);
    }
}
