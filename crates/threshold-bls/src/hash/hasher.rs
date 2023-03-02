use crate::curve::BLSError;

pub trait Hasher {
    type Error;

    fn hash(&self, domain: &[u8], message: &[u8]) -> Result<Vec<u8>, Self::Error>;
}

#[derive(Debug, Clone, Copy)]
pub struct Keccak256Hasher;

impl Hasher for Keccak256Hasher {
    type Error = BLSError;

    fn hash(&self, _domain: &[u8], message: &[u8]) -> Result<Vec<u8>, Self::Error> {
        Ok(ethers_core::utils::keccak256(message).into())
    }
}

#[cfg(test)]
pub mod tests {
    use ethers_core::utils::hex;

    use super::{Hasher, Keccak256Hasher};

    #[test]
    fn test_hash() {
        let hasher = Keccak256Hasher;
        let message = b"hello";
        let res = hasher.hash(&[], message).unwrap();
        assert_eq!(
            hex::encode(res),
            "1c8aff950685c2ed4bc3174f3472287b56d9517b9c948127319a09a7a36deac8"
        );
    }
}
