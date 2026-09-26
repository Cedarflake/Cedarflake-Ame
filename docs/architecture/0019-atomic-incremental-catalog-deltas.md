# ADR 0019: Publish incremental catalog deltas atomically

- Status: Accepted
- Date: 2026-08-18
- Last amended: 2026-09-05

## Context

ADR 0016 defines final-state reconciliation and retain-or-invalidate semantics. ADR 0018 persists
the normalized work in a leased SQLite queue. R2c-D must connect those contracts to the active
catalog without reusing the complete-snapshot publisher or allowing queue acknowledgement,
location changes, preview ownership, and the catalog revision to become separately visible.

Filesystem notifications remain hints. The active file state, optional Windows file identity,
media inspection, and an immediate prepublication revalidation remain authoritative. A failed,
stale, partial, or superseded attempt must leave the last trustworthy catalog visible.

## Decision drivers

- preserve asset identity for identity-proven edits and same-volume moves;
- prevent same-path replacements from inheriting incompatible evidence;
- publish a related bounded batch at one catalog revision;
- make queue completion and catalog mutation one crash-safe transaction;
- reject stale lease, root generation, full-scan, and catalog-revision boundaries;
- retain compatible previews across a rename and invalidate them after content change;
- prevent concurrent preview cleanup or reclamation from being overwritten by prepared work;
- keep transaction work proportional to the bounded affected location set;
- keep all source access read-only and bounded outside the SQLite transaction.

## Considered options

### Apply and acknowledge each queue row separately

Rejected. A rename, replacement, or related file burst could become partially visible, and a crash
between catalog mutation and queue acknowledgement could duplicate or lose publication evidence.

### Convert every change into a complete root scan

Rejected. It preserves snapshot atomicity but defeats R2c's incremental outcome and repeatedly
walks large roots for ordinary single-file changes.

### Stage final-state evidence and publish one guarded delta transaction

Accepted. The application inspects and revalidates a bounded lease batch outside SQLite. The
SQLite adapter then rechecks every publication guard under an immediate writer transaction and
atomically applies the complete catalog delta.

## Decision

The Ame-owned `IncrementalCatalogRepository` extends the existing catalog boundary. It loads the
active root and locations by relative path or file identity, then accepts one `CatalogDeltaBatch`.
No SQLite, watcher, or platform type crosses the port.

Each mutation records both the reconciliation outcome and its derived-evidence disposition. The
adapter rejects inconsistent combinations before opening the publication transaction:

- add and replacement require `NoReusableEvidence` plus a new location;
- modify requires `InvalidateDerived` plus a replacement location record;
- rename or move requires `RetainCompatible` or `InvalidateDerived` plus a new location record;
- unchanged identity backfill requires `RetainCompatible` plus an updated location record;
- removal requires `RemoveFromCurrentProjection` and one or more old location IDs.

The application processes path-scoped work as follows:

1. load the registered root, durable generation, active published scan, and catalog revision;
2. verify root availability and require an active published snapshot; while a replacement full
   scan is running, defer P1/P2 work but allow the reserved P0 Live lane to continue against that
   active snapshot;
3. lease only path work, leaving subtree, root, and freshness-gap rows untouched for R2c-F;
4. reject intermediate symlink or Windows reparse traversal before reading a relative path;
5. inspect path metadata, placeholder state, optional Windows file identity, and media dimensions;
6. apply ADR 0007 and ADR 0016 to decide unchanged, add, modify, rename, replacement, or removal;
7. reconcile both affected paths of a paired rename, including a replacement recreated at the old
   path and Windows case-only spelling changes;
8. persist newly available identity on an otherwise unchanged location while retaining the exact
   prior location identifier, including after v17 path normalization, and preserve complete
   compatible preview state including structured failure evidence;
9. revalidate present files, filesystem containment, and authoritative absence immediately before
   publication;
10. publish ready independent work together while returning unreadable files and cloud-only
    placeholders to bounded retry. Placeholder inspection preserves the last trustworthy catalog
    evidence, performs no content access, and cannot terminally complete the durable path work.

