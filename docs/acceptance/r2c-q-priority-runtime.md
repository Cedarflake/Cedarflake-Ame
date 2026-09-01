# R2c-Q priority runtime implementation checkpoint

Status: implementation checkpoint; not accepted

Date: 2026-08-31

Owning decision: ADR 0024

Active acceptance slice: R2c-O constrained broker and installer foundation

## Scope

This checkpoint implements change-priority runtime, bounded journal and recovery scheduling, the
one-time existing-root baseline, and independent live and continuity product state for Windows 11
x64. The current catalog schema is v29; it preserves the v24 journal payload, lineage, checkpoint,
and enqueue-before-advance invariants while extending the v25/v26 priority and recovery contracts.
It does not authorize a real library, source mutation, placeholder hydration, elevated Service
Control Manager operation, real FSCTL acceptance, signing, installation, or publication.

## Persistence and migration

- Catalog schema v25 creates one durable lane row for every queue row. Schema v26 adds persistent
  candidate ownership and the first recovery-execution frontier. The v25-to-v26 migration validates
  the complete v25 lane, authority, and baseline contracts before creating the new objects. Terminal
  v25 runs remain terminal, and a pristine active run can acquire its first frontier under v26; an
  already-advanced active run without durable frontier evidence fails closed.
- Schema v27 adds the durable metadata-inventory spool in application storage. The v26-to-v27
  migration creates its exact marker, tables, indexes, triggers, and foreign keys, then validates the
  complete current cross-table shape before commit. The spool is bound to the exact run, root
  generation, scope, directory identity, and unretired recovery authority. A partial v27 schema or a
  detached, malformed, or contradictory spool fails closed on reopen.
- Schema v28 adds the immutable canonical-root identity scheme and complete volume-plus-128-bit
  value to the spool and advances its contract marker. Candidate drain, absence publication,
  completion, and authority retirement must reproduce that proof. The v27-to-v28 migration never
  fabricates it: active opening, inventory, replay, and absence work has its unproven derived
  candidates, owners, inventory, and spool retired transactionally and is reset for bounded
  recapture under the existing allowlisted authority. Completed or retired v27 history is marked
  `RecoveryRequired`, not accepted as `Current`. Exact phase-table, rollback, reopen, and
  idempotence fixtures cover this transition.
- Schema v29 promotes a trustworthy root proof into an active root-generation publication contract.
  Foreground scans persist their opening proof and publish it atomically with the catalog; P2
  completion atomically verifies or establishes the same proof before `Current`; root replacement
  and generation retirement delete it. The v28-to-v29 migration accepts only mutually consistent
  full spool or V3 checkpoint evidence, does not upgrade V2, and rolls back conflicting proofs or a
  partial schema. An unproven active root keeps the last trustworthy catalog and becomes
  `RecoveryRequired` or `LiveOnly` rather than deriving authority from the current configured path.
- The sixth remediation removes the public proofless metadata-inventory entrypoint and compiles the
  legacy local inventory adapter only for tests. Production inventory now has one authority-bound
  path. P0/P1 workers lease real work before source access, load the matching v29 proof, and only
  then construct a private-field, non-cloneable `PublicationGuardedFileDiscovery`; authoritative
  requests cannot accept ordinary discovery. Empty queues and missing proof open no configured-root
  handle. A proof-A/root-B replacement is rejected from the pinned descendant metadata identity
  before the enumeration-root handle opens, with zero entry or directory inspection and a durable
  retry. No-proof/no-authority inventory performs zero source opens and leaves queue metrics,
  revision, run, staging, spool, candidate-owner, absence, and completion state unchanged.
- The seventh remediation removes `Deref<FileDiscovery>` from the publication capability. Its
  application-facing surface now consists only of explicit value operations, a borrow-bound opaque
  authoritative cursor, and an owned opaque metadata-inventory cursor that privately duplicates and
  retains the complete handle guard until its raw directory cursor is dropped. Neither cursor
  exposes a raw file, directory iterator, discovery reference, clone, conversion, or `into_inner`.
  The authoritative request is constructed only inside a private application coordinator, and its
  sibling-module entry requires the guard independently from ordinary request context. A stable
  compile-time negative trait assertion rejects any future `Deref<Target = FileDiscovery>` and
  positive cursor type assertions exercise the real Rust API boundary.
- Exact origin mapping is `live_notification` to P0 live, `startup_catch_up` to P1 journal, and
  metadata inventory, consistency audit, and explicit refresh to P2 recovery. Insert and update
  guards keep that mapping exact, while lane-specific eligibility indexing and leasing make it
  operational rather than a sort key.
- The immediate v24-to-v25 migration validates the complete v24 persistent-journal contract before
  mutation, deterministically classifies existing queue rows, creates no implicit recovery
  authority, and validates the complete current shape before commit. Current-catalog validation
  rejects weakened lane constraints, missing lane rows, malformed authority, and partial baseline
  lifecycle objects.
