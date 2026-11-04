use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Instant;

use bk_tree::{BKTree, Metric};

use super::fixtures::next_value;
use super::quality::distance;

type Hash = [u8; 32];

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
    let first = index * 10;
    let end = if index == 24 { 256 } else { first + 10 };
    (first..end).fold(0, |value, bit| {
        value | (u16::from((hash[bit / 8] >> (bit % 8)) & 1) << (bit - first))
    })
}

struct BandIndex {
    postings: HashMap<(usize, u16), Vec<usize>>,
    counts: Vec<u8>,
    touched: Vec<usize>,
}

impl BandIndex {
    fn new(hashes: &[Hash]) -> Self {
        let mut postings: HashMap<_, Vec<_>> = HashMap::new();
        for (id, hash) in hashes.iter().enumerate() {
            for slot in 0..25 {
                postings
                    .entry((slot, band(hash, slot)))
                    .or_default()
                    .push(id);
            }
        }
        Self {
            postings,
            counts: vec![0; hashes.len()],
            touched: Vec::new(),
        }
    }

    fn candidates(&mut self, query: &Hash, tolerance: u8) -> (Vec<usize>, usize) {
        assert!(tolerance <= 24);
        let mut examined = 0;
        for slot in 0..25 {
            if let Some(ids) = self.postings.get(&(slot, band(query, slot))) {
                examined += ids.len();
                for &id in ids {
                    if self.counts[id] == 0 {
                        self.touched.push(id);
                    }
                    self.counts[id] += 1;
                }
            }
        }
        let mut candidates = Vec::new();
        for id in self.touched.drain(..) {
            if self.counts[id] >= 25 - tolerance {
                candidates.push(id);
            }
            self.counts[id] = 0;
        }
        (candidates, examined)
    }
}

pub(crate) fn compare_indexes() {
    for count in [10_000, 70_000] {
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
        for tolerance in [8_u8, 16, 24] {
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
        for changed_bits in 0..=24 {
            let mut changed = original;
            for bit in 0..changed_bits {
                let position = (bit * 11) % 256;
                changed[position / 8] ^= 1 << (position % 8);
            }
            hashes.push(changed);
        }
    }
    let mut index = BandIndex::new(&hashes);
    for query in hashes.iter().step_by(25) {
        for radius in 0..=24 {
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
