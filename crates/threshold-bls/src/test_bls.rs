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
            "381336e809b17f1073ab04bc3ec7f42467eecd3deb2cf2131afc144f3d7e3460",
            "b21e7b73da061425eaf25194a5948b8eb7de4b48bbb2d2e9288ae5ee6fca7f98",
            "b6341502d010740f87b23ef2726c0e20884c3f32013ba6905b22597c56118242",
            "38dde2f18019cfd2160a9efabe48fed1bf6c3224f055da89b50e2ca71c4bbc9a",
            "51f4382ec948c5ba7e06815ad3a20a45404f6035b602f5dfa39915765393d157",
            "fe4334e130fd43b7ad1f25d8c62cc345f9b9940c578ceb85421b58fb8d0a78dd",
            "9a30cc4ff58a5e8a12572ce431dc681f5bb03d9443816093dbd39d98b7a2264e",
            "adc74caff81ece16321a09d27b394c0f287db98851edb5219105e30a6b8b919f",
            "ce4b662b812b4166e4cf522b4322a7492c71222c2548db916c31de45470fce20",
            "d58a2946ea20724bdf25841dcc9f40da9353578fe4c917982d9d414a9306225e",
            "574d19ba255ab559ce104ba2f58150e9be94f7d40f7a482007b5953b8967629f",
            "e90f06a218986b04c8fcabf05d647937bdb828dd443bd70cffde5d0b2dbaa24d",
            "6bfff7ae4ac5e8c53e806d5377a559d40294301a8e5ae2fa462c176432d96c22",
            "381336e809b17f1073ab04bc3ec7f42467eecd3deb2cf2131afc144f3d7e3460",
            "e90f06a218986b04c8fcabf05d647937bdb828dd443bd70cffde5d0b2dbaa24d",
            "574d19ba255ab559ce104ba2f58150e9be94f7d40f7a482007b5953b8967629f",
            "612f9f32f1a2a5ffb5aa3ac2aa34e097bd2e8208940fa8feed3617b28b69a495",
        ];

        let block_num_arr = [
            1, 18, 35, 52, 69, 86, 103, 120, 137, 154, 1, 1, 1, 1, 1, 1, 1,
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
        println!("bytes badKey = \
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
