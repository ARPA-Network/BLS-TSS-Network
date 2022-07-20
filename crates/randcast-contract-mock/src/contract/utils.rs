use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

pub fn choose_randomly_from_indices(
    seed: usize,
    indices: &[usize],
    mut count: usize,
) -> Vec<usize> {
    let mut vec = indices.to_vec();

    let mut res: Vec<usize> = Vec::new();

    let mut hash = seed;

    while count > 0 && !vec.is_empty() {
        hash = calculate_hash(&hash) as usize;

        let index = map_to_qualified_indices(hash % (vec.len() + 1), &vec);

        res.push(index);

        vec.retain(|&x| x != index);

        count -= 1;
    }

    res
}

pub fn map_to_qualified_indices(mut index: usize, qualified_indices: &[usize]) -> usize {
    let max = qualified_indices.iter().max().unwrap();

    while !qualified_indices.contains(&index) {
        index = (index + 1) % (max + 1);
    }

    index
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
