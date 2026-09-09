# Library continuity acceptance contract

Status: retained R2c regression and acceptance requirements, not proof of completion.

The [roadmap](../roadmap.md) owns stage order and current status; the
[closeout execution document](../plans/r2c-closeout.md) owns the active bounded investigation.
These requirements describe the behaviors to prove. Accepted
[ADR 0024](../architecture/0024-windows-change-driven-library-continuity.md) owns the current
change-driven architecture and supersedes historical fallback policies; other technical ownership
remains in the [ADR index](../architecture/README.md). This checklist cannot grant new recovery,
filesystem, service, or retained-library authority or replace an ADR when implementation changes.

## Safety and authority rules

- Filesystem notifications, brokered journal records, and staged recovery inventory rows are hints
  that identify what must be checked. They are never accepted as the final file state.
- P0 live work has reserved execution capacity and may publish while P1 catch-up or P2 recovery is
  incomplete. Historical continuity is not a precondition for presenting a newly revalidated change.
- The filesystem plus Ame's already accepted identity and source-state revalidation remain the
  evidence used to reconcile the catalog.
- R2c observes and reconciles source state without changing it. It does not delete, move, copy,
  rename, rewrite, hydrate, or normalize any source file.
- Offline and recall-on-data-access placeholders are identified before content access. Continuous
  synchronization must not download a cloud-only file merely to classify an event.
- An unavailable root retains its last trustworthy catalog. Inaccessibility is not evidence that
  every location has been deleted.
- Only a completely reconciled path or subtree can authoritatively remove locations that are no
  longer present. A partial or failed pass cannot publish a complete-removal claim.
- A batch of related changes is visible at one catalog revision. The UI sees either the prior
  revision or the complete new revision, never a half-applied rename or replacement.
- Full-root scanning is permitted only for first import, the explicit `更新图库` action, or resumption
  of a foreground checkpoint created by either action. Every request carries one of these typed
  reasons.
- Normal create, modify, delete, rename, move, process start, watcher restart, evidence loss,
  overflow, retry, inventory size, availability recovery, and automatic reconciliation failure must
  not trigger a complete root scan.
- Ordinary startup with a continuous checkpoint must not enumerate source-root entries, create a
  metadata-inventory run, read media content, or hydrate a placeholder.
- The broker may read only journal metadata for caller-authorized roots. It must not read media,
  mutate source files, configure the journal, open the catalog, or disclose root-external records.

## Ownership and boundaries

The Rust domain defines Ame-owned, platform-independent values for:

- library-root identity and configuration generation;
- normalized change intent, such as path reconciliation, rename candidate, subtree reconciliation,
  and root freshness unknown;
- change origin and priority lane, including P0 live notification, P1 brokered journal catch-up,
  P2 baseline or continuity-gap recovery, and user refresh; historical direct-desktop
  `StartupCatchUp`, `consistency_audit`, and ADR 0023 startup-inventory values remain readable only
  for forward migration and do not regain production authority;
- reconciliation outcomes: unchanged, added, modified, renamed or moved, replaced, removed,
  skipped, retryable failure, and terminal issue;
- watcher health and catalog-freshness states without exposing a Windows or third-party type.

The Rust application layer owns:

- starting and stopping change observation for configured, available roots;
- negotiating broker capability, validating per-root journal continuity, and scheduling shared
  per-volume reads without coupling root progress;
- converting raw signals into Ame change intents;
- durable enqueueing, enqueue-before-checkpoint, debounce, coalescing, priority isolation, retry,
  backoff, pause, cancellation, and recovery;
- deciding whether the minimum safe scope is one path, a subtree, root metadata reconciliation, or
  a complete scan;
- invoking the existing source-state, file-identity, metadata, and preview ports;
- atomically applying bounded catalog deltas and incrementing the catalog revision;
- precise retain-or-invalidate decisions for metadata, previews, fingerprints, similarity, and
  future classification evidence, expressed through stable asset identity rather than paths;
