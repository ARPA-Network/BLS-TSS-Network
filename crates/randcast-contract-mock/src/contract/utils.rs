use ethers_core::types::{Address, U256};
use ethers_core::utils::keccak256;

// Choose "count" random indices from "indices" array.
pub fn choose_randomly_from_indices(seed: U256, indices: &[usize], count: usize) -> Vec<usize> {
    let mut chosen_indices = vec![0; count];

    // Create copy of indices to avoid modifying original array.
    let mut remaining_indices = indices.to_vec();

    let mut remaining_count = remaining_indices.len();

    let seed_bytes = u256_to_vec(&seed);
    for (i, item) in chosen_indices.iter_mut().enumerate().take(count) {
        let i_bytes = u256_to_vec(&U256::from(i));

        let index = (U256::from_big_endian(&keccak256([&seed_bytes[..], &i_bytes[..]].concat()))
            % remaining_count)
            .as_usize();
        *item = remaining_indices[index];
        remaining_indices[index] = remaining_indices[remaining_count - 1];
        remaining_count -= 1;
    }
    chosen_indices
}

pub fn address_to_string(address: Address) -> String {
    format!("{:?}", address)
}

pub fn u256_to_vec(x: &U256) -> Vec<u8> {
    let mut x_bytes = vec![0u8; 32];
    x.to_big_endian(&mut x_bytes);
    x_bytes
}

/// The minimum allowed threshold is 51%
pub fn minimum_threshold(n: usize) -> usize {
    (((n as f64) / 2.0) + 1.0) as usize
}

/// The default threshold is 66%
#[allow(dead_code)]
pub(crate) fn default_threshold(n: usize) -> usize {
    (((n as f64) * 2.0 / 3.0) + 1.0) as usize
}

#[cfg(test)]
pub mod tests {
    use super::choose_randomly_from_indices;

    #[test]
    fn test() {
        let seed = 0x1111_1111_1111_1111_u64.into();
        let chosen_indices = choose_randomly_from_indices(seed, &vec![0, 1, 2], 3);
        assert_eq!(chosen_indices.len(), 3);
        assert!(chosen_indices.contains(&0));
        assert!(chosen_indices.contains(&1));
        assert!(chosen_indices.contains(&2));
    }
}
