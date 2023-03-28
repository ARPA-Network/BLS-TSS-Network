use crate::group::{self, Element, PairingCurve as PC, Point, Scalar as Sc};
use crate::hash::hasher::Keccak256Hasher;
use crate::hash::try_and_increment::TryAndIncrement;
use crate::hash::HashToCurve;
use crate::serialize::ContractSerialize;
use ark_bn254 as bn254;
use ark_ec::{PairingEngine, ProjectiveCurve};
use ark_ff::PrimeField;
use ark_ff::{Field, One, UniformRand, Zero};
use rand_core::RngCore;
use serde::{
    de::{Error as DeserializeError, SeqAccess, Visitor},
    ser::{Error as SerializationError, SerializeTuple},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::{
    fmt,
    marker::PhantomData,
    ops::{AddAssign, MulAssign, Neg, SubAssign},
};

use thiserror::Error;

use super::{BLSError, CurveType};

#[derive(Debug, Error)]
pub enum BNError {
    #[error("{0}")]
    SerializationError(#[from] ark_serialize::SerializationError),
    #[error("{0}")]
    BLSError(#[from] BLSError),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
pub struct Scalar(
    #[serde(deserialize_with = "deserialize_field")]
    #[serde(serialize_with = "serialize_field")]
    <bn254::Bn254 as PairingEngine>::Fr,
);

type ZG1 = <bn254::Bn254 as PairingEngine>::G1Projective;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct G1(pub(crate) ZG1);

type ZG2 = <bn254::Bn254 as PairingEngine>::G2Projective;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct G2(pub(crate) ZG2);

impl Serialize for G1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let bytes = self
            .serialize_to_contract_form()
            .map_err(SerializationError::custom)?;

        let mut tup = serializer.serialize_tuple(32)?;
        for byte in &bytes {
            tup.serialize_element(byte)?;
        }
        tup.end()
    }
}
impl<'de> Deserialize<'de> for G1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct G1Visitor;

        impl<'de> Visitor<'de> for G1Visitor {
            type Value = G1;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a valid group element")
            }

            fn visit_seq<S>(self, mut seq: S) -> Result<Self::Value, S::Error>
            where
                S: SeqAccess<'de>,
            {
                let bytes: Vec<u8> = (0..32)
                    .map(|_| {
                        seq.next_element()?
                            .ok_or_else(|| DeserializeError::custom("could not read bytes"))
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                let ele =
                    G1::deserialize_from_contract_form(&bytes).map_err(DeserializeError::custom)?;
                Ok(ele)
            }
        }

        deserializer.deserialize_tuple(32, G1Visitor)
    }
}

impl Serialize for G2 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let bytes = self
            .serialize_to_contract_form()
            .map_err(SerializationError::custom)?;

        let mut tup = serializer.serialize_tuple(128)?;
        for byte in &bytes {
            tup.serialize_element(byte)?;
        }
        tup.end()
    }
}
impl<'de> Deserialize<'de> for G2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct G2Visitor;

        impl<'de> Visitor<'de> for G2Visitor {
            type Value = G2;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a valid group element")
            }

            fn visit_seq<S>(self, mut seq: S) -> Result<Self::Value, S::Error>
            where
                S: SeqAccess<'de>,
            {
                let bytes: Vec<u8> = (0..128)
                    .map(|_| {
                        seq.next_element()?
                            .ok_or_else(|| DeserializeError::custom("could not read bytes"))
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                let ele =
                    G2::deserialize_from_contract_form(&bytes).map_err(DeserializeError::custom)?;
                Ok(ele)
            }
        }

        deserializer.deserialize_tuple(128, G2Visitor)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub struct GT(
    #[serde(deserialize_with = "deserialize_field")]
    #[serde(serialize_with = "serialize_field")]
    <bn254::Bn254 as PairingEngine>::Fqk,
);

impl Element for Scalar {
    type RHS = Scalar;

    fn new() -> Self {
        Self(Zero::zero())
    }

    fn one() -> Self {
        Self(One::one())
    }

    fn add(&mut self, s2: &Self) {
        self.0.add_assign(s2.0);
    }

    fn mul(&mut self, mul: &Scalar) {
        self.0.mul_assign(mul.0)
    }

    fn rand<R: rand_core::RngCore>(rng: &mut R) -> Self {
        Self(bn254::Fr::rand(rng))
    }
}

impl Sc for Scalar {
    fn set_int(&mut self, i: u64) {
        *self = Self(bn254::Fr::from(i))
    }

    fn inverse(&self) -> Option<Self> {
        Some(Self(Field::inverse(&self.0)?))
    }

    fn negate(&mut self) {
        *self = Self(self.0.neg())
    }

    fn sub(&mut self, other: &Self) {
        self.0.sub_assign(other.0);
    }
}

impl fmt::Display for Scalar {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{{:?}}}", self.0)
    }
}

