# R2c-M replacement reliability

Status: target-scale evidence pending current authorization

## Scope

This gate closes the ADR 0023 replacement continuity model rather than reusing the historical
R2c-H USN catch-up evidence. It has two serial phases:

- a disposable Windows source root exercises the production watcher, 250 ms foreground cadence,
  background recovery coordinator, and durable queue across independent processes;
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
through metadata inventory in a fresh measurement process. The Release disposable production run
recorded 25 mixed operation samples with 586 ms P50 and 871 ms P95 under a 250 ms foreground poll
cadence, 96 storm paths and 677 new observations coalesced into one new retained queue row, 1,245 ms
cross-process restart convergence, immediate shutdown, unchanged full-scan rows, and a
745,472-byte isolated catalog.

The authorization, cloud acknowledgement, physical path separation, junction alias, empty storage,
deadline, process ownership, and memory-limit controls are active. The target-scale retained-root
phase has not run in this stage because the repository-local mapping is discovery data, not current
authorization.

The complete Daily gate passes on the final audited implementation with 459 Rust tests total,
448 passed and 11 authorization-bound or manual performance tests ignored. All Flutter test files,
Windows Scan 2/2, Windows Accessibility 2/2, bridge compatibility, formatting, lint, and whitespace
checks pass. Windows Release and its packaged bridge and same-user process checks pass. The latest
independent read-only implementation audit reports no Critical, High, Medium, or Low findings; the
authorization-bound target result and its final stage audit are still pending.

## Acceptance boundary

R2c-M remains incomplete until the explicitly authorized target-scale run passes, complete Daily
and Windows Release gates pass on the same implementation, and the stage receives an independent
read-only audit with no remaining Critical, High, Medium, or Low findings.