- publishing bounded status and revision events to Flutter.

Ports must remain narrow and should extend an existing natural boundary instead of creating a
second synonym for it. The implementation must at least evaluate these responsibilities:

- `LibraryChangeSource`: streams normalized hints and health transitions;
- `PersistentChangeJournal`: queries and streams bounded caller-authorized root changes without
  exposing Win32, service, volume, or raw USN types;
- `ChangeQueue`: durably records, leases, acknowledges, retries, and supersedes pending intents;
- `IncrementalReconciler`: checks a path or bounded subtree and returns Ame reconciliation results;
- `CatalogDeltaPublisher`: applies one batch at an atomic revision boundary;
- `MetadataInventory`: enumerates and pages current metadata evidence for an explicit P2 baseline or
  proven continuity gap without reading media content.

Names are illustrative, not mandatory APIs. Before adding a port, inspect whether an existing scan,
catalog, or filesystem contract already owns that responsibility.

Adapters own all platform and dependency details:

- evaluate the mature Rust `notify` ecosystem for recursive live observation on Windows and record
  its selected version, license, maintenance, cancellation behavior, overflow semantics, packaging,
  and replacement strategy before admission;
- keep `notify` event kinds, paths, errors, threads, and global state behind the adapter;
- keep service control, named-pipe, caller-token, volume-handle, journal-buffer, file-reference, and
  root-containment details inside the Windows broker adapter;
- continue using ADR 0007's Ame-owned Windows `FILE_ID_INFO` evidence for reconciliation instead of
  inventing another asset-identity rule;
- persist the durable queue, retry state, per-root journal checkpoint, covered range, cross-root
  lineage, inventory epoch and staging state, and delta publication through the Rust SQLite adapter;
- never reopen a volume from the desktop process or request UAC during ordinary startup. Service
  install and update belong to the signed installer and use explicit one-time administrative consent;
- keep Flutter presentation-only. Flutter does not watch directories, enumerate roots, write SQL,
  call the broker, interpret USN, or infer catalog policy from platform events.

## Durable change intent

The logical persistent model must be able to express, without committing prematurely to one table
shape:

- a stable change ID and `root_id`;
- the root configuration generation so work for an unregistered or replaced root cannot publish;
- one affected relative path and an optional old path or rename-correlation identity;
- normalized intent kind and origin;
- P0, P1, or P2 lane and the source batch or journal range that owns the evidence;
- first-observed and most-recent-observed time;
- coalesced event count;
- pending, leased/in-progress, retry-wait, completed, and superseded states;
- attempt count, next retry time, and structured last failure;
- the catalog revision at enqueue and successful publication;
- the owning per-root journal checkpoint and covered boundary, or metadata-inventory epoch, scope,
  page cursor, and completion authority where applicable;
- deterministic terminal media evidence keyed by root, normalized path, complete source state, and
  inspection-engine identity so unchanged unsupported or malformed files are not decoded or retried
  again on every startup.

Per-root continuity state also binds root generation, volume identity, root file identity, journal
ID, next unread USN, covered catalog revision, broker protocol, and last structured failure. Roots
on one volume may share one bounded physical read, but a root advances only after its own work is
durably enrolled. No all-roots transaction may let one unavailable root block another root's P1
checkpoint.

This state is durable task data, not disposable thumbnail cache. Its schema changes require forward
migrations from every committed schema version and migration tests. Completed rows and obsolete
watermarks require a bounded retention strategy, but cleanup must never erase an unresolved gap or
user-owned decision.

## Event normalization and coalescing

Raw filesystem events may be duplicated, reordered, incomplete, or delivered after the path changes
again. The inbound callback must remain lightweight: it normalizes and enqueues a hint without
running image decoding, a long SQLite transaction, a directory walk, or Flutter work on the callback
thread.

