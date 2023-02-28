#[cfg(test)]
pub mod tests {
    use ark_ec::ProjectiveCurve;
    use ark_serialize::CanonicalSerialize;
    use ethers_core::{types::U256, utils::hex};

    use crate::curve::bn254::PairingCurve;
    use crate::group::{Element, Scalar};
    use crate::poly::Eval;
    use crate::{
        curve::bn254::{G2Curve, G1, G2},
        group::Curve,
        poly::{Idx, Poly},
        schemes::bn254::G2Scheme as SigScheme,
        sig::{G2Scheme, Scheme, Share, SignatureScheme, ThresholdScheme},
    };

    #[test]
    fn test_dkg_bls_over_bn254() {
        let seed_arr = [
            "2bdda4dd3ed74cb9ed050c6b6144174040215e9b997cc13ef743048709027044",
            "adba1e0e95a816030c7ab73a0ac0d519e51bb207562b225c169ba4fca9708aba",
            "2aba4fa6a8d715baf9a8e4965940c548f9d7a715093459ab3746a5516a7b8317",
            "8c766583fa9c65789080aee687f24be92eaa68775d583eccbc54468f21b7fd19",
            "772e33591d14ca86452b2fa0b6c1f50e9a89eebc9d806aaebb6f825169f65639",
            "8a767bde8c14db8d024b7b6dd98163398f60d86bc9a4ac814a213db6f489975d",
            "13c4fe7022314f4e35623d61d30f71de31192dca0a19898ff36af659e969e7a1",
            "249b38981df5bcd673f3bd50631aba97c5cc5b0dc7a85c94f11705ba950f5b0e",
            "ee37dd8f5b0b97804d115493672502eb373eca189306b49126816808ac2adcb8",
            "ccee934dfb6e63a7d154fb8cbbe6444659cf6d493b125bba2a931ce22414ddf8",
            "c8bc8d6c91c5384f0cf6a93dceec05e9580951bf07d8d4839242ac4a566e0e02",
            "d1c2a1c7ae81259aae2c94abf016febc938fdda8fd917fb88c50b3c9613c2b3a",
            "f249c2d5758f9d37d62f3efc5e26a2509dd97a154b0daf661a4831b5b339fa1c",
        ];

        let block_num_arr = [1, 18, 35, 52, 69, 86, 103, 120, 137, 154, 1, 1, 1];

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
        (0..n).for_each(|i| {
            let eval = public_poly.eval(i as Idx);
            println!(
                "member {} pubkey: {:?}",
                i,
                hex::encode(bincode::serialize(&eval.value).unwrap())
            );
        });
        println!("");

        let threshold_public_key = public_poly.public_key();

        println!(
            "threshold_public_key: {:?}",
            hex::encode(bincode::serialize(threshold_public_key).unwrap())
        );
        // print_g2_point(threshold_public_key);
        println!("");

        for i in 0..seed_arr.len() {
            let seed = hex::decode(seed_arr[i]).unwrap();
            let block_num = U256::from(block_num_arr[i]);
            let mut block_num_bytes = vec![0u8; 32];
            block_num.to_big_endian(&mut block_num_bytes);

            // Generate the partial signatures
            let msg = [seed, block_num_bytes].concat();
            println!("msg: {:?}", hex::encode(&msg));
            // hex::decode("de7c516e867a427226866fa41566ad0eb0ff69f54e4408babd2f59c54238fa860000000000000000000000000000000000000000000000000000000000000001")
            // .unwrap();

            let partials = shares
                .iter()
                .map(|s| SigScheme::partial_sign(s, &msg[..]).unwrap())
                .collect::<Vec<_>>();

            // each partial sig can be partially verified against the public polynomial
            partials.iter().enumerate().for_each(|(i, partial)| {
                let eval: Eval<Vec<u8>> = bincode::deserialize(partial).unwrap();
                println!(
                    "member {} partial signature: {:?}",
                    i,
                    hex::encode(&eval.value)
                );
                SigScheme::partial_verify(&public_poly, &msg[..], &partial).unwrap();
            });

            // generate the threshold sig
            let threshold_sig = SigScheme::aggregate(t, &partials).unwrap();

            SigScheme::verify(&threshold_public_key, &msg[..], &threshold_sig).unwrap();

            println!("signature: {:?}", hex::encode(&threshold_sig));
            // let sig_point: G1 = bincode::deserialize(&threshold_sig).unwrap();
            // print_g1_point(&sig_point);
            println!("");
        }
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
