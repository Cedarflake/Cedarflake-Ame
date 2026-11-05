use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Instant;

use bk_tree::{BKTree, Metric};

use super::fixtures::next_value;
use super::quality::distance;

type Hash = [u8; 8];

struct Hamming(Rc<Cell<usize>>);

impl Metric<Hash> for Hamming {
    fn distance(&self, left: &Hash, right: &Hash) -> u32 {
        self.0.set(self.0.get() + 1);
        distance(left, right)
    }

    fn threshold_distance(&self, left: &Hash, right: &Hash, threshold: u32) -> Option<u32> {
        let distance = self.distance(left, right);
        (distance <= threshold).then_some(distance)
    }
}

fn band(hash: &Hash, index: usize) -> u16 {
    u16::from_le_bytes([hash[index * 2], hash[index * 2 + 1]])
}

fn neighbors(value: u16, radius: u8) -> Vec<u16> {
    let mut result = vec![value];
    if radius >= 1 {
        for first in 0..16 {
            result.push(value ^ (1 << first));
            if radius >= 2 {
                for second in first + 1..16 {
                    result.push(value ^ (1 << first) ^ (1 << second));
                }
            }
        }
    }
    result
}

struct BandIndex {
    postings: HashMap<(usize, u16), Vec<usize>>,
    seen: Vec<bool>,
    touched: Vec<usize>,
}

impl BandIndex {
    fn new(hashes: &[Hash]) -> Self {
        let mut postings: HashMap<_, Vec<_>> = HashMap::new();
        for (id, hash) in hashes.iter().enumerate() {
            for slot in 0..4 {
                postings
                    .entry((slot, band(hash, slot)))
                    .or_default()
                    .push(id);
            }
        }
        Self {
            postings,
            seen: vec![false; hashes.len()],
            touched: Vec::new(),
        }
    }

    fn candidates(&mut self, query: &Hash, tolerance: u8) -> (Vec<usize>, usize) {
        assert!(tolerance <= 10);
        let mut examined = 0;
        for slot in 0..4 {
            for value in neighbors(band(query, slot), tolerance / 4) {
                if let Some(ids) = self.postings.get(&(slot, value)) {
                    examined += ids.len();
                    for &id in ids {
                        if !self.seen[id] {
                            self.touched.push(id);
                            self.seen[id] = true;
                        }
                    }
                }
            }
        }
        let candidates = std::mem::take(&mut self.touched);
        for &id in &candidates {
            self.seen[id] = false;
        }
        (candidates, examined)
    }
}

pub(crate) fn compare_indexes() {
    for count in [10_000, 70_000, 560_000] {
        let mut state = 0x1847_c920_9f16_55d1;
        let hashes: Vec<Hash> = (0..count)
            .map(|_| std::array::from_fn(|_| next_value(&mut state) as u8))
            .collect();
        let queries: Vec<Hash> = (0..100)
            .map(|_| std::array::from_fn(|_| next_value(&mut state) as u8))
            .collect();
        let calls = Rc::new(Cell::new(0));
        let mut tree = BKTree::new(Hamming(Rc::clone(&calls)));
        let start = Instant::now();
        for hash in &hashes {
            tree.add(*hash);
        }
        println!(
            "bktree n={count} build_us={} calls={}",
            start.elapsed().as_micros(),
            calls.get()
        );
        let start = Instant::now();
        let mut bands = BandIndex::new(&hashes);
        let posting_capacity: usize = bands.postings.values().map(Vec::capacity).sum();
        println!(
            "bands n={count} build_us={} buckets={} posting_capacity_bytes={} hash_bytes={}",
            start.elapsed().as_micros(),
            bands.postings.len(),
            posting_capacity * size_of::<usize>(),
            count * size_of::<Hash>()
        );
        for tolerance in [2_u8, 6, 10] {
            calls.set(0);
            let start = Instant::now();
            let found: usize = queries
                .iter()
                .map(|query| tree.find(query, u32::from(tolerance)).count())
                .sum();
            println!(
                "  bktree radius={tolerance} queries=100 elapsed_us={} calls={} matches={found}",
                start.elapsed().as_micros(),
                calls.get()
            );
            let start = Instant::now();
            let mut comparisons = 0;
            let mut postings = 0;
            let mut found = 0;
            for query in &queries {
                let (candidates, examined) = bands.candidates(query, tolerance);
                comparisons += candidates.len();
                postings += examined;
                found += candidates
                    .iter()
                    .filter(|&&id| distance(query, &hashes[id]) <= u32::from(tolerance))
                    .count();
            }
            println!(
                "  bands radius={tolerance} queries=100 elapsed_us={} postings={postings} comparisons={comparisons} matches={found}",
                start.elapsed().as_micros()
            );
        }
    }
}

#[test]
fn candidate_partition_preserves_matches_at_every_admitted_radius() {
    let mut state = 0x741b_125e_d008_6391;
    let mut hashes = Vec::new();
    for _ in 0..12 {
        let original: Hash = std::array::from_fn(|_| next_value(&mut state) as u8);
        for changed_bits in 0..=10 {
            let mut changed = original;
            for bit in 0..changed_bits {
                let position = (bit * 11) % 64;
                changed[position / 8] ^= 1 << (position % 8);
            }
            hashes.push(changed);
        }
    }
    let mut index = BandIndex::new(&hashes);
    for query in hashes.iter().step_by(11) {
        for radius in 0..=10 {
            let expected: Vec<_> = hashes
                .iter()
                .enumerate()
                .filter_map(|(id, hash)| (distance(query, hash) <= u32::from(radius)).then_some(id))
                .collect();
            let (candidates, _) = index.candidates(query, radius);
            let mut actual: Vec<_> = candidates
                .into_iter()
                .filter(|&id| distance(query, &hashes[id]) <= u32::from(radius))
                .collect();
            actual.sort_unstable();
            assert_eq!(actual, expected, "radius {radius}");
        }
    }
}
