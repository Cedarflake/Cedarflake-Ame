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

This storage implementation is not yet connected to the desktop dispatcher. Feature storage,
foreground admission, model setup and shared progress remain part of the same R5-04 workflow.
The guarded verification connection is recorded below. No background classification is enabled
by these internal operations.

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