Live observations enter P0. P0 owns reserved admission, leasing, revalidation, and publication
capacity that P1 journal replay and P2 inventory cannot consume. P1 and P2 check for higher-priority
work at every bounded page and yield before beginning another page. Persisting `priority` without
that execution isolation is insufficient.

After a short, testable stabilization window, apply at least these rules:

- `create` followed by any number of `modify` events becomes one reconcile-or-add intent;
- repeated `modify` events for one path become one reconciliation;
- `create` followed quickly by `delete` is checked against final filesystem state rather than
  assumed to be a no-op;
- a reliably paired `rename(old, new)` is handled as one atomic intent;
- an unpaired rename degrades to an old-path reconciliation and a new-path reconciliation;
- a directory rename, move, or removal marks the minimum affected subtree instead of materializing
  an unbounded event for every known descendant;
- a stronger parent-subtree intent supersedes unleased child-path intents;
- a later event for the same path prevents an earlier leased result from overwriting newer state;
- application-owned catalog, preview, temporary, log, and model storage is excluded and must not be
  located inside a source root in the first place.

An initial debounce range may be measured around 350–1000 ms, but the final value must be justified
by controlled event-burst evidence rather than copied as a permanent constant. In-memory ingress,
database leases, batch size, retry count, and concurrency must all be bounded. Under a storm Ame may
be delayed; it must not grow memory without limit or silently drop events while claiming `已同步`.

## Incremental reconciliation

For every stable path or subtree intent:

1. Verify that the root still exists in Ame, its configuration generation matches, and its current
   availability permits inspection.
2. Inspect root and path metadata before content. Distinguish missing, directory, regular file,
   offline placeholder, inaccessible, locked, and unsupported states.
3. Stop before content access for offline or recall placeholders and preserve explicit availability
   evidence.
4. For a locally readable candidate, reuse the existing format evidence, source state, optional
   Windows file identity, and metadata compatibility rules.
5. Compare against the current published location using ADR 0007's order of evidence.
6. Reuse derived evidence only when source state and engine identity remain compatible. Otherwise
   invalidate only what can no longer be trusted.
7. Revalidate required identity and state immediately before publication. If the file changed again,
   return the intent to the queue instead of publishing stale evidence.
8. Publish the complete bounded batch and one new catalog revision in a single transaction.
9. Admit only formats supported by the pinned decoder. Unsupported formats, malformed content, and
   decoder-limit violations complete once as terminal per-file evidence; only open, lock, read, and
   source-race failures consume retry attempts.

Required semantics:

- New local file: add a location; do not infer permanent logical identity from its path.
- Unchanged file: retain orientation-corrected dimensions and compatible preview artifacts; do not
  repeat metadata analysis or preview generation and do not create a meaningless visible refresh.
- In-place edit: preserve logical asset identity when accepted platform evidence supports it, while
  invalidating stale dimensions, preview, metadata, fingerprint, similarity, and classification
  evidence. Continue publishing the last trustworthy revision until replacement dimensions and the
  complete bounded delta can publish atomically.
- Same-volume rename or move: preserve the asset when identity matches and replace its location
  atomically; compatible dimensions and preview artifacts follow the stable identity instead of
  remaining attached to an obsolete path.
- Replacement at the same path: create a new asset and prevent it from inheriting the former
  file's dimensions, preview artifacts, other derived evidence, or user decisions.
- Removal: remove the published location only after an authoritative observation; do not let a
  delayed delete remove a new replacement now occupying that path. When the last active location is
  authoritatively removed, current derived projections must no longer surface the asset and its
  unreferenced previews become eligible for bounded reclamation.
- Cross-volume move: treat delete and create evidence conservatively unless a separately admitted
  stronger identity proves continuity; never transfer classification merely because names match.
- Directory change: enumerate only the minimum subtree in bounded windows. Absence is authoritative
  only for the scope that completed successfully.

