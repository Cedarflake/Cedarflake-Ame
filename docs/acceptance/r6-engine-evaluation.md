# R6 similarity engine evaluation

Date: 2026-11-04. Scope: R6-01 candidate evaluation on local-primary, based on `6958dea6`.
The [roadmap](../roadmap.md#current-execution-queue) owns activation and engine-admission status.

## Integration candidates

Upstream's latest release still resolves to
[Czkawka 12.0.2](https://github.com/qarmin/czkawka/releases/tag/12.0.2).
Its MIT core's [similar-image implementation](https://github.com/qarmin/czkawka/blob/12.0.2/czkawka_core/src/tools/similar_images/core.rs)
owns directory traversal, path-based decoding, cache loading and parallel execution. These entrypoints
do not accept Ame's held source lease. The complete core also brings unrelated media dependencies.
Neither its pipeline nor the GPL Krokiet interface is copied or admitted.

The same release's lockfile selects `image_hasher` 3.1.1 and `bk-tree` 0.5.0. The former accepts an
already decoded image; the latter can index Ame-owned hash bytes using a supplied distance metric.
The checked-in probe pins both as development dependencies, with default features disabled. It uses
the existing `image` 0.25.10 dependency and introduces no runtime decoder or production engine.

| Candidate | License | Evaluation boundary |
| --- | --- | --- |
| [image_hasher 3.1.1](https://docs.rs/crate/image_hasher/3.1.1/source/Cargo.toml) | MIT OR Apache-2.0 | Gradient hashes at 8 and 16 pixels per edge; Mean with DCT at 16 |
| [bk-tree 0.5.0](https://github.com/eugene-bulkin/rust-bk-tree) | MIT | Hamming-distance candidate retrieval; no filename or cache ownership |
| Ame partitioned lookup | Ordinary indexed lookup | 25 disjoint bands over 256 bits; full distance confirms candidates |

## Reproduction and fixture scope

[`similarity_engine_probe`](../../rust/examples/similarity_engine_probe.rs) generates 24 deterministic,
non-uniform 320-by-240 scenes with different geometry, gradients and colors. It checks JPEG quality 65,
half-size resampling, quarter-turn rotation, horizontal mirror, centered crops retaining 95% and 80%
of each dimension, and a localized 80-by-25 watermark. The 276 different-scene pairs are negatives.
Each comparison checks all eight rotation/reflection variants against the reference orientation.

The index probe compares identical 10,000/70,000-entry hash populations and 100 queries. Random hash
populations test index growth rather than photographic quality. The candidate-completeness case
compares band retrieval against exhaustive distances over a small fixture with known bit changes
at every radius from 0 through 24.

Run the example with `cargo run --locked --manifest-path rust/Cargo.toml --release
--example similarity_engine_probe -j 1`, and its focused case with `cargo test --locked
--manifest-path rust/Cargo.toml --release --example similarity_engine_probe -j 1 -- --test-threads=1`,
serially under the repository tooling lock. Neither command scans directories or reads original media.

## Fixture observations

The retained Release probe produced these Hamming-distance ranges for the 256-bit Gradient hash.
It also reports accepted counts at radii 8/16/24. Its candidate-completeness case passed for all
25 admitted radii; this is the probe's one focused regression case.

| Transformation | Cases | Minimum / median / maximum distance |
| --- | --- | --- |
| JPEG quality 65 | 24 | 0 / 2 / 10 |
| Half-size resampling | 24 | 0 / 2 / 6 |
| Quarter-turn rotation | 24 | 0 / 0 / 0 |
| Horizontal mirror | 24 | 0 / 0 / 0 |
| Localized watermark | 24 | 3 / 8 / 14 |
| 95% centered crop | 24 | 23 / 36 / 50 |
| 80% centered crop | 24 | 92 / 106 / 114 |
| Different scenes | 276 | 94 / 113 / 128 |

Radius 16 admitted all 120 recompression/resize/rotation/mirror/watermark cases and none of the
different-scene pairs. Crop recall is limited. These constructed images establish the comparison's
behavior, not a real-library accuracy percentage or a final numerical threshold.

For 70,000 entries and 100 identical radius-24 queries, BK-tree performed 6,114,188 full distances
in 310.597 ms. The partitioned candidate lookup performed 202,861 full distances in 13.267 ms after
206,081 posting visits. Its allocated posting capacity was 22,321,664 bytes, plus 2,240,000 hash bytes;
these are measured containers, not total process memory. Both runs returned zero random matches.

The partition bound follows directly from disjoint bands: at most 24 changed bits can affect at
most 24 of the 25 bands. More generally, a radius-r candidate retains at least 25-r equal bands.
Every retained candidate still needs its full distance. This supports evaluating a persistent band
index instead of retaining a library-sized BK-tree in memory.

## Remaining admission evidence

R6-01 still needs bounded decoding and cancellation through the admitted-source boundary, a fixed
normalization/parameter identity, and the final engine ADR. R6-02 will connect persistent evidence,
bounded candidate windows and stale-source publication rejection. Dense or low-information hashes,
source changes and real-image false positives require explicit handling; radius alone is insufficient.
The desktop, real-library and constrained-client checks remain in R6-04. This probe changes no product
interaction, catalog schema, model, source file or analysis scheduling.
