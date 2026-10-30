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

## Application boundary

The application exposes version-bound profiles, cached-feature first-pass/refinement evidence and
uncalibrated/low-separation/routine-sampling recommendations. There is no built-in numerical profile,
background enablement, human confirmation, catalog write or R4 organization change in this boundary.
The domain profile owner is 91 lines, the application screening owner is 131 lines, and its dedicated
tests are 347 lines; neither production owner embeds test bodies. The existing matching owner is
135 lines after adding its narrow internal taxonomy/model accessors.

All 32 focused domain/matching/screening tests pass, including 14 new screening regressions, with
zero failed or ignored. Cases cover retained suggestions without calibration, one cached-feature
refinement, separated first-pass retention, ties, threshold equality, single-category schemes,
semantic/model changes, category retirement, obsolete features, display renames and profile identity.
The local monolithic `--all-features --lib classification` build exhausted compiler memory before
executing tests. A diagnostic harness uses hash-checked hard links to the same nine production/test
source files and the same direct Blake3/thiserror versions, preserving their normal module paths.
Its 32-test pass establishes these focused boundaries; complete repository integration remains owned
by the exact-commit hosted gate. No production test or hosted requirement was removed.

The preceding `f8078187` hosted run passed every applicable job except Static. Clippy rejected an
unnecessary enclosing `Ok(...?)` in native session initialization; the expression now returns the
existing fallible session construction directly. This changes no inference configuration or error
mapping. Hosted verification of the combined screening change and lint correction follows its push.