/// G1 points can be multiplied by Fr elements
impl Element for G1 {
    type RHS = Scalar;

    fn new() -> Self {
        Self(Zero::zero())
    }

    fn one() -> Self {
        Self(ZG1::prime_subgroup_generator())
    }

    fn rand<R: RngCore>(rng: &mut R) -> Self {
        Self(ZG1::rand(rng))
    }

    fn add(&mut self, s2: &Self) {
        self.0.add_assign(s2.0);
    }

    fn mul(&mut self, mul: &Scalar) {
        self.0.mul_assign(mul.0);
    }
}

/// Implementation of Point using G1 from BN254
impl Point for G1 {
    type Error = BNError;

    fn map(&mut self, data: &[u8]) -> Result<(), BNError> {
        let hasher = TryAndIncrement::new(&Keccak256Hasher);

        let hash = hasher.hash(&[], data)?;

        *self = Self(hash);

        Ok(())
    }
}

impl fmt::Display for G1 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{{:?}}}", self.0)
    }
}

/// G1 points can be multiplied by Fr elements
impl Element for G2 {
    type RHS = Scalar;

    fn new() -> Self {
        Self(Zero::zero())
    }

    fn one() -> Self {
        Self(ZG2::prime_subgroup_generator())
    }

    fn rand<R: RngCore>(mut rng: &mut R) -> Self {
        Self(ZG2::rand(&mut rng))
    }

    fn add(&mut self, s2: &Self) {
        self.0.add_assign(s2.0);
    }

    fn mul(&mut self, mul: &Scalar) {
        self.0.mul_assign(mul.0)
    }
}

/// Implementation of Point using G2 from BN254
impl Point for G2 {
    type Error = BNError;

    fn map(&mut self, data: &[u8]) -> Result<(), BNError> {
        let hasher = TryAndIncrement::new(&Keccak256Hasher);

        let hash = hasher.hash(&[], data)?;

        *self = Self(hash);

        Ok(())
    }
}

impl fmt::Display for G2 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{{:?}}}", self.0)
    }
}

impl Element for GT {
    type RHS = Scalar;

    fn new() -> Self {
        Self(One::one())
    }
    fn one() -> Self {
        Self(One::one())
    }
    fn add(&mut self, s2: &Self) {
        self.0.mul_assign(s2.0);
    }
    fn mul(&mut self, mul: &Scalar) {
        let scalar = mul.0.into_repr();
        let mut res = Self::one();
        let mut temp = *self;
        for b in ark_ff::BitIteratorLE::without_trailing_zeros(scalar) {
            if b {
                res.0.mul_assign(temp.0);
            }
            temp.0.square_in_place();
        }
        *self = res;
    }
    fn rand<R: RngCore>(rng: &mut R) -> Self {
        Self(bn254::Fq12::rand(rng))
    }
}

impl fmt::Display for GT {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{{:?}}}", self.0)
    }
}

pub type G1Curve = group::G1Curve<PairingCurve>;
pub type G2Curve = group::G2Curve<PairingCurve>;

#[derive(Clone, Debug, Serialize)]
pub struct PairingCurve;

