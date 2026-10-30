# R5 purpose-aware screening calibration

Status: scoped calibration evidence; stage disposition belongs to the roadmap.

The [requirements](../product/r5-classification.md) and
[execution plan](../plans/r5-classification.md#r5-02-taxonomy-model-and-runtime-admission) own the
purpose-aware rubric and separated evaluation. [ADR 0034](../architecture/0034-versioned-classification-contracts.md)
owns feature compatibility and the one-refinement application boundary.

## Frozen comparison

The initial candidate retains SigLIP 2 Base/16-224 FP16 image and INT8 text, the admitted native CPU
runtime, one thread and one image/description per batch. Existing image features and the 24 original
description features are reused for calibration. The refinement has four more specific descriptions
per category, including conversational reactions made from photographs, narrative comic context,
useful interface content and documentary photographs. It uses maximum description cosine and never
re-encodes a source image.

The original 240-image comparison is calibration data, including its prior model-selection context.
Collapsing four known visual clusters leaves 236 groups: 188 definite provisional labels and 48
ambiguous labels. The labels retain their original authoring authority. Historical folders are not
ground truth, and these visual judgments are not user-confirmed accuracy measurements.

A fresh local-primary sample contains 128 images, with labels sealed before encoding or matching.
Known exact content, recognizable post/page series and close 256-bit difference-hash matches against
calibration are excluded. Five additionally identified within-holdout visual clusters are represented
once, leaving 120 groups: 103 definite provisional labels and 17 ambiguous groups. Unknown visual
relationships can remain; this is a bounded initial-screening evaluation, not a guarantee that every
semantic relationship in the library is separated. Ambiguous groups remain in coverage and review
counts, outside the definite-label agreement denominator.

The rubric/descriptions hash is
`7941035af88af56ea62485f128d84d1c186655f0166988189294871e56d09338`.
The sealed holdout-label hash is
`8f7675e7aa605056e59647b54d3b8e942d2f01f27685c9744b00b7cbd24da3cf`.
Private paths, labels, contact sheets, tensors and per-item predictions remain in ignored local
storage. No sample media or model weights are committed.

## Candidate operating policy

Calibration chooses a first-pass separation cutoff that maximizes definite-label agreement, breaking
ties toward fewer refinements. Optional-review selection maximizes error-retrieval F0.5 on calibration,
favoring concentrated checks rather than a large review queue. This statistical objective does not
assign a measured human-time cost to an inspection or mistake. Both cutoffs are frozen before the
single held-out comparison:

- one refinement when initial separation is below `0.0024277272168546915`;
- optional checking when final separation is below `0.006240375805646181`.

The frozen operating-policy hash is
`8267d04a03c7d151a02cce4041e131bdb5a75c31b31b5d8df85deee4673d46eb`.

| Observation | Calibration, first pass | Calibration, one refinement | Holdout, first pass | Holdout, one refinement |
| --- | ---: | ---: | ---: | ---: |
| Definite-label agreement | 160/188 | 164/188 | 82/103 | 82/103 |
| Groups re-screened | 0 | 18/236 | 0 | 7/120 |
| Optional checks per 1,000 groups | 173.73 | 135.59 | 125.00 | 100.00 |
| Definite errors inside review | 18 | 13 | 6 | 5 |
| Definite errors outside review | 10 | 11 | 15 | 16 |

Applying the refined descriptions to every group performs worse: 149/188 calibration and 75/103
holdout agreement. The bounded policy preserves more useful first-pass results. It does not improve
holdout agreement, and its smaller review set leaves one additional definite error outside review.
These cutoffs are **not admitted as automatic defaults or a high-confidence guarantee**.

The held-out final result has the following category evidence. Precision uses definite predictions;
recall uses definite reference labels. Neither is a human-certified rate.

| Category | Correct / predicted | Correct / reference |
| --- | ---: | ---: |
| Illustration | 17/28 | 17/17 |
| Comic | 1/5 | 1/2 |
| Meme/reaction | 23/25 | 23/32 |
| Screenshot | 23/27 | 23/23 |
| Photograph | 14/14 | 14/16 |
| Other defined content | 4/4 | 4/13 |

Comic coverage is small, and documents/design content is often proposed as illustration or screenshot.
A wider separation alone does not resolve those purpose boundaries. A replacement operating policy
needs calibration-only fitting and a new untouched holdout; the evaluated holdout cannot be recycled
as an independent pass. Actual inspection/correction time and total human work remain measurements of
the connected R5-07 workflow. The earlier lightweight-model comparison remains retained.

## Source and execution evidence

Preparation uses only the configured local-primary root. Metadata traversal opens no media content;
selected reads reject cloud/reparse attributes, pin directory namespaces, hold read-only data handles
against writes/deletion and compare file identity before and after reading. Each source is bounded
to 64 MiB, decoded images to 40 million pixels and a preparation run to 1 GiB of source bytes.
Only first-frame, oriented RGB pixels and bounded full-frame review thumbnails are retained.

The first preparation exposed empty Windows directory-entry file identities and was rejected before
prediction comparison. Direct no-follow file metadata repaired the sampler; a guard rejects a zero
identity. The valid run inspected 68,688 metadata entries and 49,493 eligible groups, read 131 files
and 237,434,612 bytes, excluded two oversized decoded images and one possible visual overlap, and
retained 128 samples. Neither run writes source media or requests placeholder hydration. The rejected
sample is excluded from calibration and evaluation.

The supervised native executable admitted in the [runtime record](r5-runtime-admission.md) encodes
24 refinement descriptions and 128 prepared tensors. No original source is opened during inference;
both workers exit and are joined. Worker digest:
`2c63587eec11339617e7f050ca4c1584243e85831e2b2a9561487774aac2e2ea`.
Protocol, model artifacts, tensor preprocessing and text-batch identity retain that record's checks.

## Category-aware operating candidate

A second policy fits only the original 236 calibration groups. Each initial leading category gets
its own refinement cutoff, chosen by definite-label agreement and then fewer refinements. Review
cutoffs use the same error-retrieval objective within each final leading category. Fewer than ten
definite predictions leave that category's review band uncalibrated. This support floor is a fitting
guard, not an accuracy guarantee. The descriptions, native runtime and cached text features are
unchanged. The first 120-group holdout is excluded from this fit.

| Category | Refine below | Optional check below |
| --- | ---: | ---: |
| Illustration | 0.0009956361027434468 | 0.010479376651346684 |
| Comic | 0 | Uncalibrated |
| Meme/reaction | 0.0009016544208861887 | 0.005887806881219149 |
| Screenshot | 0 | 0.002599731320515275 |
| Photograph | 0 | 0 |
| Other defined content | 0 | Uncalibrated |

Exact ties still receive one refinement and remain boundary cases. An uncalibrated category keeps
its proposed result and enters optional checking; no human confirmation is inferred. On calibration,
agreement is 163/188 with six refinements and 43 optional checks. A deterministic five-fold comparison
that refits within each training partition gives 161/188, three refinements and 44 checks; the
unchanged first pass gives 160/188. This is internal calibration evidence.

The frozen policy hash is
`93ac9469ca56a67e15c445c2714626c7265aec5db78751db98cad1dbaac1b64d`.
A new 192-image sample is labeled before inference. Exact content, known post series and close
visual hashes from both earlier partitions are excluded during preparation. Visual inspection
additionally excludes ten known prior-family overlaps and eleven repeated within-sample members.
The sealed comparison therefore has 171 groups: 158 definite and 13 ambiguous. Label hash:
`a2e43ca7f8047d4bd2c98ae9eefb3dea3809c0ab3167b8177ee71a6300f43d11`.
The same provisional-authority and unknown-relationship limits apply.

| New held-out observation | First pass with the previous global check cutoff | Category-aware policy |
| --- | ---: | ---: |
| Definite-label agreement | 136/158 | 136/158 |
| Groups re-screened | 0 | 0 |
| Optional checks | 18/171 | 28/171 |
| Optional checks per 1,000 groups | 105.26 | 163.74 |
| Definite errors inside review | 9 | 10 |
| Definite errors outside review | 13 | 12 |

The additional ten checks cover one additional definite error. This cohort has no cases below the
new refinement cutoffs, so it establishes no fresh refinement gain. Final category agreement is
25/33 illustration predictions, 4/5 comic, 36/44 meme, 41/45 screenshot, 22/22 photograph and 8/9 other;
the corresponding definite reference counts are 27, 4, 46, 43, 28 and 10. Comic coverage remains small.
The unchecked subset contains 12 errors among 135 definite groups. It is a sampling band, not a
validated high-confidence accuracy claim.

Retain this category-aware profile as the initial-screening integration candidate for these exact
descriptions and encoder revisions. It preserves useful suggestions, limits automatic refinement,
and makes unsupported review bands explicit. It does not establish a reduction in actual correction
time; the connected R5-07 workflow owns that comparison. Custom definitions keep suggestions usable
without borrowing an unrelated category's calibration. No further threshold tuning is performed
against either evaluated holdout.

Preparation reads 203 local-primary files and 282,921,670 bytes after metadata-only traversal. Seven
exact overlaps, three unrecognized images and one oversized decoded image are excluded. Source
writes and placeholder hydration requests remain zero. A palette-transparency conversion warning
is repaired through explicit RGBA-to-RGB conversion; a focused palette fixture confirms identical
RGB bytes with no warning. Retained tensors keep the same image-pipeline semantics.
One verified worker encodes all 192 prepared tensors and retires cleanly, without opening originals.
The production Rust matcher and category-aware screening owner reproduce all 192 frozen category,
review-band and refinement outcomes from the cached features. This parity check includes excluded
samples only for numerical correspondence; they remain excluded from the quality denominator.

## Native image-input comparison

The native RGB8/Triangle input from `7a4e257a` was compared against the retained category-aware
cohort with the same sealed labels, descriptions, numerical policy and pinned model graphs. No
threshold was fitted and no new untouched-holdout result is claimed. Source digests and Windows
identities matched all 192 fixed inputs. Native admission decoded 189; three exceeded the existing
16,777,216-pixel limit. Those three remain explicit analysis failures, reducing the 171 eligible
groups to 168 comparable groups with 155 definite references. Their absence is a coverage loss,
not a correct prediction.

| Paired observation on the same 168 groups | Retained Pillow input | Native input |
| --- | ---: | ---: |
| Definite-reference agreement | 133/155 | 133/155 |
| Optional checks | 28 | 28 |
| Automatic refinements | 0 | 1 |
| Definite errors inside optional checking | 10 | 10 |
| Definite errors outside optional checking | 12 | 12 |

Two suggestions change: one screenshot-to-meme change agrees with its provisional reference, while
one meme-to-illustration change loses agreement. Review flags are unchanged. Minimum paired feature
cosine is 0.98593109 and median cosine is 0.99993449. These observations support retaining the frozen
numerical cutoffs as an initial-screening integration candidate under the native pipeline revision;
they establish neither bit equivalence nor a new high-confidence accuracy claim. The reference-label
authority, small-category coverage and human-time limitations above continue to apply.

The largest pixel difference belongs to a partial-frame GIF. Its encoded rectangle occupies
`(63, 52)–(449, 455)` of a 512-by-512 logical canvas. Pillow retains palette-background RGB outside
that rectangle, while the native decoder initializes it to transparent zero. Giving both the same
canvas reduces their maximum channel difference to one 8-bit level; the suggestion remains unchanged.
A synthetic partial-frame GIF now covers this native behavior alongside the existing orientation,
alpha, first-frame, format, limit and cancellation fixtures. All six pixel tests pass in 0.20 seconds.

Preparation read 192 local-primary inputs and 252,462,778 bytes without source mutation or placeholder
hydration. Three bounded diagnostic rereads of the same GIF added 36,807 bytes. Only one temporary
encoded image existed at a time; it was retired before the next input. Prepared tensors and aggregate
comparison results remain outside source trees and Git. Preparation took 594.05 seconds in a Debug
helper that includes process startup and unbuffered tensor serialization; this is not a production
decode-throughput result. The existing admitted native worker encoded 189 tensors in 232.58 seconds
and exited cleanly. It receives prepared pixels and opens no original media.

The pixel-owner SHA-256 is
`5ec3f83e5d66f122d2304c98f268cc7275382243fc0c54555c75cf82c246170b`;
the numerical worker retains digest
`2c63587eec11339617e7f050ca4c1584243e85831e2b2a9561487774aac2e2ea`.
This comparison reuses the admitted model graphs through their old worker handshake; validation of
the rebuilt current caller/worker pair remains part of runtime integration. The native pipeline has
its own compatibility identity and cannot relabel prior cached image features.

## Application boundary

The screening application boundary exposes version-bound profiles, cached-feature first-pass/refinement
evidence and uncalibrated/low-separation/routine-sampling recommendations. Its original focused checks
cover no background enablement, human confirmation, catalog write or R4 organization change.
The subsequent [configuration integration](r5-incremental-analysis.md#classification-configuration-and-activation)
selects the frozen category-aware candidate only for its exact descriptions and native pipeline
revisions. That integration reuses the measurements above and does not add a new quality claim.
The domain profile owner is 184 lines and the application screening owner is 141 lines. Dedicated
tests occupy 360 lines for common screening and 145 for category profiles; neither production owner
embeds test bodies. The existing matching owner is
135 lines after adding its narrow internal taxonomy/model accessors.

All 39 focused domain/matching/screening tests pass, including 21 screening regressions, with
zero failed or ignored. Cases cover retained suggestions without calibration, one cached-feature
refinement, separated first-pass retention, ties, threshold equality, single-category schemes,
semantic/model changes, category retirement, obsolete features, display renames and profile identity.
The local monolithic `--all-features --lib classification` build exhausted compiler memory before
executing tests. A diagnostic harness uses ten hash-checked production/test source files, nine through
hard links and one exact copy, with the same direct Blake3/thiserror versions and normal module paths.
Its 39-test pass and warning-free Clippy check establish these focused boundaries; complete repository integration remains owned
by the exact-commit hosted gate. No production test or hosted requirement was removed.

The preceding `f8078187` hosted run passed every applicable job except Static. Clippy rejected an
unnecessary enclosing `Ok(...?)` in native session initialization; the expression now returns the
existing fallible session construction directly. This changes no inference configuration or error
mapping. The `1d79729f` hosted Static job passes this correction; that commit's remaining hosted jobs
and the subsequent category-aware change retain their own exact-commit gate.
