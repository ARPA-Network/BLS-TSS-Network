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
            "03c3e92afb3f0269f37ea28e202bbd315b1d709cc01f2c2afe2805fd4c80d2d4",
            "84a4be63154932c9cca9a34f9a931f1f0fba5d28f76c67f8f192fc9a3f66920d",
            "226a2536aeefd4a8acc9c5ce74f8b152a782214ad8f341886c9530a0ebbcf42f",
            "75fc83a48d1fac46f0d87e3752dbbf391ce3e3158f954232b10b8662598d9ee5",
            "522f5dc85539775dc0ab01953f6a286c7ade1d2555cd84e69c26e3b783c61efe",
            "35923fa15bca24d755a28af8fc280d76aef33f7f2a2a13e1e3de99733f345789",
            "f87a5f99c8a4fc89828ebb2fc631a9189b5da12804c93ebf2b3343c303957fe1",
            "002bb5e97e37f9b8f0216be6ae850c12105593b743bdd56972a498347bd9d6e8",
            "66ecc5843bea2256c4464dae004e7f6335cd6da72bcc8fa905f6a950a361f948",
            "c8eaa51e153e39c71e3bece1af0856a976ed93ab5a4bf4b17f5a2070e480e97b",
            "6ddcbff04cf4d7733db9b763d93c9c39c11097ae2121800d0d1f2c94d531c8bf",
            "4cb0e29a4928e365cbe774e0958305b6879ab501c8398fc2394d928ec324ff67",
            "2a276927aef9a0cb4ae82727c188b2988e782e2e94d8727984b6e76463ae9dea",
            "03c3e92afb3f0269f37ea28e202bbd315b1d709cc01f2c2afe2805fd4c80d2d4",
            "6ddcbff04cf4d7733db9b763d93c9c39c11097ae2121800d0d1f2c94d531c8bf",
            "4cb0e29a4928e365cbe774e0958305b6879ab501c8398fc2394d928ec324ff67",
            "6528885a4c73f9fca5701f4d6c75201267e8f0036e1f5c1ab4c130f655377f94",
            "6ddcbff04cf4d7733db9b763d93c9c39c11097ae2121800d0d1f2c94d531c8bf",
            "9df2e3aabb7acfb198c4e3852c5899068d63ae9a4f30ff0cccc124b207e54663",
        ];

        let block_num_arr = [
            1, 18, 35, 52, 69, 86, 103, 120, 137, 154, 1, 1, 1, 1, 1, 1, 1, 1, 1
        ];

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
                "bytes internal _partialPublicKey{} = hex{:?};",
                i + 1,
                hex::encode(bincode::serialize(&eval.value).unwrap())
            );
        });
        println!("bytes internal _badKey = \
        hex\"111111550230862eccaa516975047156d5c7cdc299f5fce0aaf04e9246c1ab2122f8c83061984377026e4769de7cc228004221275241ee6a33622043a3c730fc183f7bff0be8b3e21d9d56bc5ed2566ce3193c9df3396bd8cdc457e7c57ecbc010092c9cf423391bff81f73b1b33ac475dbf2b941b23acc7aa26324a57e5951b\";");
        println!("");

        let threshold_public_key = public_poly.public_key();

        println!("// Group Public Key");
        println!(
            "bytes internal _publicKey = hex{:?};",
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
                    "uint256 internal _partialG{}I{} = 0x{};",
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
                "uint256 internal _signature{} = 0x{};",
                i + 1,
                hex::encode(&threshold_sig)
            );
            println!(
                "uint256[] internal _sig{} = [_signature{}, _partialG{}I1, _partialG{}I2, _partialG{}I3, _partialG{}I4, _partialG{}I5];",i + 1,i + 1,i + 1,i + 1,i + 1,i + 1,i + 1
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