Full scans continue to stage and atomically replace a complete root snapshot. Incremental work uses
atomic delta publication but must retain the same trust rule: failed, cancelled, stale, or partial
work does not replace trustworthy state.

## Query, preview, and presentation consistency

- Every published delta increments the same catalog revision used by bounded keyset queries.
- Existing stale-cursor protection remains authoritative. Flutter handles a revision change through
  an Ame-owned refresh contract rather than querying SQL or rebuilding the whole application.
- Stable asset and location identity is used to merge a bounded update while preserving the active
  source, filters, sort, selection, preview, and visible scroll anchor when possible.
- A rename must not briefly appear as both a removed tile and an unrelated new tile.
- An edited visible image invalidates and recreates only the necessary preview; off-screen previews
  remain bounded and demand-driven.
- A dimensions change publishes with the same atomic catalog revision as its source-state change.
  Flutter assembles the replacement manifest and layout snapshot separately, keeps the last
  trustworthy geometry until the replacement is complete, and preserves a compatible logical
  viewport anchor. It never clears a tile to a transient square merely because reinspection or
  preview generation is pending.
- Preview demand and publication carry the exact root, active scan, location, source generation,
  source revision, algorithm, orientation, and size-bucket identity. The global catalog revision
  protects query snapshots but is not part of this source lease, so an unrelated catalog write does
  not supersede otherwise exact preview work. A late result may populate only the matching preview
  entry; it cannot restore an obsolete path, overwrite newer evidence, or mutate layout dimensions.
- Every bounded delta exposes enough stable identity and evidence disposition for later analysis
  consumers to retain compatible results after a rename, invalidate them after content change or
  replacement, and remove them from current projections after authoritative deletion. R2c defines
  this contract without implementing R5 classification.
- If the currently previewed file is removed, replaced, unavailable, or offline, the viewer presents
  a clear state and a safe return path instead of displaying stale bytes as current.
- Synchronization remains part of the existing library and source workflow. It does not create a
  sidebar Task entry or a second gallery.
- `更新图库` explicitly requests an application-owned full scan for the selected root. Automatic
  watcher and journal reconciliation remain the normal incremental path; exceptional metadata
  inventory remains application-owned P2 recovery. Flutter does not enumerate files.

## Lifecycle and race handling

Startup order:

1. Load the last trustworthy catalog, root configuration, unresolved change queue, and any
   recoverable foreground explicit full-scan checkpoint. Retire prerelease
   `authoritative_recovery` full-scan checkpoints without resuming them.
2. Check each root's availability using metadata only.
3. Establish live observation before continuity work so new events do not open another avoidable
   gap.
4. For each broker-capable local NTFS root, query the current journal identity and exclusive end
   boundary. If its checkpoint is continuous, enqueue only the missing P1 range and perform no root
   enumeration when that range is empty.
5. Process P0 immediately while P1 catches up. Advance a root checkpoint only after its bounded
   range is durably enrolled; report `已同步` only after that checkpoint covers the declared boundary
   and required queue lineage is terminal.
6. Start P2 metadata inventory only for a one-time migration or first-authority baseline, or after a
   specific continuity failure. Keep P0 live during recovery, replay the closing journal interval,
   and require complete scope authority before publishing absence.
7. Project an unsupported persistent-change source as `LiveOnly` or an explicit blocked capability;
   never compensate with an automatic complete inventory on every process start.

First import uses its foreground scan as the first-authority baseline rather than scheduling a
second metadata inventory. Its opening journal boundary and healthy observer are established before
enumeration, its first snapshot may publish while scan-window P0 work remains queued, and its
closing journal interval plus P0/P1 queues must converge before the per-root finalizer can publish
`Current`. Ordinary changes during that scan, including a same-path content replacement, never
invalidate the whole first import or turn into a user-visible retry request. After an unfinished
first import loses its execution owner, startup restores only its paused task. Explicit Continue
re-establishes observer-first capture and rebuilds that unpublished inventory unless persistent
coverage proves the observation gap; the current resume adapter does not supply that proof. A
published baseline instead retains ordinary bounded replay without another complete enumeration.