- Recovery authority is durable, immutable, root-generation bound, and accepted only for matching
  P2 queue work. Readiness and leasing join the persisted, unretired authority; a worker cannot mint
  authority from an ordinary audit, startup, elapsed time, empty journal, queue pressure, slow work,
  or `LiveOnly`. Its reason domain is limited to the ADR 0024 baseline and gap allowlist.
- Every recovery run persistently owns each candidate it publishes. Queue insertion, owner
  insertion, and inventory cursor advancement are one transaction. Replayed pages are idempotent;
  higher-lane coalescing records a safe superseding disposition; exhausted retry remains unresolved
  and blocks completion. Recovery authority cannot retire until every owned candidate has passed
  final filesystem revalidation and is either applied or safely superseded.
- The v28 source capture keeps one live handle-owned Windows directory cursor for the active
  directory and consumes at most 128 real source entries per raw batch. It checks cancellation
  between entries and yields at every raw
  batch. Entries from an incomplete directory remain provisional: process reconstruction resets
  that one directory and ignores its rows for candidate and absence authority. Only source
  exhaustion plus closing directory-type, no-follow/reparse, and matching Windows file-identity
  revalidation atomically marks it complete.
- Completed directories and their entries remain in the application-storage spool across output-
  page reopen and process reconstruction. Stable output is exposed in 4,095-entry pages only after
  directory completion, so a completed parent is not read again while nested children continue.
  One mid-directory process loss reads at most that incomplete directory twice. Repeated crashes at
  the same point may repeat it because this is not a persistent Windows directory cursor, but
  provisional data cannot create false freshness or partial absence. No spool or sidecar is written
  in the source tree, and completion, cancellation, replacement, and bounded cleanup retire derived
  application-storage rows.
- On Windows 11 x64, descendant enumeration opens every component relative to the live pinned root
  through the ADR 0024 `NtCreateFile` adapter and rejects ancestor or terminal reparse traversal.
  Each component uses the live parent directory's `FileCaseSensitiveInfo`; case folding is enabled
  only for an insensitive parent, and final containment matches a live root identity instead of
  lowercased strings. File-content access begins with an attribute-only root-relative handle;
  offline, recall, or reparse evidence performs zero access upgrades. After a no-delete rebind of
  the configured root, a second root-relative terminal-file `NtCreateFile` alone requests
  `FILE_READ_DATA | FILE_OPEN_NO_RECALL` and omits delete sharing. Its complete ID, volume,
  attributes, non-reparse state, and live root-identity containment must match the metadata proof
  before reading. Both calls use null EA buffers with zero EA length. Raw volume/device namespaces
  are rejected before any OS open. Ancestor junction replacement cannot substitute outside content,
  and moving the object outside the root has zero authorized content reads.
