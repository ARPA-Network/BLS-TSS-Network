use crate::curve::{bn254, BLSError};
use crate::group::{Point, Scalar};
use ark_ec::{AffineCurve, ModelParameters, ProjectiveCurve};
use ark_ff::Field;
use ark_ff::PrimeField;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ethers_core::utils::hex;

pub trait ContractSerialize: Sized {
    /// Serialize the group element into a byte vector.
    fn serialize_to_contract_form(&self) -> Result<Vec<u8>, BLSError>;
    /// Deserialize the group element from a byte vector.
    fn deserialize_from_contract_form(bytes: &[u8]) -> Result<Self, BLSError>;
}

impl ContractSerialize for bn254::G1 {
    fn serialize_to_contract_form(&self) -> Result<Vec<u8>, BLSError> {
        let mut xbytes = vec![];
        self.0.into_affine().serialize(&mut xbytes).unwrap();

        xbytes.reverse();

        Ok(xbytes)
    }

    fn deserialize_from_contract_form(bytes: &[u8]) -> Result<Self, BLSError> {
        let mut ele_bytes = bytes.to_vec();
        ele_bytes.reverse();

        let affine = ark_bn254::G1Affine::deserialize(&mut &ele_bytes[..])
            .map_err(|_| BLSError::ContractSerializationError)?;

        Ok(bn254::G1(affine.into_projective()))
    }
}

impl ContractSerialize for bn254::G2 {
    fn serialize_to_contract_form(&self) -> Result<Vec<u8>, BLSError> {
        let mut xbytes = vec![];
        self.0.into_affine().x.serialize(&mut xbytes).unwrap();

        let mut x1 = xbytes[..32].to_vec();
        let mut x2 = xbytes[32..].to_vec();

        x1.reverse();
        x2.reverse();

        let mut ybytes = vec![];
        self.0.into_affine().y.serialize(&mut ybytes).unwrap();

        let mut y1 = ybytes[..32].to_vec();
        let mut y2 = ybytes[32..].to_vec();

        y1.reverse();
        y2.reverse();

        let bytes = [&x1[..], &x2[..], &y1[..], &y2[..]].concat();

        Ok(bytes)
    }

    fn deserialize_from_contract_form(bytes: &[u8]) -> Result<Self, BLSError> {
        let mut x1 = bytes[..32].to_vec();
        let mut x2 = bytes[32..64].to_vec();

        let f_y1 =
            <<ark_bn254::g2::Parameters as ModelParameters>::BaseField as Field>::BasePrimeField::from_be_bytes_mod_order(
                &bytes[64..96],
            );

        let f_y2 =
            <<ark_bn254::g2::Parameters as ModelParameters>::BaseField as Field>::BasePrimeField::from_be_bytes_mod_order(
                &bytes[96..],
            );

        let f_y =
            <ark_bn254::g2::Parameters as ModelParameters>::BaseField::from_base_prime_field_elems(
                &[f_y1, f_y2],
            )
            .ok_or(BLSError::NotValidPoint)?;

        if f_y > -f_y {
            x2[0] |= 1 << 7;
        }

        x1.reverse();
        x2.reverse();

        let bytes = [&x1[..], &x2[..]].concat();

        let affine = ark_bn254::G2Affine::deserialize(&mut &bytes[..])
            .map_err(|_| BLSError::ContractSerializationError)?;

        Ok(bn254::G2(affine.into_projective()))
    }
}

pub fn scalar_to_hex<S: Scalar>(s: &S) -> String {
    let bytes = bincode::serialize(s).unwrap();
    hex::encode(bytes)
}

pub fn point_to_hex<P: Point>(p: &P) -> String {
    let bytes = bincode::serialize(p).unwrap();
    hex::encode(bytes)
}

#[cfg(test)]
pub mod tests {
    use crate::{group::Element, serialize::ContractSerialize};
    use rand::thread_rng;

    #[test]
    fn test_serialize_bn254_g1_element() {
        use crate::curve::bn254::G1;

        for _ in 0..10 {
            let g1 = G1::rand(&mut thread_rng());

            let g1_bytes = g1.serialize_to_contract_form().unwrap();

            let g1_deserialized = G1::deserialize_from_contract_form(&g1_bytes).unwrap();

            assert_eq!(g1, g1_deserialized);
        }
    }

    #[test]
    fn test_serialize_bn254_g2_element() {
        use crate::curve::bn254::G2;

        for _ in 0..10 {
            let g2 = G2::rand(&mut thread_rng());

            let g2_bytes = g2.serialize_to_contract_form().unwrap();

            let g2_deserialized = G2::deserialize_from_contract_form(&g2_bytes).unwrap();

            assert_eq!(g2, g2_deserialized);
        }
    }
}