Root changes:

- A newly added root completes its first trustworthy full scan before live deltas are applied to the
  published result; events arriving during the scan wait behind that publication boundary.
- Removing a root stops observation and invalidates its old generation. Unregistering a source from
  Ame never deletes or modifies its files.
- A changed root path or policy receives a new generation so old queued work cannot publish into the
  new configuration.
- An offline or disconnected root pauses processing. It does not publish mass removals.

Shutdown:

- stop accepting new live callbacks;
- immediately hide the desktop window so background teardown cannot present as an application hang;
- suspend a running foreground full scan at a durable checkpoint so the next process can resume it;
- cancel watcher, metadata inventory, path, subtree, and bounded root work; on the next start
  establish a new live watcher boundary and continue from the last durably advanced per-root journal
  checkpoint;
- safely return any currently leased non-scan batch instead of treating its old in-memory state as
  evidence for changes that may occur while Ame is closed;
- persist foreground full-scan checkpoints and durable catalog/queue state, but discard superseded
  inventory staging after replacement authority exists; preserve journal checkpoints only through
  their last durably enrolled boundaries;
- use bounded graceful shutdown so a watcher or queue cannot hang the window close path;
- leave foreground full scans and durable queue work recoverable on the next startup without keeping
  the window visible.

## Failure and degradation matrix

- Unsupported, malformed, or decoder-limit media: record one structured terminal issue, persist its
  source-state and engine-version evidence atomically with completion, and continue the batch without
  consuming retry attempts.
- Temporarily unreadable or locked file: preserve the last trustworthy location and retry with
  bounded backoff; a later successful or terminal result closes the row.
- File changes again during processing: fail final revalidation, coalesce the newer event, and retry.
- Notification buffer overflow or known event loss: mark live observation degraded and replay the
  continuous P1 journal interval. Start P2 only if journal evidence cannot cover the gap.
- Watcher failure: restart with bounded exponential backoff while P1 preserves continuity; do not
  start inventory merely because the watcher restarted.
- Broker protocol, permission, or service failure: retain the checkpoint and cached catalog, keep
  P0 live when possible, and retry within a bounded policy. A root whose automatic persistent path
  is unavailable becomes `LiveOnly` or `更新受阻`; it is not silently scanned on every startup.
- Journal ID change, trimmed checkpoint, unsupported record, root/volume identity mismatch, or
  unprovable containment: start one explicit P2 baseline or recovery epoch while keeping P0 live.
- Root offline, disconnected, or inaccessible: retain its catalog and display availability status;
  do not reinterpret failure as deletion.
- Database transaction failure: roll back the entire delta, keep the intent retryable, and do not
  increment catalog revision.
- Huge directory rename or removal: process descendants through durable inventory pages; do not keep
  every row in memory or claim complete removals until the scope completes.
- Inventory failure or repeated source races: preserve the last trustworthy catalog and durable
  authority, then report `更新受阻` with one structured root error instead of starting a full scan.

Escalation order is:

```text
P0 live path or subtree reconciliation
-> P1 persistent journal catch-up
-> P2 pageable metadata-inventory recovery for a proven gap
-> blocked or LiveOnly state when complete automatic continuity cannot converge
```

The application must expose which level is in progress and why without leaking implementation
jargon into normal UI copy.

## Persistent catch-up and exceptional metadata recovery

ADR 0024 replaces ADR 0023's routine startup inventory. Every supported local NTFS root establishes
the live watcher first, then uses the constrained journal broker to validate and replay its missing
per-root USN range. Roots on one volume share a bounded physical read, but each root enrolls work and
advances independently. A failed or unavailable root cannot hold another root's P1 progress.

