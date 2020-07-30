use std::collections::HashSet;

use rand::RngCore;

#[inline(always)]
pub fn random_usize<R: RngCore>(rng: &mut R, m: usize) -> usize {
    // generate a 128-bit integer (to minimize statistical bias)
    let mut le_bytes: [u8; 16] = [0u8; 16];
    rng.fill_bytes(&mut le_bytes);

    // reduce mod the number of repetitions
    let n: u128 = u128::from_le_bytes(le_bytes) % (m as u128);
    n as usize
}

pub fn random_subset<R: RngCore>(rng: &mut R, m: usize, l: usize) -> Vec<usize> {
    let mut members: HashSet<usize> = HashSet::new();
    let mut samples: Vec<usize> = Vec::with_capacity(l);

    while samples.len() < l {
        // generate random usize
        let n = random_usize::<R>(rng, m);

        // if not in set, add to the vector
        if members.insert(n) {
            samples.push(n);
        }
    }

    // ensure a canonical ordering (for comparisons)
    samples.sort();
    samples
}
