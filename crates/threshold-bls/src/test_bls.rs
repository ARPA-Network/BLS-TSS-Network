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
            "de7c516e867a427226866fa41566ad0eb0ff69f54e4408babd2f59c54238fa86",
            "d33ade962639dfbe2c2cb144c8b81b9b41426183f4a8f9d97a3797267085e3e9",
            "d43a01309e5ce06f563855ea4d63bfd19566144d62073344ca66a52fe459fcd2",
            "edea6587954fc90bf55a2e710f2edda6d40cbc59050201e7e04b44af906eb1dc",
            "3884a5de852ba49a376e504abed8501ce14c9fd7d0a8a4aaca2a81d9d87b35da",
            "5e834c9e0ee3c1df597ce48258054d49ed258ecb6d31fbce6a78be9daf9afaa1",
            "e0ab0f3bc77b9c2b203d07aaa5af51280d8bba6c5bb1cd54763959c1b8770854",
            "428874369318654d220b552cd69526384de1a9854b9a194f3ee725424d32e4bd",
            "640670d5370a0b1142d202dd8d733cdc5317af68692ed48653ec348d2432c6f6",
            "1e049c83faf1abc374ad113f0e50995b4ed8a5be475156a346e773bb0d5cea7d",
            "912879b65da7afad27afb740fed587019e3aac62a0731fcdb33a38a5e04a9dbc",
            "34fa9bc41a34b4a1fb4dc01764f8f0cc61f6fc90284a603c49e393c220213552",
            "a4371e17c84d644e0823a95223011a30b995e36afd180b719769ad6ce88ceeae",
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

        println!("// Node Partial Public Keys");

        (0..n).for_each(|i| {
            let eval = public_poly.eval(i as Idx);
            println!(
                "bytes partialPublicKey{} = hex{:?};",
                i + 1,
                hex::encode(bincode::serialize(&eval.value).unwrap())
            );
        });
        println!("bytes badKey = \
        hex\"111111550230862eccaa516975047156d5c7cdc299f5fce0aaf04e9246c1ab2122f8c83061984377026e4769de7cc228004221275241ee6a33622043a3c730fc183f7bff0be8b3e21d9d56bc5ed2566ce3193c9df3396bd8cdc457e7c57ecbc010092c9cf423391bff81f73b1b33ac475dbf2b941b23acc7aa26324a57e5951b\";");
        println!("");

        let threshold_public_key = public_poly.public_key();

        println!("// Group Public Key");
        println!(
            "bytes publicKey = hex{:?};",
            hex::encode(bincode::serialize(threshold_public_key).unwrap())
        );
        // print_g2_point(threshold_public_key);
        println!("");
        println!("// Partial Signatures");

        for i in 0..seed_arr.len() {
            let seed = hex::decode(seed_arr[i]).unwrap();
            let block_num = U256::from(block_num_arr[i]);
            let mut block_num_bytes = vec![0u8; 32];
            block_num.to_big_endian(&mut block_num_bytes);

            // Generate the partial signatures
            let msg = [seed, block_num_bytes].concat();
            println!("// msg: {:?}", hex::encode(&msg));
            // hex::decode("de7c516e867a427226866fa41566ad0eb0ff69f54e4408babd2f59c54238fa860000000000000000000000000000000000000000000000000000000000000001")
            // .unwrap();

            let partials = shares
                .iter()
                .map(|s| SigScheme::partial_sign(s, &msg[..]).unwrap())
                .collect::<Vec<_>>();

            // each partial sig can be partially verified against the public polynomial
            partials.iter().enumerate().for_each(|(j, partial)| {
                let eval: Eval<Vec<u8>> = bincode::deserialize(partial).unwrap();
                println!(
                    "uint256 partial{}_{} = 0x{};",
                    i + 1,
                    j + 1,
                    hex::encode(&eval.value)
                );
                SigScheme::partial_verify(&public_poly, &msg[..], &partial).unwrap();
            });

            // generate the threshold sig
            let threshold_sig = SigScheme::aggregate(t, &partials).unwrap();

            SigScheme::verify(&threshold_public_key, &msg[..], &threshold_sig).unwrap();

            println!(
                "uint256 signature{} = 0x{};",
                i + 1,
                hex::encode(&threshold_sig)
            );
            println!(
                "uint256[] sig{} = [signature{}, partial{}_1, partial{}_2, partial{}_3, partial{}_4, partial{}_5];",i + 1,i + 1,i + 1,i + 1,i + 1,i + 1,i + 1
            );
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