Checkpoint advancement follows durable enqueue. The application captures an exclusive journal end
boundary, validates volume, journal, root generation, broker protocol, root-set containment, and
catalog authority, durably enrolls the bounded normalized plans, then advances only that root's
checkpoint. A crash before advancement rereads an idempotent range. A raw journal reason never
authorizes catalog removal; every candidate passes the existing final-state reconciler.

The broker is installed and updated with explicit administrative consent, but Ame remains an
ordinary-user process and shows no UAC during normal startup. The broker reads existing journal
metadata only. It never reads media, opens the catalog, changes the journal, modifies source files,
hydrates placeholders, accepts arbitrary volume access, or returns records outside a caller-
authorized configured root. Versioned bounded IPC, caller-token and root verification, pipe DACL,
service SID, minimum proven privileges, cancellation, and root-external disclosure tests are release
gates.

P2 metadata inventory is authorized only for a one-time migration or first-authority baseline, or a
proven gap such as journal recreation or trimming, root/volume identity mismatch, reconstruction or
containment ambiguity, incompatible protocol or record version, or simultaneous watcher and journal
coverage loss. The inventory:

- records normalized path, entry kind, size, modification evidence, Windows file identity when
  available, and placeholder attributes;
- never reads media bytes, decodes images, generates previews, follows an untrusted reparse directory
  outside the root, or hydrates a cloud placeholder;
- stages derived evidence in application storage and compares it with the published catalog in
  bounded pages;
- yields to P0 and P1 at every page boundary;
- may publish additions and modifications early only after path final-state revalidation;
- publishes an absence only after the complete owning scope and closing journal boundary succeed;
- accepts and immediately publishes independent P0 live work throughout the run; and
- fails closed without mass removal when partial, cancelled, raced, unavailable, or incomplete.

A continuous no-change checkpoint creates no inventory run and enumerates exactly zero source-root
entries. A permanently unsupported root becomes `LiveOnly` or is refreshed explicitly; routine O(N)
startup inventory is not a compatibility fallback.

## Explicit refresh and recovery triggers

Ame does not schedule a periodic full-root consistency audit. A fixed seven-day interval adds
unbounded work without supplying evidence that anything changed. Continuous freshness instead uses
the P0 live watcher, P1 persistent journal, durable queue, and P2 inventory only when a specific
baseline or continuity-gap authority exists.

A complete root scan may start only when one of these authorities exists:

- a root is imported for the first time;
- the user explicitly selects `更新图库` for that root;
- a foreground full scan created by first import or explicit refresh is resumed from its durable
  checkpoint.

Normal file events, normal process restart, elapsed time, retry, watcher interruption, overflow,
an empty journal range, metadata-inventory size, and automatic recovery failure do not authorize a
complete scan. Work that exceeds the 4,096-entry or 128-path batch ceiling continues through bounded
P1 or P2 pages. Legacy direct-desktop `StartupCatchUp`, `consistency_audit`, prerelease
`authoritative_recovery` full-scan checkpoints, and ADR 0023 startup epochs are migration input only;
they cannot bypass the ADR 0024 broker, checkpoint, lane, or baseline contract. The last trustworthy
catalog remains visible while automatic continuity runs.

## Acceptance evidence

R2c is not complete until all applicable evidence exists. Its preview evidence is limited to
retain-or-invalidate behavior caused by automatic source changes; cache capacity, reclamation,
manual cleanup, storage relocation, and restart reconciliation remain R2b-owned contracts.

- create, modify, same-volume rename/move, same-path replacement, and removal update the gallery
  incrementally;
- the same controlled changes produce deterministic retain, invalidate, or remove semantics for
  derived evidence without keying any future smart-album result to an absolute path;
- normal single-file changes do not trigger a complete root scan;
- process start, watcher restart, overflow, retry, oversized inventory, and automatic recovery
  failure do not trigger a complete root scan;
- prerelease automatic full-scan checkpoints are retired into the one-time ADR 0024 migration
  baseline when required and cannot re-enter the production scanner;
