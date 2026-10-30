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

Classification verification selects one leased catalog source even when no same-size peer exists;
the ordinary R3 duplicate scan retains its peer pruning. Existing compatible fingerprints can be
published through the shared bounded gallery/organization transaction without rereading bytes.
Otherwise the guarded reader holds source and namespace handles through a transaction that checks
the classification claim, records the exact fingerprint, publishes group membership and completes
the source task. Fingerprints retain the exact engine's immutable run provenance. Classification
verification has a distinct run identity and never emits the full duplicate scan's UI progress.
Cancellation before commit rolls back both group publication and the task checkpoint.

Content analysis loads its immutable run and exact selected source under a short transaction, then
releases the transaction before opening or decoding media. The application checks that its matcher
and encoder describe the same stored run. It reuses a compatible feature or decodes and encodes one
source under ADR 0034's held capability. Claim/source authority is checked before inference and again
before publishing the cached feature; source revalidation also precedes the final prediction commit.
Pause, stop, a replaced run/session, removed source or changed generation rejects the old completion.
Cancellation observed while inserting a prediction rolls back its task checkpoint as well.

Prediction identity binds the run, session, task, lease and epoch. A repeated completion cannot add
another prediction after its task has completed. A completed compatible feature may survive a failed
catalog transaction and be reused by the next admitted attempt; it carries no completion or human
decision authority. Physical reads, model work and feature-cache transactions never hold the catalog
transaction. Foreground scheduling and automatic retry remain the dispatcher owner's responsibility.

Native exact-source verification and content inference share one application-owned background slot.
Admission is FIFO at each file/content boundary, so a classification task can run between exact
duplicate reads while the global duplicate scan continues. At most eight callers may wait; waiters
observe cancellation every 25 ms and never acquire a source or database transaction while waiting.
The exact-reader capability retains admission through fingerprint publication and drops its held
source before releasing the slot. Content execution acquires admission before opening its catalog
session and retains it through source retirement. Cancellation does not release another task's slot
or change durable manual intent. Text-model initialization and each uncached description use the
same slot independently, allowing exact reads between descriptions. Image initialization remains
within its content task's admission, without acquiring the slot twice.

The native dispatcher owns its repository session and one executor. It processes a bounded source
window and one claimed task per turn. Six indexed queue seeks provide the earliest retry deadline
without materializing task lists. An empty queue waits on a coalesced condition-variable signal;
the earlier of a retry deadline and the resident encoder's idle deadline supplies a timed wake.
Notification identity is captured before the turn, preventing a signal between the queue check and
wait from being lost. Transient retries wait five seconds from the observed failure and retain the
durable three-attempt budget.

The initial admission requires three quiet seconds. Cached browsing resets this interval only
before admission or after the queue becomes idle. Actual foreground source reads, import, suspend
and memory pressure cancel the active turn and request model retirement. Readiness resumes after
quiet without modifying durable manual intent. Manual controls signal cancellation before the
short persistence transaction and signal its committed result afterward; only the persisted epoch
authorizes publication. Close retires the runtime while preserving the selected durable mode.

One native runtime may own model resources in an application process. A replacement cannot acquire
ownership until the old runtime has retired and dropped its executor. Unproven retirement retains
process admission until process restart. Model preparation remains demand-driven: source-only work,
manual pause and an empty queue do not start an encoder. The native setup must describe the exact
stored run, rules and optional screening profile; an unavailable profile cannot silently become an
uncalibrated replacement. Setup or unknown execution errors end the loop with queued work retained,
while unavailable, corrupt and transient per-source failures retain their distinct queue outcomes.
File I/O failure during decoding is an unavailable-source outcome, separate from corrupt image data.

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
