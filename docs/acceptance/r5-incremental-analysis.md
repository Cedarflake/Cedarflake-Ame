# R5 incremental analysis evidence

The [roadmap](../roadmap.md#r5-implementation-sequence) owns stage status and order.
[ADR 0036](../architecture/0036-durable-classification-dispatch.md) owns durable dispatch authority;
the [runtime record](r5-runtime-admission.md#demand-loaded-encoder-residency) retains native inference
and encoder-lifetime measurements.

## Durable work records — 2026-10-30

Schema 66 adds source-verification and exact-content tasks, dispatch intent, stopped-run identities,
and process-session leases. Source admission reads at most 64 committed catalog entries per backlog
page or 256 explicitly selected entries. Compatible content shares one task, including copies
verified before the complete duplicate scan finishes. One transaction publishes a prediction and
its completed checkpoint; manual controls and process replacement invalidate earlier claims.

Manual pause survives process restart. Abandoned and transient work consumes a three-attempt budget;
unavailable sources wait without consuming that budget. Explicit retry resets failed/waiting work.
Requested, new-content and backlog lanes receive weighted turns, while only one task holds a claim.
Stop retains the run identity; a new explicit run may reuse compatible unfinished work and evidence.
Execution records remain outside portable human-decision archives. No source file reads, media
mutation, hydration or downloads are added by these repository operations.

Sixteen new regression cases cover duplicate sharing, per-source exact publication, manual pause,
restart and abandoned work, stopped/disabled runs, surviving copies after removal, failure budgets,
fair admission, bounded backlog, atomic result/checkpoint rollback, and migration preservation and
rollback. The historical version-33 snapshot now checks the two empty work tables and exact initial
dispatch singleton before comparing all retained older rows.

Local library compilation with `classification-worker` passed in 20.37 seconds. Production Clippy
with that feature and warnings denied passed in 27.97 seconds. A debug-info override caused a
dependency compilation to exhaust memory; reusing the canonical default-profile cache completed
the library check. The full root test compile also exhausted compiler memory and supplies no test
execution result. Hosted exact-commit checks own the new runtime and migration regressions; no
workload or assertion was removed. Formatting and whitespace checks passed.

This storage implementation is not yet connected to the desktop dispatcher. Foreground admission,
model setup and shared progress remain part of the same R5-04 workflow. The guarded verification
and feature-storage connections are recorded below. No background classification is enabled by
these internal operations.

| Owner | Production lines | Inline test lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| Work domain / port | 77 / 71 | 0 | Repository scenarios below |
| Repository facade | 145 | 0 | 469 admission/publication + 252 recovery |
| Dispatch / admission / leases / completion | 248 / 241 / 245 / 144 | 0 | Shared repository scenarios |
| Migration / schema | 108 / 59 | 14 migration fixture helper | 116 migration |

## Guarded per-source verification

The classification workflow now selects its leased catalog source without requiring a same-size
duplicate peer. It uses the existing root-relative no-recall reader and 256-MiB encoded-source bound.
The held file and namespace lease spans fingerprint revalidation and the atomic fingerprint,
gallery/organization membership and task-checkpoint transaction. Existing compatible fingerprints
retain their original provenance and can complete verification without another media read.

The ordinary exact scanner keeps its peer pruning, run authority, bounded batches and publication
order. Shared membership publication now accepts the already-validated bounded generation list;
classification first validates its current task/session/epoch and exact source identity. Its native
entrypoint reuses the validated catalog-session registry and background write lane. Classification
verification does not emit or finish the complete duplicate scan's progress.

Six additional database scenarios exercise singleton admission, zero-read evidence reuse, source
lease retention through commit, rollback of fingerprint/membership/run/checkpoint, pause during
validation, hash cancellation and changed-source rejection. Existing R3 batch tests retain their
run-isolation, terminal-authority, bounded-publication and revision/rollback checks.

Production Clippy with `classification-worker` and warnings denied passed in 18.92 seconds outside
the restricted executor after the native entrypoint was corrected to use the catalog-session owner.
The restricted attempt exhausted compiler memory. A targeted database-test build then failed to map
Rust test metadata because Windows reported its pagefile limit (OS error 1455), and was stopped
without running tests. This is not evidence of an executed regression pass. Exact-commit hosted
checks retain those scenarios. Formatting and `git diff --check` passed; no originals were opened.

The hosted `dbdf2365` run found one compile-time mismatch in an existing stale-source test: it still
treated the shared publication result as a boolean. The assertion now requires `Superseded` for each
changed generation, revision and path, retaining the same rejection cases with the typed outcome.
The failed run did not execute the database regressions; their results require the corrected head.

| Owner | Production lines | Inline test lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| Source verification application / port | 86 / 28 | 0 | 285 database/application integration |
| Source verification transaction | 132 | 0 | Shared verification scenarios |
| Exact fingerprint publication | 122 | 0 | Existing source-identity and publication suites |
| Shared gallery membership / organization publication | 129 / 71 | 0 | Existing batch, grouping and recovery suites |

## Compatible feature reuse

The native classification facade now exposes a separate SQLite feature cache and the description
preparation use case. Independent image/text identities and exact content/description keys allow
rule-only rematching without another image inference. Text preparation retains completed entries
after cancellation, releases its text encoder on success or failure, and preserves a warm image
encoder when all descriptions are already cached. No model is loaded by opening or cleaning the cache.

The cache uses the existing pinned-directory capability and held file identity. Initialization
publishes a complete temporary database without replacing another creator's file. Raw header
identity/version checks precede SQLite write access to an existing file. Integrity checks cover
the key and payload; normalized values are restored without a second normalization. Recency and
accounting update atomically, with indexed eviction and bounded cleanup under ADR 0034's limits.

Twelve added Rust scenarios cover restart and exact-vector preservation, independent encoder
invalidation, content/description changes, recency eviction, rollback after eviction, corrupt values,
bounded cleanup, unrelated/newer database preservation, Windows namespace lifetime, rule-only
rematching, interrupted description reuse, failed-description retirement and warm-image retention.
Their execution belongs to the exact-commit hosted gate. A local library Clippy attempt identified
integer-decoding and facade-export errors, which were corrected, but compilation then exhausted
memory; it supplies no passing Clippy result. No further full-root test build was attempted.

A focused SQLite check executed the production schema with `trusted_schema=OFF`: 260 insertions
accounted for 68,640 bytes, bounded cleanup left four entries, rollback restored all 260, and the
eviction query used `feature_recency`. Formatting and whitespace checks pass. This check covers SQL
accounting and query shape, not the Rust adapter or Windows file capability. The
[`cb3f7d53` hosted job](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/41089127551/job/124178269076)
subsequently passed all twelve cache/description scenarios. That job still failed two exact module
loading assertions because the adapter roster omitted the new feature-cache module. The roster now
admits only its exact declaration and the new native-input declaration, retaining alternate-path,
generated-module and broader-visibility rejection. The corrected complete gate remains pending.

| Owner | Production lines | Inline test lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| Feature-cache adapter / storage / schema | 272 / 179 / 26 | 0 | 322 |
| Description preparation / repository port | 76 / 41 | 0 | 307 |

## Guarded content execution

The feature-gated native entrypoint connects a leased content task to source admission, compatible
feature lookup, image preparation, encoder residency, bounded matching and transactional prediction
publication. It checks the stored run before source access, holds the file/namespace capability
through commit, and rechecks source/task authority after inference. A cache hit skips decode and
encoding. A failed catalog commit can retain the completed compatible feature without completing the
task or changing human decisions. Cancellation during prediction insertion rolls back the checkpoint.

Six database/application scenarios cover committed publication and lost acknowledgements, cache-only
matching with a held source, incompatible pipeline rejection before I/O, cancellation at decode and
inference, concurrent pause/source replacement, failed source validation, checkpoint rollback/reuse
and cancellation during the final transaction. Two real Windows source-capability cases cover Chinese
paths, wrong extensions, read-only file/namespace retention and exact-hash fallback when the revision
is uncertain. These cases await the new head's hosted execution; no local database pass is claimed.

Five decoder tests passed in a focused single-process probe in 0.13 seconds, with no compiler warnings
in the corrected probe. Its decoder and test files match the production files byte-for-byte. It covers
seven admitted formats, first-frame GIF behavior, RGB channel/alpha handling, EXIF rotation, invalid or
oversized inputs, and cancellation before/during I/O. It does not stand in for the Windows capability,
catalog transaction or actual model path. The initial restricted compile hit OS error 1455; a separate
serial probe used the repository lockfile offline after correcting its module/dependency setup.

Six synthetic PNG inputs compare native preprocessing with Pillow 12.3.0 bilinear resizing. The
224-by-224 input matches exactly; the other five differ by at most one 8-bit channel level, with mean
absolute differences from 0.07446 to 0.19791 levels. The image-pipeline revision now binds the native
RGB8/Triangle implementation, leaving text compatibility unchanged. This establishes a numerical
difference, not classification equivalence. Native-input comparison against the retained calibration
cohort remains required before activating its calibrated profile. These probes read no original media,
download no dependencies or models, and modify no source media.

| Owner | Production lines | Inline test lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| Content execution / analysis port | 163 / 48 | 0 | Database/application scenarios below |
| Native source capability / pixel preparation | 100 / 96 | 0 | 126 / 147 |
| Analysis publication transaction | 84 | 0 | 547 database/application integration |
| Existing local-file owner | 2,629 unchanged | 3,452; twelve exact-roster lines added | Existing availability suites plus adapter loading rejection |

The integrated dispatcher, foreground resource admission, native-input calibration comparison and
desktop enablement remain in R5-04. The entrypoint alone does not enable automatic analysis.
