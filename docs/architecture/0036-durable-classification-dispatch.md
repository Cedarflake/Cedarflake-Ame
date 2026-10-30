# ADR 0036: Durable classification dispatch and publication authority

Status: accepted implementation decision

Date: 2026-10-30

## Context

The R5 lifecycle requires explicit pause across restart, incremental work before global duplicate
analysis finishes, shared analysis of identical content, and rejection of late results after stop
or configuration changes. Worker residency under ADR 0034 owns processes and memory; it cannot
establish durable scheduling intent or authorize catalog publication. ADR 0035 owns human decisions
and immutable prediction evidence, independently of these execution records.

## Decision

Schema 66 adds separate dispatch state, stopped-run identities, and bounded-query work records to
the catalog. These are execution state, excluded from portable decision archives. A typed repository
owns short transactions for admission, claims, checkpoints and result publication. No transaction
is held during source reads, decoding, model initialization or inference.

Dispatch intent distinguishes disabled, automatic, manually paused and stopped. A process session
has its own identity; each control transition advances a durable epoch. A claim binds that epoch,
session, run, content configuration, selected physical source and a monotonic lease identity.
Publication rechecks all of them in the same transaction as the immutable prediction and checkpoint.
Foreground yielding does not change manual intent. Startup replaces abandoned claims, retaining the
manual pause; repeated interrupted attempts exhaust the same bounded failure budget as worker errors.
The application retires an old process before admitting its replacement under ADR 0034.

Stopping records the run identity permanently. Cache misses, process restart and ordinary automatic
dispatch cannot restart it. Explicit enablement supplies a new run; compatible committed evidence
and features remain reusable. A semantic rule change may replace an active run while preserving a
manual pause. Human classification, Favorites, album membership and review history are unaffected.

Source verification work is keyed by physical source generation and configuration. Confirmed content
work is keyed by fingerprint scheme, digest, length and configuration; multiple locations therefore
join one compatible task. Source admission uses bounded committed-catalog windows, without directory
enumeration. The existing guarded exact reader verifies sources without a same-size duplicate peer.
Each verified source can continue independently; the complete duplicate scan is not a prerequisite.
Current source membership and exact evidence are checked again before inference/publication.

Requested scope, new content and backlog use a seven-slot weighted admission cycle: requested,
requested, new, requested, new, requested, backlog. Empty lanes are skipped. Ready work within a lane
uses arrival/retry time and stable work identity. This bounds starvation without assigning a worker
per lane. A source or worker transient failure permits at most three failed attempts; unavailable
sources wait for a relevant availability signal or explicit retry, and permanent failures require
changed content or explicit retry. Cancellation/yield at a completed checkpoint consumes no attempt.

## Validation and replacement

Migration checks preserve prior human records and prove atomic rollback. Repository tests exercise
duplicate admission, fairness, stale leases, stop/disable/pause, restart, source replacement, bounded
retries and atomic result/checkpoint publication. Model/foreground measurements remain in R5-04's
integrated verification; this decision assigns authority and does not claim those outcomes.

The scheduler may change its batching or execution strategy behind the work port. Changing durable
keys or lease semantics requires a forward migration. Queue/cache reclamation never deletes human
decisions or changes stopped-run intent. Older application versions use their preserved migration
snapshot for rollback rather than opening schema 66.