- duplicate, reordered, incomplete, and late events converge on correct final filesystem state;
- related changes publish atomically at one catalog revision;
- a database failure or cancellation preserves the last trustworthy catalog;
- queued work survives a controlled process interruption without duplicate publication;
- a watcher overflow or failure marks live observation degraded and recovers from the continuous
  journal range; P2 starts only when that range cannot prove coverage;
- an offline or disconnected root retains its last catalog and does not publish mass removals;
- controlled content edits, same-path replacement, identity-proven rename or move, temporary
  unavailability, and authoritative removal produce the documented retain, atomic dimensions
  replacement, preview invalidation, or removal eligibility without a transient layout change;
- OneDrive and other recall placeholders are not hydrated by observation, metadata inventory, or
  authoritative recovery;
- the production Flutter gallery refreshes through bounded contracts and preserves stable identity
  and scroll position where the owning query remains valid;
- source removal, application shutdown, pause, retry, and cancellation do not hang the desktop app;
- every schema migration, adapter contract test, application test, Flutter state/accessibility test,
  and Windows integration scenario passes;
- Rust format, Clippy with warnings denied, Rust tests, generated bridge checks, Flutter analysis,
  Flutter tests, Windows Debug/Release build, and `git diff --check` pass serially;
- controlled fixtures and authorized real-root samples prove source bytes and entries are unchanged;
- closed-app create, modify, delete, rename, and move changes are covered by brokered journal replay
  without source-root enumeration on the continuous path;
- a no-change normal startup creates no inventory and enumerates exactly zero source-root entries;
- a P0 live change reaches the visible catalog at P95 no greater than one second even while P1 or P2
  is active, and a closed-process single change reaches it at P95 no greater than two seconds after
  normal runtime readiness;
- journal reset, trimming, broker failure, root replacement, and migration baseline use explicit P2
  recovery without gating independent P0 publication or publishing partial absence;
- broker caller identity, root containment, pipe access, protocol compatibility, installer lifecycle,
  portable `LiveOnly` degradation, root-external nondisclosure, and no-journal-mutation gates pass;
- cached catalog content remains immediately usable while startup continuity runs;
- development diagnostics expose active phase, elapsed time, bounded counts, and issue code;
- remaining filesystem limitations and measured performance are recorded honestly.

## Explicit exclusions and anti-drift constraints

- Do not implement R3 fingerprinting, R5 classification, or R6 similarity to avoid finishing
  freshness.
- Do not build a second asset-identity or metadata pipeline for watcher events.
- Do not attach future classification or smart-album membership to a path or make Flutter infer
  retain-or-invalidate policy from a filesystem event.
- Do not accept platform notifications as authoritative state or assume they are ordered and unique.
- Do not full-scan the approximately 259 GB library in response to every change.
- Do not place the watcher, queue, inventory, or SQLite policy in Flutter.
- Do not open a volume or request elevation from the desktop process. UAC is limited to explicit
  broker installer, repair, update, or removal operations.
- Do not let the broker read media, mutate source files, configure the journal, open the catalog,
  accept arbitrary volumes or roots, or return root-external records.
- Do not create metadata inventory on an ordinary startup with a continuous journal checkpoint.
- Do not treat a persisted priority value as sufficient; P0 must retain real reserved execution
  capacity and must preempt P1/P2 at bounded boundaries.
- Do not let one root's P1 or P2 failure block another root on the same volume.
- Do not let an automatic synchronization or recovery path create a full-scan request.
- Do not add a synchronization, task, timeline, or duplicate sidebar destination.
- Do not mutate, normalize, hydrate, move, or delete source files.
- Do not expose a production control before its complete application use case, failure state, and
  tests are connected.
- Do not mark a slice complete because events print to logs, a fixture works, compilation passes, or
  a screenshot looks correct.
- After compaction or handoff, recover recent original conversation, inspect the live implementation
  and ADRs, and compare actual verification before continuing from this section.