- Every P0 live and P1 journal path, paired rename, root/subtree reconciliation, resumed P2 source
  or candidate boundary, absence authorization/page, completion, and finalizer is required to bind
  the active generation to the persisted v29 proof before inspection and before the catalog
  transaction. The production guard normalizes the configured path to long-DOS descendant names,
  then opens the actual local DOS volume root (`C:\` on the C drive) through the Win32 adapter with
  `FILE_TRAVERSE | FILE_READ_ATTRIBUTES | SYNCHRONIZE`. Each ancestor and root descendant is opened
  relative to its already pinned parent through the native adapter with pure `FILE_TRAVERSE`.
  Both use backup/open-reparse semantics and read/write sharing without delete sharing. A separate
  root-relative `FILE_READ_ATTRIBUTES | SYNCHRONIZE` metadata handle
  validates each component; directory guards never request no-recall, and final paths or short-name
  expansion never become authority. The complete RAII chain remains held through the actual SQLite
  commit or rollback. Focused production-scheduler regressions now prove this window for P0 root,
  P0 subtree, and bounded authoritative P2 with an existing v29 proof. Ancestor/root rename or
  replacement, missing components, and insufficient minimal ancestor access fail closed without
  catalog removal, revision/completion publication, checkpoint `Current`, or authority retirement.
- Each output page uses one ordered index range with a `LIMIT page_size + 1` sentinel. The sentinel
  proves final-page state without the previous full remaining-row count and anti-join, so database
  work is proportional to the bounded page plus an index probe rather than all unstaged rows.
- The existing-root baseline persists opening identity, bounded inventory authority, replay and
  absence phases, closing boundary, and atomic completion. Completion establishes the supported
  checkpoint and current continuity state while retiring recovery authority in the same
  transaction. Partial, failed, cancelled, crashed, or identity-drifted attempts retain the last
  trustworthy catalog and resumable state; they do not publish mass absence.

## Production control flow

- Production starts the live observer and performs the first P0 poll before opening or querying the
  retained journal session. An absent broker, protocol mismatch, unsupported root, or portable
  process produces explicit `LiveOnly` with no automatic inventory.
- P0 owns reserved low-lane admission headroom, lane-specific leasing, a dedicated bounded worker,
  final-state filesystem revalidation, and SQLite catalog publication. P0 root and subtree work does
  not share P2's worker slot. Every coordinator poll services P0 before P1 or P2, so active, full,
  or retrying recovery cannot consume all admission, lease, or publication capacity.
- SQLite mutation uses one lane-aware admission coordinator. Once the current transaction ends, a
  waiting P0 writer is admitted ahead of newly arriving P1 or P2 work. The permit covers only the
  SQLite transaction; filesystem inspection, broker connect/read, hashing, signature verification,
  and other slow I/O occur outside it.
- Each production runtime epoch creates one validated SQLite session. Migration and full schema plus
  cross-table validation happen once outside the runtime mutex and priority permits. Later poll and
  worker connections perform only constant-size canonical-path, Windows file-identity, WAL,
  application-ID, user-version, and schema-cookie checks. File replacement or schema-cookie drift
  fails closed and triggers full revalidation outside the mutex; ordinary data-version changes do
  not. The priority permit covers only `BEGIN IMMEDIATE` through commit or rollback, never open,
  PRAGMA configuration, migration, validation, filesystem, broker, hash, or signature work.
- A bounded P0 root or subtree that proves watcher evidence cannot be reconstructed is promoted in
  one transaction: an independent pending P2 control row and `WatcherUncoveredGap` authority are
  created, durable lineage is transferred, and the original P0 row is superseded without retaining
  its lease or worker. Injected failure rolls the complete transition back; replay is idempotent. If
  lower-lane admission is full, P0 remains retryable and no P2 state is created. Ordinary backlog
  size never performs this promotion.
- P1 and P2 have distinct worker ownership. P1 requests at most 64 records per broker page and
  performs at most one bounded physical journal page for a shared volume group before yielding;
  roots advance independently and rotate fairly. Typed reset, trim, reconstruction, containment,
  and broker-after-current failures are persisted per root in the same transaction as
  `RecoveryRequired`, P2 control, and authority without checkpoint advancement. Cancellation,
  transient, non-recoverable, and `LiveOnly` outcomes do not silently create P2, and one failed root
  does not discard a healthy sibling's result.
- P2 can start only from persisted allowlisted authority. Its production candidate drain is bounded,
  retains ownership across backpressure, crash, and retry, and rotates roots after every 128-entry
  raw source batch as well as every output page. P1 checks P0 ready/active state before opening its
  next bounded page; P2 checks both P0 and P1. A bounded page already in progress may finish, but a
  lower lane cannot unconditionally open another page while higher work is ready or active.
- The default unresolved ceiling is 4,096: steady P2 is capped at 3,072, with 512 rows reserved for
  P1 and 512 for P0. A migrated v26 queue may temporarily carry the former 3,584-row P2 occupancy as
  capacity debt. Rows that predate candidate ownership use a persistent bounded legacy-debt path:
  path work completes without inventing an owner, while repeated non-path controls compact in one
  transaction to one blocked root `FreshnessUnknown` control with merged evidence. Superseded rows
  keep their bounded lineage, point to the unresolved survivor, and are protected from terminal
  cleanup, avoiding both evidence loss and the per-change lineage cap. Without a pre-existing
  matching allowlisted authority that survivor cannot
  enumerate, publish absence, advance a checkpoint, or become `Current`. While P1 waits, exactly one
  bounded P2 debt page may release a slot; new P2 enumeration, refill, control, and finalizer pages
  remain blocked. P0 still owns its reserve, the next admission is P1, and P2 then resumes per-root
  rotation without lost work or starvation.
- Legacy readiness selects zero-owner non-path work only when an unblocked survivor exists or more
  than one row still needs compaction. The last survivor, including an input with exactly one row,
  runs once and is durably exhausted as `legacy_recovery_authority_missing`; it cannot repeatedly
  start a worker after reopen. Projection is blocked `RecoveryRequired` with
  `recovery_blocked=true` and an actionable update-library control. Explicit refresh or later
  legitimate authority supersedes it; no authority, enumeration, absence, checkpoint, or `Current`
  is fabricated.
- During an existing-root baseline, the watcher and P0 remain live, the opening boundary brackets
  inventory, the closing range is replayed through P1, and absence remains withheld until complete
  coverage. A healthy sibling continues P0 or journal progress while another root is unavailable or
  recovering.
- A watcher-gap promotion captures the complete root, volume, journal, and opening `NextUsn`
  identity and atomically changes root/checkpoint state to `RecoveryRequired`. After inventory,
  production captures the closing `NextUsn`, persists the replay window, and drives the exact
  `[opening, closing)` interval through P1 under the same recovery authority. Identity drift,
  cancellation, crash, or non-terminal replay withholds absence, checkpoint replacement,
  `Current`, and authority retirement. If the opening query is temporarily unavailable, the
  pending typed failure remains durable and can be completed only by a later matching fresh query.
- The runtime registry mutex protects only short ownership and epoch transitions. Broker connect,
  filesystem work, SQLite operations, hashing, signature verification, and pipe waits occur outside
  it. One public stop returns success only after P0, P1, and P2 workers are cancelled and joined,
  the core observer is stopped, the admitted journal session is closed exactly once, and the epoch
  is removed from the registry. The first caller stores one absolute two-second deadline in the
  registry epoch; Starting, Polling, Stopping, Draining, concurrent stop, and late owner paths reuse
  that exact value across every lane, all roots and native watchers, core shutdown, and journal
  close. A late runtime is installed as complete `Draining` ownership before the already exhausted
  deadline is applied; no phase receives a new two seconds. The wait
  uses channel and condition-variable completion without holding the registry mutex. Journal close
  runs as one retained owned task. Timeout keeps the same receiver, join handle, and session in
  `Draining`; retry or reaper joins that task exactly once and cannot call close again or admit a new
  epoch first. Flutter invalidates the current poll generation and invokes native stop immediately,
  without awaiting the active poll. A never-ending poll cannot extend the public call past its
  absolute deadline: the Polling entry immediately becomes `Draining` with the same complete runtime
  `Arc` and first deadline even while the operation mutex is held. Its late result is discarded and
  cleanup resumes on that owner; it cannot publish stale state or clear a restarted epoch. Concurrent
  callers share only the stop in flight, failure can retry, success can restart, and dispose blocks
  subsequent starts while sharing the active stop. Constructor, poll/operation, and drain panics are
  contained inside this registry boundary. Matching runtime-less constructor state is cleared and
  notified without touching another epoch. Poll panic retains or creates one `Draining` deadline;
  drain acquires the runtime mutex outside `catch_unwind`, then rechecks the exact epoch, `Arc`, and
  deadline before cleanup. P0/P1/P2, observer, journal close outcome, and every join handle remain
  owned until the corresponding idempotent step completes. One-shot panic can retry the same owner;
  persistent panic remains `Draining` without mutex poisoning, deadline refresh, duplicate stop,
  close, join, detach, `Empty`, or ABA admission. The stable
  `library_synchronization_owner_panicked` error never includes the panic payload.
- The ninth remediation makes desktop startup an explicit lifecycle rather than an optimistic poll
  loop. Flutter reports `started`, `failed`, or `skipped`, shares one in-flight start per generation,
  and retries the native start call only through the finite 100 ms, 500 ms, and 2 s backoff sequence.
  It creates no poll timer before a current-generation start succeeds. Stop and dispose invalidate
  the generation and cancel any retry timer; a native lifecycle that may have been entered is still
  stopped even when logical start failed. Native `library_synchronization_not_started` converges
  safely, while every other stop failure preserves the running ownership claim for retry. A late
  old start result cannot publish, create a timer, clear, or overwrite a replacement epoch.
  Native panic reconciliation now distinguishes a retained matching owner from an epoch proven
  drained by the monotonic `retired_through_epoch` watermark. A deterministic test-only barrier
  forces stop to publish `Empty` and install a replacement epoch after the old operation drops its
  runtime guard but before it reconciles. The old caller still receives the sanitized stable panic
  error and cannot cancel, close, replace, or clear the new runtime. Same-epoch impossible states,
  including runtime-less `Stopping`, remain fail closed and cannot adopt an external runtime.
- The tenth remediation linearizes lifecycle requests before asynchronous bridge scheduling. Short
  synchronous bridge calls allocate checked process-global owner tickets and cancellation fences.
  Stop advances a cancel-through watermark even for `Empty`, records the first absolute two-second
  deadline at the public Dart boundary, and immediately changes a covered `Starting`, `Ready`, or
  `Polling` owner to `Stopping` or `Draining` while preserving its cancellation flag and complete
  runtime `Arc`. Its asynchronous task accepts only that admitted fence and drains under the stored
  deadline; delayed bridge scheduling cannot create another allowance. Starts at or below the
  watermark fail before epoch allocation or construction, newer tickets can begin only after drain,
  and poll requires the exact active owner ticket. Same-ticket bounded retry, different-ticket
  rejection, old-poll replacement isolation, checked overflow, poisoned state, and impossible owner
  transitions remain fail closed.
- Flutter reserves one owner ticket for a shared start operation and reuses it only across the
  explicit `catalog_database_busy` and `catalog_database_locked` retry allowlist. Non-transient start
  failures execute once. Trusted cached state reaches `runApp` before a process-lifetime owner
  schedules background start with synchronous and asynchronous failure handling already attached.
  Shutdown shares owner disposal and issues the synchronous cancellation fence without awaiting a
  hung start; generation checks discard every late result and preserve the stopped or replacement
  UI.
- The eleventh remediation makes each successfully reserved global cancellation fence responsible
  for its matching asynchronous drain. Local controller state cannot suppress that call after the
  process-global registry has already moved an owner to `Stopping` or `Draining`; an unstarted second
  controller, hot-restart replacement, or close-before-start therefore cannot strand the prior
  owner. Native `Empty` and `library_synchronization_not_started` remain idempotent convergence.
  The process-lifetime owner shares an in-flight or successful close, clears a failed close future
  for same-owner retry, and keeps its permanent closing state so startup can never resume.
- The twelfth remediation makes native lifecycle tokens kind-distinct. The bridge retains
  `u64`/Dart `BigInt`, while Rust decodes the low-bit start-ticket or stop-fence tag into
  `RuntimeStartTicket` or `RuntimeStopFence` before range or watermark validation. All ordering and
  cancel-through semantics use one checked monotonic `LifecycleOrdinal`; same-start retry,
  concurrent or late stop, and an old admitted fence against a newer epoch retain their previous
  order. The tag rejects an unchanged start raw value at the stop boundary, but it proves only value
  shape, not issuance: changing the low bit produces a stop-shaped value with the same ordinal.
  Process-local allocation stops at `u64::MAX >> 1` and fails closed before tagged encoding can
  overflow.
- The thirteenth remediation proves exact stop-fence issuance with a process-local, hard-capped
  64-entry ledger. Reservation records the exact pending typed fence and covered owner under the
  registry lock before it advances the watermark or returns. Pending entries are never evicted; a
  full ledger with no reclaimable consumed history returns
  `library_synchronization_lifecycle_fence_capacity_exhausted` without changing the ordinal,
  watermark, cancellation signal, owner, deadline, or runtime identity. Stop marks exact membership
  consumed, concurrent and repeated callers share one drain, and consumed history is reclaimed only
  by later reservation pressure when it no longer covers the active owner. Empty-registry fences
  remain delayed idempotent no-ops for newer owners, timed-out or panicked drains remain retryable,
  and a later real fence can finish the same owner without refreshing its first deadline. The
  bounded ledger is not persisted and requires no migration.
- Rust boundary coverage pins zero and unissued start tickets, zero and unadmitted stop fences,
  epoch exhaustion returning to `Empty` without construction, and an old fence remaining unable to
  drain a newer epoch.
- Rust bridge and Flutter models project live status independently from
  `BaselineRequired`, `CatchingUp`, `Current`, `RecoveryRequired`, `LiveOnly`, and `Unavailable`.
  Compact Chinese status explains `LiveOnly` as coverage only while Ame is open and does not expose
  service, journal, queue, privilege, or USN terms. A revalidated location publishes with preview
  pending; preview generation is not a visibility gate.

## Controlled performance and startup evidence

The production performance fixture uses disposable Windows roots and an actual SQLite catalog. It
prepares 2,048 real PNG P1 candidates through the active broker path, a cold authority-backed
10,000-entry P2 recovery using the production 4,095-entry output page, and a concurrent low-lane
SQLite writer. The first P0 sample occurs after only 128 real P2 source entries have been consumed,
before the first P2 output page exists. It records 25 watcher-event-to-visible samples and requires
both lanes to be active and make real progress in all 25 samples.

In the final run, P1 was active and progressed in 25/25 samples and completed all 2,048 candidates;
P2 was active and consumed real source entries in 25/25 samples. P2 source reads advanced from 128
to 3,200 to 10,000 and then published 4,095 staged entries. The competing low-lane writer advanced
from 1 to 391,335 transactions. Queue P95 was 31 ms, worker-start P95 was 30 ms, visible-query P95
was 0 ms, and the current third-round end-to-end timing was P50 136 ms, P95 179 ms, maximum 251 ms,
with zero samples strictly above one second. The current focused rerun passed in 98.16 seconds. The
earlier P50 61 ms, P95 70 ms, maximum 78 ms, 70.19-second run is retained only as a historical
pre-remediation snapshot.

The production coordinator fixture uses one real controlled directory with 4,096 files and never
calls staging, publication, frontier, authorization, or completion helpers in place of production
work. Production poll/open/worker consumes the 4,095-entry source page, reaches 4,096 through the
steady 3,072-row P2 backpressure and drain/refill path, terminalizes every owner, and drives closing
replay, absence, control/run completion, checkpoint `Current`, and authority retirement through the
same coordinator. The current eighth-remediation ordinary-user rerun passed in 73.61 seconds; the
earlier 33.63-second run remains historical evidence.

The no-change fixture performs 100 consecutive production start and shutdown cycles against a
selected disposable root whose checkpoint equals every captured `NextUsn`. It records 100 journal
queries, 100 session closes, zero physical journal reads, zero metadata inventory runs, and zero
selected-root entry enumerations. The final-diff focused rerun passed this exact contract in 5.07
seconds.

These fixtures use controlled disposable data and do not fabricate or access retained real-library
content. They prove the in-process scheduling and catalog-publication contracts; they are not a
substitute for an elevated installed-service run or separately authorized retained-library
acceptance.

## Verification recorded at this checkpoint

- Focused remediation fixtures pass for candidate publication rollback at queue, ownership, and
  cursor boundaries; idempotent replay; higher-lane supersession; retry exhaustion; more than 4,095
  candidates; P0 admission during active/full/retrying P2; multi-root rotation; watcher-gap opening,
  closing, replay, drift, cancellation, and crash boundaries; typed P1 failure isolation; runtime
  stop under delayed P0/P1/P2 and blocked broker/filesystem/SQLite work; v28 phase-table migration,
  rollback, exact-shape, idempotence, and reopen validation; exactly 3,584 zero-owner legacy P2 rows,
  non-path compaction without authority, multi-root debt rotation, and the retained 64-owner positive
  path; ancestor-junction replacement during enumeration and candidate drain; offline zero-content-
  upgrade and move-outside-root zero-read rejection; case-sensitive sibling selection; configured-
  root absence/finalizer rebinding and held-pin rename rejection; indexed sentinel spool paging;
  runtime-session identity and
  schema-cookie invalidation; and bounded durable-spool enumeration.
- Schema v29 migration fixtures pass for mutually consistent spool/V3 proof adoption, V2
  non-upgrade, conflicting multi-proof rollback, and partial-schema rollback. Runtime fixtures pass
  for atomic foreground proof establishment, resume rejection after namespace replacement, P2 proof
  establishment and conflict rollback, generation retirement, and path-scoped P0/P1 guard ownership
  through the real SQLite delta commit. Three additional production-poll regressions cover P0 root,
  P0 subtree, and bounded authoritative P2. A root-ID-keyed one-shot hook runs after real source
  enumeration and catalog mutation but immediately before commit; root or ancestor rename fails with
  Win32 32, the injected structured error rolls the transaction back to `retry_wait`, and rename
  succeeds after worker shutdown. Catalog revision, completion/current state, active publication
  proof, and P2 authority retirement remain unchanged.
- The Win32 traverse-helper fixture records pure `FILE_TRAVERSE`, read/write sharing without delete,
  and backup/open-reparse semantics for each temporary owned descendant prefix; rename and delete fail with
  Win32 sharing violation while held and succeed after drop. An ordinary-user disposable root below
  Public Documents supplies positive production volume-to-root evidence: the `C:\` volume handle
  accepts `FILE_TRAVERSE | FILE_READ_ATTRIBUTES | SYNCHRONIZE`, while `C:\Users`,
  `C:\Users\Public`, and `C:\Users\Public\Documents` accept root-relative no-follow pure-traverse
  guards; a held root rename returns Win32 32,
  and rename succeeds after drop. The ordinary-user controlled fixture enters
  the case-sensitive branch (`controlled_case_sensitive_directory=true`) for both a root component
  and nested only-case-different component. Raw/device names are rejected before the
  production-connected configured-root wrapper, while terminal root-relative opens record a relative
  component and never a volume path. The workspace sandbox returns Win32 5 for one otherwise valid
  profile ancestor; that restricted result is environment-specific negative capability evidence,
  not a claim about Windows profile ACLs. The controlled capability-failure fixture separately
  proves zero catalog delta, revision/completion, `Current`, checkpoint, or authority publication
  and a `RecoveryRequired` projection.
- Stop fixtures pass for late Starting and Polling owners crossing the first absolute deadline,
  repeated stop preserving the exact `Instant`, immediate complete `Draining` ownership, restart
  rejection, blocked close across other drained lanes, retry/reaper joining the same close task
  once, close failure, panic, and disconnect. Flutter
  controller fixtures pass for a never-completing poll, late old completion after restart, concurrent
  stop, failure retry, start during stop, and dispose sharing stop while preventing future starts.
- Eight panic-ownership fixtures pass for constructor panic with and without a concurrent stop; poll
  panic before worker creation and after P0/P1/P2 creation across public-deadline races; one-shot
  panic at request-stop, P0, P1, P2, journal-close installation, and journal-close result boundaries;
  persistent panic; and a stale drainer that had completed its first registry check before another
  epoch appeared. They prove stable sanitized errors, condition-variable progress, the same epoch,
  runtime `Arc`, and first deadline, an unpoisoned mutex, retained join/close ownership, exactly-once
  journal close, retry completion, fail-closed persistence, and no premature restart or ABA.
- Legacy-debt fixtures pass for exact 1, 2, and 3,584 zero-owner non-path rows, crash/reopen
  idempotence, blocked recovery projection, and explicit-refresh convergence without fabricated
  authority or freshness.
- First-open P2 fixtures pass for replacement before source open, ancestor swap before source open,
  guard ownership through spool-init commit/rollback, and a legitimate no-proof baseline that first
  establishes a guarded spool identity. Proof mismatch performs zero enumeration and creates no
  spool, candidate, absence, catalog, revision, `Current`, or authority change.
- Capability fixtures compile the negative non-`Deref` assertion and both guarded cursor signatures.
  The Public Documents runtime fixture drops the original wrapper while an inventory cursor remains:
  root rename still fails with Win32 32, enumeration completes, and rename succeeds only after cursor
  finish drops the duplicated guard. The workspace-profile Win32 5 result remains only scoped
  negative environment evidence, not a blanket Windows capability claim.
- The P1 revision-rebase suite passes 3/3, including final revalidation after a concurrent catalog
  revision, bounded churn, and cancellation. The local registration failure fixture passes and
  proves a typed non-recoverable root cannot block a healthy sibling or fabricate P2. The production
  25-sample priority fixture passes with the measurements above.
- Seventh-remediation focused modules pass 41/41 incremental, 11/11 authoritative, 35/35 metadata
  inventory, 274/274 catalog, 62/62 migration, 47/47 production, 23/23 legacy, 34/34 local-file,
  and 36/36 scan-library tests with two intentional scan ignores. The 15/15 stop and 4/4 journal-
  close filters are green subsets of the production suite. The public-facade compile-fail doctest
  passes 1/1. The eighth-remediation complete serial production module passes 55/55 in 203.65
  seconds; exact ordinary-user reruns pass the 3,584-row legacy fixture in 0.89 seconds and the
  4,096-file fixture in 73.61 seconds. Their earlier workspace-sandbox failures were the expected
  `root_publication_namespace_guard_unsupported` capability boundary, not production regressions;
  neither fixture nor its assertion was weakened.
- Ninth-remediation red evidence first failed on the unmodified lifecycle: the Flutter focused run
  passed 20 tests and failed 2 because no second native start occurred and polling ownership was
  still enabled optimistically; the deterministic Rust interleaving failed 1/1 because the old
  caller returned `library_synchronization_owner_state_invalid` instead of the stable owner-panic
  error. After the fixes, the Flutter lifecycle file passes 26/26, the panic/stop filter passes 5/5,
  and the complete ordinary-user serial production module passes 57/57 in 147.66 seconds.
- Tenth-remediation red evidence first failed on the prior lifecycle: the Flutter controller file
  passed 26 tests and failed 1 because a non-transient native start was attempted four times instead
  of once; the deterministic pre-admission Rust interleaving failed 1/1 because stop observed
  `Empty` and the delayed start still entered its constructor. After the fixes, the controller file
  passes 28/28, the process-lifetime owner file passes 2/2, and the complete ordinary-user serial
  production module passes 62/62 in 241.94 seconds. Focused Rust coverage also passes delayed start
  admission, old-poll replacement isolation, same-ticket retry, checked ticket and fence overflow,
  and a delayed asynchronous stop task consuming the deadline captured by its synchronous fence.
- Eleventh-remediation red evidence first failed on the prior Dart lifecycle. The controller file
  passed 28 tests and failed 1 because an unstarted controller admitted a global stop fence but made
  zero native drain calls. The process-lifetime owner file passed 2 tests and failed 1 because its
  first failed close future remained identical to the second call. After the fixes those files pass
  29/29 and 3/3. The four new Rust lifecycle boundary tests were direct green against the unchanged
  native production implementation; the focused filter passes 4/4. The complete ordinary-user
  serial production module passes 66/66 in 281.70 seconds.
- Twelfth-remediation evidence passed the earlier start raw value unchanged after a later real stop.
  It proved kind decoding, but did not test a low-bit flip and therefore did not establish issuance
  provenance. Its historical 5/5 lifecycle-boundary and 67/67 ordinary-user production results are
  retained only as type-shape and regression evidence.
- Thirteenth-remediation red evidence presents `start_ticket.raw() | 1` after a later real fence has
  advanced the watermark and published `Draining`. The tagged implementation decoded and accepted
  that same-ordinal forgery, returned success, and closed the owner; the focused run failed 1 test
  with 861 filtered. The exact ledger now returns
  `library_synchronization_lifecycle_fence_invalid`, performs zero closes, retains `Draining`, and
  lets the real admitted fence drain exactly once. The production lifecycle-boundary filter passes
  6/6, the stop-fence filter passes 8/8, and the active-owner capacity-pressure retry fixture passes
  1/1. These regressions include raw zero/one, same-kind unissued values, atomic 64-entry exhaustion,
  consumed-history reclamation, same-fence concurrent/repeated use, delayed Empty no-op, checked
  overflow, and old-fence/new-owner isolation.
- The isolated release-profile large-v26 session fixture populated 1,024 queue, lineage, owner, and
  frontier rows, completed its one migration and full validation in 55.447 ms, then performed 100
  production-session reopens in 1.1784592 seconds with a 13.2528 ms maximum individual reopen and
  exactly one recorded full validation.
- Current explicit-file format and development- and release-profile
  `cargo check --all-targets --all-features` pass. Warnings-denied Clippy passes in both profiles.
  The eighth-remediation Daily recorded 839
  passed, zero failed, and 11 ignored library tests in 217.89 seconds. The ninth-remediation Daily
  recorded 841 passed, zero failed, and 11 ignored library tests, followed by 3/3 integration tests.
  The tenth-remediation Daily records 846 passed, zero failed, and 11 ignored library tests,
  followed by 3/3 broker integration tests.
- `quality_lint.ps1` exits zero with 149 files unchanged by formatting and no Dart analyzer issues.
  The eleventh-remediation Daily first attempt reached Rust compilation but Windows rejected
  metadata mapping with OS error 1455 at 34.54/37.80 GB committed memory. The same unmodified gate
  exits zero with one Cargo build job and one Rust test thread: 850 passed, zero failed, and 11
  ignored library tests in 553.63 seconds, followed by 3/3 broker integration tests in 2.49 seconds,
  all 309 Flutter unit and widget tests, controlled Windows scan 2/2, native Windows accessibility
  2/2, generated bridge hash compatibility, and tracked-diff whitespace validation.
- The current internal Windows x64 Release application builds in 241.0 seconds under the same
  single-build-job resource bound; the earlier 60.3-, 77.5-, and 125.2-second results remain
  historical. The full Windows Release
  orchestrator is not claimed because its required externally pre-signed Application bundle,
  pre-signed broker, and expected publisher were not supplied; an explicit absent-input invocation
  fails closed at signed-bundle admission with `The Ame application bundle was not found`. Elevated
  SCM, real brokered FSCTL, real-library, signing, source-immutability, and no-hydration evidence
  remain outside this unattended checkpoint.
- Twelfth-remediation explicit Rust formatting, development- and release-profile check, and
  warnings-denied Clippy pass. `quality_lint.ps1` passes with 149 files unchanged and no Dart
  analyzer issues. The unchanged Daily gate exits zero under one Cargo build job and one Rust test
  thread: 851 Rust library tests pass, none fail, and 11 remain intentionally ignored in 255.01
  seconds; broker integration passes 3/3 in 2.37 seconds; all 309 Flutter unit and widget tests,
  controlled Windows scan 2/2, native Windows accessibility 2/2, generated bridge compatibility,
  and tracked-diff whitespace validation pass. The internal Windows x64 Release application builds
  in 96.9 seconds. Formal `release_verify_windows.ps1` with explicit absent external application and
  broker paths fails closed before SCM or process startup with `The Ame application bundle was not
  found`; externally signed bundle verification remains open and is not claimed.
- Thirteenth-remediation explicit Rust formatting, development- and release-profile check, and
  warnings-denied Clippy pass. `quality_lint.ps1` passes with 149 files unchanged and no Dart
  analyzer issues. The complete ordinary-user serial production module passes 72/72 in 74.32
  seconds. The Daily gate exits zero with one Cargo build job and one Rust test thread: 856 Rust
  library tests pass, none fail, and 11 authorization or manual-performance tests remain ignored in
  190.63 seconds; broker integration passes 3/3 in 2.36 seconds; every Flutter test file, controlled
  Windows scan 2/2, native Windows accessibility 2/2, generated bridge compatibility, and
  tracked-diff whitespace validation pass. The internal Windows x64 Release application builds in
  89.9 seconds. Formal absent external application-bundle and broker inputs fail closed before SCM
  or process startup with `The Ame application bundle was not found`; externally signed bundle
  verification remains open and is not claimed.
- The fourteenth independent read-only re-audit reports zero Critical, High, Medium, or Low
  findings. Its fresh focused evidence is 6/6 Rust lifecycle-boundary tests, 8/8 stop-fence tests,
  1/1 active-owner capacity/retry test, 29/29 Flutter controller tests, 3/3 process-lifetime owner
  tests, and a clean `git diff --check`. This closes the R2c-Q implementation/audit checkpoint; it
  does not convert R2c-Q, R2c-P, R2c-O, or the accumulated R2c milestone into accepted work.

## Acceptance boundary

This document records an R2c-Q implementation checkpoint, not R2c-Q acceptance. R2c-O remains the
active acceptance slice because its authorization-bound elevated installed-service evidence is
still open. R2c-P also remains an implementation checkpoint. The complete Daily gate and the Windows
Release orchestrator must not be conflated: Daily, controlled Windows scan, native accessibility,
the internal Windows x64 Release build, and the fourteenth independent read-only re-audit pass,
while externally signed bundle verification, packaged release bridge smoke, a real installed-
service lifecycle, real brokered FSCTL evidence, and separately authorized retained-library
evidence are not claimed. R2c-R has begun only as a non-external controlled local reliability
checkpoint; that work is recorded separately and is not R2c-R acceptance.
