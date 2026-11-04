# R6 similarity engine evaluation

Date: 2026-11-04. Scope: R6-01 candidate evaluation on local-primary, based on `6958dea6`.
The [roadmap](../roadmap.md#current-execution-queue) owns activation and engine-admission status.
The selected boundary and immutable parameters are owned by
[ADR 0038](../architecture/0038-bounded-perceptual-similarity.md).

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
| Ame partitioned lookup | Ordinary indexed lookup | Four disjoint 16-bit bands; nearby band keys nominate full-distance candidates |

## Reproduction and fixture scope

[`similarity_engine_probe`](../../rust/examples/similarity_engine_probe.rs) generates 24 deterministic,
non-uniform 320-by-240 scenes with different geometry, gradients and colors. It checks JPEG quality 65,
half-size resampling, quarter-turn rotation, horizontal mirror, centered crops retaining 95% and 80%
of each dimension, and a localized 80-by-25 watermark. The 276 different-scene pairs are negatives.
The final comparison takes the minimum across both images' eight rotation/reflection variants.
It uses the alpha-on-white, grayscale, 256-edge sample defined in ADR 0038.

The index probe compares identical 10,000/70,000/560,000-entry hash populations and 100 variant
queries. The last size represents the maximum hash count for 70,000 contents with eight variants;
it is not a real-photo dataset. Comparing one content may require eight such queries. Random hash
populations test index growth rather than photographic quality. The candidate-completeness case
compares retrieval against exhaustive distances with known bit changes at every radius from 0 to 10.

Run the example with `cargo run --locked --manifest-path rust/Cargo.toml --release
--example similarity_engine_probe -j 1`, and its focused case with `cargo test --locked
--manifest-path rust/Cargo.toml --release --example similarity_engine_probe -j 1 -- --test-threads=1`,
serially under the repository tooling lock. Neither command scans directories or reads original media.

## Initial 64-bit scheme observations

The final Release probe produced these results for 64-bit Gradient at strict / standard / broad
maximum distances 2 / 6 / 10. The four focused cases passed: candidate completeness, borrowed-input
cancellation, malformed/oversized input, and bounded normalization with flat-input rejection.

| Transformation | Cases | Minimum / median / maximum | Accepted strict / standard / broad |
| --- | --- | --- | --- |
| JPEG quality 65 | 24 | 0 / 0 / 2 | 24 / 24 / 24 |
| Half-size resampling | 24 | 0 / 0 / 0 | 24 / 24 / 24 |
| Quarter-turn rotation | 24 | 0 / 0 / 0 | 24 / 24 / 24 |
| Horizontal mirror | 24 | 0 / 0 / 0 | 24 / 24 / 24 |
| Localized watermark | 24 | 0 / 1 / 4 | 19 / 24 / 24 |
| 95% centered crop | 24 | 0 / 4 / 6 | 8 / 24 / 24 |
| 80% centered crop | 24 | 9 / 15 / 22 | 0 / 0 / 2 |
| Different scenes | 276 | 13 / 24 / 29 | 0 / 0 / 0 |

Standard admitted all 144 recompression/resize/rotation/mirror/watermark/light-crop cases and none
of the 276 different-scene pairs. The same normalized fixture with 256-bit Gradient admitted no light
crops at radius 16; Mean/DCT admitted four, and only eight watermarks. This supports the 64-bit choice.
The constructed images establish reproducible behavior, not real-library accuracy. Heavy cropping
remains weak. Flat hashes are ineligible; low-detail and color-only differences can still collide.

## Candidate cost and memory

For 100 identical broad queries, the final Release run measured:

| Hash entries | BK-tree comparisons / time | Band comparisons / time | Posting visits | Matches, both |
| --- | --- | --- | --- | --- |
| 70,000 | 4,833,764 / 78.200 ms | 80,567 / 5.510 ms | 81,044 | 1 |
| 560,000 | 35,919,181 / 1,686.503 ms | 645,576 / 19.379 ms | 649,249 | 2 |

The 560,000-entry index held 25,774,048 bytes of posting capacity and 4,480,000 bytes of hashes;
map, candidate and allocator costs are additional. A Windows child-process observation across the
same index and large-image workloads recorded a 156,659,712-byte peak working set. This is the
standalone probe's peak, not a production application ceiling. Dense postings still need bounded
persistent paging. ADR 0038 gives the completeness argument and limits indexed keys to 548 per variant.

## Borrowed input boundary

The probe's decoder accepts only a borrowed `Read + Seek` source. It rewinds that reader, identifies
the format from encoded bytes and applies EXIF orientation. Its limits match the existing background
image boundary: 256 MiB encoded input, 16,384 per dimension, 16 Mi pixels and 128 MiB decoder allocation.
Cancellation is checked before source access, on reads/seeks and between decode phases.

The borrowed source remains intact; cancellation during I/O or before entry returns no pixels.
Malformed/empty/oversized input is rejected. Transparent colors are composited before grayscale;
quarter-turns preserve the variant set, and flat samples supply no useful match structure.

The final 4096-by-4096 RGBA fixture encoded to 19,692,145 PNG bytes. Decode took 123.897 ms,
normalization 72.116 ms, and eight bounded hashes 3.113 ms. The final grayscale sample occupied
65,536 bytes. Cancellation requested after 1 and 50 ms retired normalization after another 1.710
and 19.120 ms respectively. These timings include ordinary host variation and do not promise a
deadline inside every codec. Production Windows leases and stale-publication checks need integration.

## Real-library quality review - 2026-11-05

The production adapter at `58ac3f28` analyzed an isolated copy of the retained local-primary catalog
(63,077 locations). The run checked 2,004 sources and published 1,997 content records in 183.145 s;
its sampled peak RSS was 313 MiB. Cancellation requested at the 2,000-source polling threshold
completed after 34 ms. Six sources were skipped: four revision changes and two source-guard failures.
This was a bounded partial run, not full-library coverage. The original catalog digest and 17 sampled
source digests remained unchanged; the derived catalog passed integrity and foreign-key checks.

Strict / standard / broad returned 1 / 3 / 6 visible groups, with queries taking 149-153 ms.
A deterministic sample of up to three reference/member pairs per observed distance produced 16
pairs: visual review found two same-image resize variants, two related but different expressions,
and twelve clearly unrelated pairs. Both sampled distance-5 pairs were unrelated, including an
illustrated wallpaper paired with a system-monitor screenshot. Standard therefore admits obvious
false positives; the synthetic fixture did not establish adequate production quality. These selected
pairs are counterexamples, not an estimate of whole-library precision or recall.

Algorithm/parameter selection must address these counterexamples while retaining transformation
coverage before R6 acceptance. The comparison below selects a replacement; production and broader
real-library verification remain separate. Private source references and hashes remain only in ignored local diagnostic storage;
no source image or catalog is committed.

## Detail-hash comparison - 2026-11-05

A bounded offline diagnostic reused production normalization and the pinned `image_hasher` 3.1.1
and `image` 0.25.10. It re-read only the 22 distinct local-primary sources from the reviewed pairs,
held no-write/no-delete file and ancestor guards, rejected placeholders/reparse traversal, and
verified each SHA-256 before decoding. The 16 recomputed Gradient-64 distances exactly matched the
prior native run, locating the counterexamples in hash discrimination rather than stored-distance
mapping. All transformed images remained in memory; only metrics were written.

| Candidate | Twelve unrelated pairs | Same-image resize pairs | Real watermark variants |
| --- | --- | --- | --- |
| Gradient 64, standard radius 6 | Two admitted | 2/2 | 22/22 |
| Median/DCT 64, radius 8 | None admitted | 2/2 | 14/22 |
| Gradient 256, radius 44 | None admitted | 2/2 | 22/22 |
| Gradient 64 with paired 256-bit confirmation at 44 | None admitted | 2/2 | 22/22 |

Mean/DCT retained obvious false matches; Gradient/DCT and Median/DCT lost more real watermarks.
The 256-bit Gradient distances for the known unrelated pairs ranged from 50 to 101; same-image
resize pairs remained zero. Radius 44 covers this fixture's light-crop maximum. It is an empirical
choice on a small selected sample, not an independently established whole-library accuracy bound.

The selected paired comparison requires both distances from the same pair of transforms. Standard
retained all 132 real-source JPEG-65, half-size, quarter-turn, mirror, watermark and 95%-crop
variants. Of 144 analogous synthetic variants, standard retained 143 and broad retained 144;
one light crop needed broad after requiring consistent orientation. None of 276 different-scene
pairs matched. Only 2/22 real and 0/24 synthetic 80%-crop variants matched at broad. Distinct related
expressions were rejected. Heavy crops and semantic similarity remain outside reliable coverage.

The final diagnostic completed in 23.907 s with a 197,640,192-byte peak working set. It included all
seven single-hash comparisons and the paired scheme, not just production hash cost. The paired
scheme keeps 548 candidate keys per variant; a direct 16-band detail index would need 2,192.
The production integration follows ADR 0038. Ten existing focused Rust cases passed after extending
the matching fixture with a coarse collision whose detail match belongs to an incompatible
orientation. The migration fixture covers atomic 74-to-75 rollback/retry, unchanged legacy hashes
and choices, retired legacy continuations/leases, and rejection of incompatible confirmation rows.
Dense posting continuation, restart, fixed-reference membership, source/cancellation admission and
review decisions retained their existing coverage. No new test case or suite was added.

The first upgrade check rejected SQLite's quoted table name after rename; the migration's canonical
DDL now matches that exact shape. The final ten-case run passed in 21.59 s without build warnings.
Source leases and normalization are unchanged. The native verification below covers the connected
adapter, persistence and query path.

Hash construction has a 43-line adapter owner; the reader facade has 151 lines and 100 dedicated test
lines. Confirmation migration/validation has 121 production/shared lines and 40 inline test-support
lines; the extended migration cases have 225 dedicated lines. Matching, posting and cursor owners
have 151, 93 and 114 production lines, with 306 dedicated analysis-test and 359 review-test lines.
The existing catalog/migration facades receive only the schema version and ordered migration hooks:
2,991/177 and 4,124/2,349 production/shared versus inline test/support lines respectively.
The catalog facade's unchanged dedicated test tree contains 11,606 lines; the migration coordinator's
own cases remain inline. No broader facade decomposition or test expansion is included.

## Paired-engine native verification - 2026-11-05

The guarded optimized native library from `3c59c25b` analyzed a fresh isolated copy of the same
63,077-location local-primary baseline, upgrading it from schema 45 to 75. No user catalog or
source tree was modified. The run stopped at the existing 2,000-source polling threshold:

| Observation | Gradient-64 at `58ac3f28` | Paired scheme at `3c59c25b` |
| --- | --- | --- |
| Checked sources / content evidence | 2,004 / 1,997 | 2,003 / 1,996 |
| Strict / standard / broad groups | 1 / 3 / 6 | 1 / 1 / 1 |
| Analysis elapsed | 183.145 s | 148.656 s |
| Sampled peak working set | 313 MiB | 406 MiB |
| Cancellation retirement | 34 ms | 35 ms |

These are single partial-run observations, not a general throughput or memory bound. The new group
contains three distinct contents. Both reference/member pairs have distance zero and exactly match
the SHA-256 pairs already visually confirmed as same-image resize variants. This covers every
current reference/member relationship; none of the twelve known unrelated pairs or the related
but different expressions remains. Group queries took 152.5-155.6 ms.

The six skips remained four changed revisions and two source-guard failures. Original baseline
SHA-256 and all 17 retained source-sample hashes were unchanged. The derived catalog passed SQLite
quick-check and foreign-key checks, the child exited with code 0, and scratch retirement completed.
The prior small reports and private pair manifest are retained in ignored diagnostic storage.

A connected fixture check used the same guarded library to scan and analyze two existing synthetic
scenes and their JPEG-65, half-size, quarter-turn, mirror, watermark, 5%-crop and 20%-crop variants.
All 16 sources produced evidence in 806 ms; their persisted paired signatures matched the evaluation
probe. Standard and broad retained the twelve admitted variants in two separate scene groups;
both heavy crops remained outside those groups. Strict omitted one light crop. Source SHA-256
values were unchanged, SQLite quick-check and foreign-key checks passed, and the leased images and
catalog were removed. This closes the production-entry transformation check without a new test suite.

This resolves the observed standard-degree false-positive blocker. Full-library precision/recall,
retained-scale work growth and constrained-client measurements remain outside this partial run.
Heavy crops, layout changes and semantic resemblance retain the stated quality limits.

## Retained-library growth and sampled matches - 2026-11-05

The same guarded `3c59c25b` library subsequently checked 10,006 sources against a fresh isolated
copy of the 63,077-location baseline. It published 9,974 content records in 596.026 seconds, with
a 402-MiB sampled peak working set and 216-ms cancellation retirement. After the initial 1,000
sources, successive 1,000-source samples took 51-62 seconds; the sampled memory peak did not grow.
Strict / standard / broad returned 8 / 11 / 13 groups, taking 610 / 524 / 522 ms respectively.
These measurements cover actual persistent matching and group queries at this retained scale,
alongside the unchanged coarse-index growth and bounded-posting evidence above.

The derived catalog retained 13 groups and 66 memberships, including reference members. Sampling
up to three reference/member pairs per observed distance produced 28 pairs. Guarded reads verified
each source SHA-256 before bounded decoding; temporary review sheets were removed after inspection.
Eleven pairs shared meme artwork with different inset images, fifteen shared screenshot layouts
with different text, and two broad-only pairs depicted the same character with different props or
poses. No wholly unrelated scene pair was observed in this selected sample. This is visual-layout
similarity, including changed text and small inserts, not evidence of identical image content or a
whole-library precision estimate. The earlier same-image resize checks remain separate evidence.

There were 26 revision-change skips and two source-guard failures. The retained baseline SHA-256
and all 17 source-sample hashes were unchanged. SQLite quick-check passed with zero foreign-key
issues; the child exited with code 0 and the scratch catalog was retired. Constrained-client
measurement remains pending; the existing 2-core, 8-GiB host and retained catalog were read-only
preflighted without changing its checkout, installed SDK or source media.

## Provenance and remaining integration

The shared `adapters/analysis_source.rs` boundary was extracted from production classification input
after `14e9ee00`. Classification consumes the same held file and namespace guard through this owner;
its error codes, uncertain-revision fingerprint check, pixel normalization and publication revalidation
are preserved. The source owner has 101 production lines; the classification facade has 71 and its
pixel owner 103. The two retained dedicated test files have 327 lines; no cases were added.

Seven existing decoder cases passed in the first Release run. The two Windows source cases first
failed at default temporary-directory admission: the sandbox token failed namespace ACL protection,
and ordinary execution could not create the default temporary fixture. The unchanged binary and two
cases then passed with `TEMP`/`TMP` redirected through `Invoke-AmeAcceptanceScratch`. They prove held
source/ancestor protection, byte preservation after release, uncertain-revision content matching and
wrong-root rejection. The scratch scope retired successfully. These are shared-reader checks, not
production similarity scheduling or persistent-result acceptance.

The initial [256-bit probe](https://github.com/Cedarflake/Cedarflake-Ame/commit/926f71d67ac08493cacf87abe4c3f72997f9bbfc)
showed BK-tree's broad-search cost and weak crop recall. The
[borrowed-input extension](https://github.com/Cedarflake/Cedarflake-Ame/commit/4883cf4dd2c3e25bbabe8b3d9b828ed66f85c42b)
established the path-free input seam. The current probe replaces those provisional parameters with
the admitted normalization, symmetric comparison and 64-bit index. An unused-import warning in the
first normalization build was removed; the final build and execution were warning-free.

The [integration evidence](r6-similar-images.md) owns production source admission, persistence,
bounded matching, durable choices and desktop interaction. The paired-engine verification above
supersedes the initial 64-bit quality choice; broader-library and constrained-client measurements
remain in R6-04. Hosted CI covers each actual source commit separately.