A first import still owns the only publication boundary because there is no active snapshot to
update. For a replacement full scan, P1/P2 work that discovers the scan after leasing is a
coordination deferral, not a processing failure: the queue returns the lease to ready state and
restores its attempt budget. P0 Live work does not defer. It publishes to the active snapshot and
mirrors the same mutation into the exact running scan's staging in one transaction. If the active
published catalog boundary disappears during preparation, every lane fails closed.

The publisher uses `BEGIN IMMEDIATE` and, before changing catalog state, rechecks:

- the root still exists and its durable generation remains active;
- either no full scan is running or paused, or the batch is entirely P0 Live work and the exact
  running scan can receive the same staged mutation;
- the referenced active scan remains a completed published snapshot;
- the global catalog revision still equals the prepared revision;
- every queue row is still leased by the exact lease generation in the batch.
- every `RetainCompatible` mutation still sees the preview path, lifecycle status, and structured
  issue evidence from which it was prepared.

Only after those guards pass does the transaction detach obsolete preview ownership, remove old
locations, upsert new locations, transfer compatible ready-preview ownership, stale affected
unreferenced preview artifacts, delete affected orphan assets, apply the bounded active-location
count delta, increment the catalog revision once when at least one mutation exists, and complete
every included queue lease at that same revision. When a replacement scan is running, the same P0
transaction also mirrors create, replace, removal, terminal-media, and physical-identity hardlink
effects into that scan's staging. The scan never captures, leases, or completes P0 rows, including
P0 work already leased when `begin_scan` runs. Abandoning the scan discards only its staging and
does not undo the active P0 result or queue completion.

Before a replacement scan publishes, it rejects unfinished P0 work for the root, reconciles each
staged location against the current active generation, and recomputes the staged location count
inside the writer transaction. A higher active generation wins over older staged evidence;
compatible observations keep the greater generation; incompatible equal-generation evidence fails
closed. The caller's previously counted staging rows are validation input, not publication
authority. A `Ready` preview cannot be committed without a live ready artifact owner. An unchanged
batch completes its leases without creating a meaningless revision.

The batch is bounded to 128 completions, 256 mutations, and four explicit removals per mutation.
All filesystem and image work happens before the transaction; the transaction performs no source
access. R2c-D introduces no schema migration and continues using schema v17.

Configured-root unregistration uses the same all-or-nothing storage boundary but is a distinct
user-interactive command rather than a filesystem-observation delta. It waits for any active writer
to finish and never preempts a P0 commit already in progress. Once waiting, it is admitted before
new Live, Journal, or Recovery writers, so a continuous notification stream cannot starve the user
command; the UI still receives control because unregistration and catalog reads run on asynchronous
bridge workers. Inside one immediate transaction the adapter materializes only
the selected root's affected asset and preview-artifact identifiers in connection-local temporary
tables, retires that root's queue and scan state, detaches only its preview owners, stales only its
newly unreferenced artifacts, and deletes only newly orphaned affected assets. Shared assets,
remaining preview owners, unrelated orphan records, and source files are untouched. Failure rolls
back the complete removal and catalog revision. This bounded cleanup uses existing root-, location-,
asset-, and artifact-key indexes and does not require a schema migration. A native success or
already-absent result is an idempotent committed outcome. The presentation controller must then
remove the affected root from every loaded projection immediately and publish a bounded first page;
the O(remaining-library) timeline aggregation is not part of removal completion. It runs as a
separate background read whose request sequence, query, catalog revision, and query identity must
still match before publication. A post-commit first-page failure retains the committed root identity
for reload-only recovery and cannot issue unregistration a second time.

Physical SQLite page reclamation is deliberately outside that transaction. After a successful
unregistration, the application releases the catalog connection and returns the committed result
before scheduling maintenance. Maintenance may only compact pages SQLite already placed on its
freelist; it cannot delete rows, revise catalog state, or weaken the root-removal scope. Failure,
cancellation, insufficient temporary capacity, or a busy maintenance window therefore leaves the
logical removal committed and retryable without endangering shared assets, remaining preview
owners, queue history, user data, or source files. A newly requested unregistration preempts active
maintenance before joining the user-interactive writer lane. That admission wait is capped at five
seconds and returns `catalog_user_interactive_write_timeout` if the interrupted maintenance
connection does not exit; timing out removes the queued priority waiter before background work may
continue. Every write-maintenance outcome invalidates the process-owned session metadata for the
catalog path, so the next request reopens and fully validates the database after `VACUUM` rather
than reusing pre-maintenance prepared state.