impl PC for PairingCurve {
    type Scalar = Scalar;
    type G1 = G1;
    type G2 = G2;
    type GT = GT;

    fn pair(a: &Self::G1, b: &Self::G2) -> Self::GT {
        GT(<bn254::Bn254 as PairingEngine>::pairing(a.0, b.0))
    }
}

// Serde implementations (ideally, these should be upstreamed to Zexe)

fn deserialize_field<'de, D, C>(deserializer: D) -> Result<C, D::Error>
where
    D: Deserializer<'de>,
    C: Field,
{
    struct FieldVisitor<C>(PhantomData<C>);

    impl<'de, C> Visitor<'de> for FieldVisitor<C>
    where
        C: Field,
    {
        type Value = C;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a valid group element")
        }

        fn visit_seq<S>(self, mut seq: S) -> Result<C, S::Error>
        where
            S: SeqAccess<'de>,
        {
            let len = C::zero().serialized_size();
            let bytes: Vec<u8> = (0..len)
                .map(|_| {
                    seq.next_element()?
                        .ok_or_else(|| DeserializeError::custom("could not read bytes"))
                })
                .collect::<Result<Vec<_>, _>>()?;

            let res = C::deserialize(&mut &bytes[..]).map_err(DeserializeError::custom)?;
            Ok(res)
        }
    }

    let visitor = FieldVisitor(PhantomData);
    deserializer.deserialize_tuple(C::zero().serialized_size(), visitor)
}

fn serialize_field<S, C>(c: &C, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    C: Field,
{
    let len = c.serialized_size();
    let mut bytes = Vec::with_capacity(len);
    c.serialize(&mut bytes)
        .map_err(SerializationError::custom)?;

    let mut tup = s.serialize_tuple(len)?;
    for byte in &bytes {
        tup.serialize_element(byte)?;
    }
    tup.end()
}

#[derive(Clone, Debug)]
pub struct BN254Curve;

impl CurveType for BN254Curve {
    type G1Curve = G1Curve;

    type G2Curve = G2Curve;

    type PairingCurve = PairingCurve;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{de::DeserializeOwned, Serialize};
    use static_assertions::assert_impl_all;

    assert_impl_all!(G1: Serialize, DeserializeOwned, Clone);
    assert_impl_all!(G2: Serialize, DeserializeOwned, Clone);
    assert_impl_all!(GT: Serialize, DeserializeOwned, Clone);
    assert_impl_all!(Scalar: Serialize, DeserializeOwned, Clone);

    #[test]
    fn serialize_group() {
        for _ in 0..10 {
            serialize_group_test::<G1>(32);
            serialize_group_test::<G2>(128);
        }
    }

    fn serialize_group_test<E: Element>(size: usize) {
        let empty = bincode::deserialize::<E>(&vec![]);
        assert!(empty.is_err());

        let rng = &mut rand::thread_rng();
        let sig = E::rand(rng);
        let ser = bincode::serialize(&sig).unwrap();
        assert_eq!(ser.len(), size);

        let de: E = bincode::deserialize(&ser).unwrap();
        assert_eq!(de, sig);
    }

    #[test]
    fn serialize_field() {
        serialize_field_test::<GT>(384);
        serialize_field_test::<Scalar>(32);
    }

    fn serialize_field_test<E: Element>(size: usize) {
        let rng = &mut rand::thread_rng();
        let sig = E::rand(rng);
        let ser = bincode::serialize(&sig).unwrap();
        assert_eq!(ser.len(), size);

        let de: E = bincode::deserialize(&ser).unwrap();
        assert_eq!(de, sig);
    }

    #[test]
    fn gt_exp() {
        let rng = &mut rand::thread_rng();
        let base = GT::rand(rng);

        let mut sc = Scalar::one();
        sc.add(&Scalar::one());
        sc.add(&Scalar::one());

        let mut exp = base.clone();
        exp.mul(&sc);

        let mut res = base.clone();
        res.add(&base);
        res.add(&base);

        assert_eq!(exp, res);
    }
}
