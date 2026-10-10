# R2c-M replacement reliability

Status: accepted on 2026-08-22

## Scope

This gate closes the ADR 0023 replacement continuity model rather than reusing the historical
R2c-H USN catch-up evidence. It has two serial phases:

- a disposable Windows source root exercises the production watcher, the tracked foreground cadence
  currently set to 250 ms, background recovery coordinator, and durable queue across independent
  processes;
- an isolated online backup of the retained catalog exercises production metadata inventory against
  the two explicitly authorized roots.

The retained source paths are represented only as `local-primary` and `cloud-primary`. Machine
paths, account identity, and local mapping data are not acceptance output.

## Required evidence

The disposable phase must prove:

- create, modify, same-directory rename, cross-directory move, same-path replacement, and delete
  reach the catalog;
- event-to-visible P95 is no greater than 1,000 ms;
- a write storm coalesces into bounded durable work;
- changes made after the runtime stops converge through the next continuity epoch;
- shutdown remains bounded and continuity creates no full-scan row.

The retained phase must prove:

- a fresh post-backup process opens the retained catalog and makes the initial 500-item gallery
  page plus 4,096-item layout manifest available before continuity work begins;
- each root metadata inventory completes within 45 seconds;
- the inventory invokes no media inspection, preview, signature, or hashing path;
- a separate repeated-inventory safety pass is enclosed by complete Windows directory-entry
  metadata snapshots, and placeholder attributes remain unchanged;
- no full-scan row is created;
- queue, isolated catalog, elapsed-time, and Job Object memory evidence remain bounded.

The production adapter and deterministic test suite separately own no-recall handle behavior,
overflow conversion, cancellation, supersession, incomplete-scope absence rejection, v19-to-v20
migration, full-scan reason allowlisting, lifecycle state, diagnostics, and notification semantics.
Daily and Windows Release repeat those contracts before R2c-M can be accepted.

The cold 45-second inventory is deliberately not preceded by a complete source walk, because that
would warm the filesystem and invalidate the initial measurement. Its first-touch safety is owned
by the no-recall adapter contract. The retained run reports the cold timing and the separately
enclosed repeated-inventory source-safety result as distinct evidence; it does not claim a dynamic
pre/post source snapshot around the cold pass.

## Current evidence

The small retained-catalog fixture passes with additions, modification, removal, and rename routed
through metadata inventory in a fresh measurement process. The final Release disposable production
run recorded 25 mixed operation samples with 557 ms P50 and 828 ms P95 under a 250 ms foreground
poll cadence, 96 storm paths and 676 new observations coalesced into one new retained queue row,
1,188 ms cross-process restart convergence, immediate shutdown, unchanged full-scan rows, and a
745,472-byte isolated catalog.

The authorization, cloud acknowledgement, physical path separation, junction alias, empty storage,
deadline, process ownership, and memory-limit controls passed. The explicitly authorized retained
phase completed against both logical roots in a fresh isolated catalog. `local-primary` inventoried
35,086 entries in 7,674 ms with 1,314 candidates and 33,772 unchanged entries. `cloud-primary`
inventoried 50,472 entries in 14,409 ms with 1,171 candidates and 49,301 unchanged entries. The two
root-scoped cached gallery pages and initial manifest chunks were available in 515 ms, the authorized
layout manifests retained 79,102 items, the source metadata and placeholder-state safety pass
remained unchanged, queue growth was zero, full-scan rows were unchanged, isolated catalog growth
was 8,001,072 bytes, and peak Job Object memory was 1,389,211,648 bytes under the
2,147,483,648-byte limit.

The target-tested implementation passes the complete Daily gate with 461 Rust tests total, 450
passed and 11 authorization-bound or manual performance tests ignored. All Flutter test files,
Windows Scan 2/2, Windows Accessibility 2/2, bridge compatibility, formatting, lint, and whitespace
checks pass. Windows Release and its packaged bridge and same-user process checks pass. The final
independent read-only stage audit reported no Critical, High, Medium, or Low findings.

Phase 24 preserves this historical accepted evidence and keeps the R2c-M entrypoint runnable while
removing its independent `250 ms` cadence constant. The test-only
`ProductionSynchronizationTestHarness` now stores the single shared
`ProductionSynchronizationCadence` parsed from
`tool/library_synchronization_poll_interval_ms.txt`; every R2c-M foreground wait obtains its interval
from that harness. Behavior tests prove both the normal harness and an exact `875\n` policy use the
shared value, and a controlled tracked-policy 875 mutation passes before restoration to 250.

The phase-24 follow-up review found that the first correction still exposed the value as a naked
`Duration` to R2c-M's actual wait helper. A real-path `Duration::from_millis(250_u64)` mutation left
the former semantic source scan and accessor-only test green. The harness no longer exposes an
interval getter: the private-field cadence value owns waiting, and `wait_for` requires that opaque
type as its first parameter. A compile-time signature contract rejects a naked `Duration` with Rust
`E0308`. The cfg(test)-only shared sleeper capture executes the actual R2c-M helper with the controlled
875 ms value and records one exact 875 ms request without sleeping. The non-accessing R2c-M guardrail
passes. No retained catalog, `local-primary`, or `cloud-primary` phase was rerun or accessed for this
maintenance correction.

## Acceptance boundary

R2c-M is accepted with target-scale evidence, complete Daily and Windows Release gates, and an
independent read-only stage audit with no remaining findings. The final R2c integration review
against `main` remains a separate unmerged PR boundary.