## Validation gates

- controlled valid images prove unchanged, add, edit, metadata-engine reinspection, paired rename,
  same-path replacement, authoritative removal, rename-followed-by-removal, identity backfill,
  old-path replacement, and Windows case-only rename outcomes;
- migrated v17 identity backfill reuses the active legacy location identifier instead of creating a
  second location from the normalized path spelling;
- related valid files publish at one revision and one malformed image cannot block an independent
  valid sibling;
- a new cloud-only placeholder creates no location, an existing placeholder retains its last
  trustworthy location, and both remain durable unresolved work without hydration;
- identity-preserving rename transfers compatible preview ownership atomically and preserves
  structured failed-preview evidence;
- preview cleanup invalidates an already prepared retain-compatible delta without changing the
  catalog revision;
- path-only leasing and normal P1/P2 coordination deferral do not consume authoritative or retry
  work;
- intermediate filesystem links cannot escape the selected root, while an explicitly selected
  linked root remains valid;
- unrelated orphan assets and preview artifacts are not visited or rewritten by one delta;
- stale lease, changed revision, and retired generation publish no delta; a running replacement
  scan admits only an all-P0 Live batch and receives the same create, replace, removal, terminal,
  and hardlink effects in staging;
- `begin_scan` leaves pending and already leased P0 ownership untouched, scan publication rejects
  unfinished P0, cancellation preserves P0 state, and an older staged generation cannot overwrite a
  newer active generation;
- an injected queue-completion database failure rolls back locations, revision, and completion;
- inconsistent outcome and evidence-disposition combinations fail closed;
- controlled fixture bytes are unchanged after reconciliation;
- root unregistration preserves shared assets and preview owners, does not rewrite unrelated orphan
  assets or ready artifacts, rolls back its complete cleanup on failure, and receives the next write
  permit after an active P0 transaction even when another Live writer is already queued;
- post-removal page maintenance is separately cancellable and retryable, and an injected
  maintenance interruption cannot reverse the committed unregistration or rewrite a remaining
  shared asset and preview owner;
- Rust format, Clippy with warnings denied, the complete Rust suite, and repository Daily pass.

## Validation evidence

The focused adapter and application fixtures are recorded in
`docs/acceptance/r2c-d-incremental-delta-publication.md`. They use temporary controlled roots and
isolated catalogs. No real-library root or cloud placeholder was accessed.

## Consequences and risks

- A single-file or related bounded path batch no longer requires a root-wide scan.
- First import retains precedence because no active snapshot exists. During a replacement full
  scan, P1/P2 work waits without consuming retry attempts, while P0 Live work keeps its reserved
  event-to-visible path and is mirrored into scan staging.
- A filesystem change after final revalidation is still possible. A newer overlapping durable
  event supersedes the lease; the transaction rejects the stale worker before publication.
- Independent good work may publish while a malformed sibling retries. A reliably paired rename
  remains one lease; both old and new final states publish atomically, so a recreated old path
  cannot retain stale catalog evidence.
- Subtree enumeration, root freshness recovery, cancellation escalation, and low-frequency audit
  remain R2c-F responsibilities rather than being approximated by unsafe partial removal.
- R2c-E must connect the returned bounded counts and catalog revision to the existing Flutter
  refresh and anchor-preservation workflow; Flutter does not infer reconciliation policy.

## Replacement strategy

Replace only the incremental catalog port or its SQLite adapter. A replacement must preserve the
same outcome/disposition contract, prepublication revalidation, first-import precedence, P1/P2 scan
deferral, P0 active-and-staging atomicity, newer-generation precedence, unfinished-P0 publication
guard, lease and generation guards, one-revision visibility, preview ownership rules, and
all-or-nothing queue completion. A storage replacement cannot require source mutation or move
policy into Flutter.
