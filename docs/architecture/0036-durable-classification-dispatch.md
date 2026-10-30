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

Schema 67 stores the complete analysis setup alongside its immutable run. Category descriptions
remain in the versioned rule tables; calibrated thresholds are stored as exact floating-point bits
with their calibration identity and bounded policy payload. Reconstruction validates the model,
both rule sets, category identities and profile revision before creating a matcher. A setup row with
no profile explicitly selects uncalibrated analysis; a missing legacy row never implies that choice.
Run creation, setup storage and dispatch activation commit together under separate catalog and
dispatch revision guards. Repeating an already committed activation preserves subsequent pause,
disable or stop intent. Continuing a paused run remains a separate command.

Activation also captures the catalog's source-generation high-water mark. Committed sources at or
below that mark enter the backlog lane; later new or changed generations enter the new-content lane.
The mark survives pause and restart and resets when a new run starts. Existing schema-66 queues
retain their priorities; migration captures the current catalog mark without starting analysis or
reconstructing unknown calibration. These execution records remain outside portable human archives.
Independent arrival and backlog cursors admit at most 64 sources per turn. While backlog remains,
up to 32 places go to arrivals and the remaining places go to backlog. A new source therefore does
not wait for the complete old catalog walk before entering its priority lane.

Schema 69 also records a root's published scan and generation in the same transaction as gallery
publication. Source generations can be allocated during staging and become visible after admission
has passed their number. One coalesced receipt per root retains that handoff across restart, without
inserting an entire scan into the work queue in the publication transaction. A root/scan/generation
index supports bounded distinct-source seeks; renaming a path cannot move its generation behind the
receipt's cursor. The dispatcher considers only ranges already passed by the ordinary cursors,
reserving at most half the new-content budget for these receipts and retaining the 64-source total.
Existing source work keeps its priority, failure state and checkpoint. New queue entries and receipt
advancement commit together. A newer published scan replaces the old receipt; stale namespaces are
discarded and root removal cascades its receipt. Run replacement resets admission over the currently
published catalog, so it clears the superseded receipts. Migration records existing published roots
without changing dispatch intent or reading media. Receipts and this derived index are excluded from
portable human decisions.

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

The desktop opens an opaque application session after the first usable catalog frame. Disabled or
stopped dispatch stays inactive. An enabled or manually paused setup restores its selected local
installation without starting an encoder. The native session initially inhibits admission until
the desktop supplies its latest foreground state. Its blocking loop runs on a bridge background
executor; synchronous activity and cancellation calls only update the dispatch signal.

The desktop lifecycle owner retains the open/run future and joins it during shutdown. A close that
arrives during opening retires the returned session without running it. Close while running cancels
the native turn, waits for execution and availability recovery, and then releases the handle. No
session mutex is held over inference or recovery. Catalog changes wake admission; currently available
root generations feed serial, coalesced recovery. Removed or superseded roots lose pending recovery,
and native generation checks fence in-flight requests. Foreground original reads and import state
reuse the existing desktop owners. Paused/detached application lifecycle states inhibit admission;
ordinary focus loss or a minimized window does not. A memory-pressure event retires resources and holds admission for
one minute before trying the normal quiet interval; the integrated resource check owns that interval's
measurement. These signals never overwrite manual pause or revive a stopped run.

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

Schema 68 records an unavailable physical copy against its work identity, source generation and
root generation. Source selection skips those observations and can immediately try another current,
compatible copy without spending the transient-failure budget. A newly verified copy can wake the
same waiting content task. Successful completion clears its negative observations. Source removal
reconsiders affected waiting work in the removal transaction; the next claim checks the remaining
membership before selecting, waiting or retiring it.

Availability recovery accepts current source selections or a current root generation. Source batches
retain the 256-selection bound and address only the corresponding source and compatible content
tasks. Root recovery processes at most 64 observations per transaction under a fixed failure-identity
boundary. A later negative observation receives a new identity, so an earlier recovery page cannot
erase it. Recovery preserves manual pause, does nothing while disabled or stopped, and never retries
permanent failures or resets the failure budget. Native controls signal committed queue changes
without cancelling unrelated active inference. These observations remain derived execution data,
outside portable human archives. Migration requeues legacy waiting work once because schema 67 did
not retain the failed-copy identity; it preserves durable mode and opens no source files.

## Validation and replacement

Migration checks preserve prior human records and prove atomic rollback. Repository tests exercise
duplicate admission, fairness, stale leases, stop/disable/pause, restart, source replacement, bounded
retries and atomic result/checkpoint publication. Model/foreground measurements remain in R5-04's
integrated verification; this decision assigns authority and does not claim those outcomes.

The scheduler may change its batching or execution strategy behind the work port. Changing durable
keys or lease semantics requires a forward migration. Queue/cache reclamation never deletes human
decisions or changes stopped-run intent. Older application versions use their preserved migration
snapshot for rollback rather than opening the newer schema.
