#[cfg(test)]
pub mod tests {
    use crate::{
        poly::{Idx, Poly},
        schemes::bls12_381::G1Scheme as SigScheme,
        sig::{Scheme, Share, SignatureScheme, ThresholdScheme},
    };

    #[test]
    fn test_bls381() {
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
        let msg = b"hello";

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
        println!("{:#?}", threshold_sig);

        SigScheme::verify(&threshold_public_key, &msg[..], &threshold_sig).unwrap();
        println!("finish.")
    }
}
