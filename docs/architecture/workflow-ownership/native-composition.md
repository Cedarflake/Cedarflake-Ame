# Native production composition

Status: binding scoped requirements under [ADR 0037](../0037-feature-module-dependency-boundaries.md).

Read this contract when changing a retained native factory or its resource lifetime. Exact admitted
source/symbol/target edges and regression owners live in
[`rust_boundary_policy.json`](../../../tool/dependency-boundaries/rust_boundary_policy.json).
Retained entries are technical composition boundaries; `activeIds` remain unfinished migration.
Both kinds reject new or stale dependencies. The fixed legacy inventory is never regenerated.

## Native tool entrypoints

The `file_operations` facade exposes the exact `WindowsFileOperations` adapter constructor for the
Windows Sandbox admission probe under [ADR 0042](../0042-windows-file-operations.md). It allocates
no native resources and grants no source-file authority. Calls use the `NativeFileOperations` port,
retain namespace guards through execution, and join the Shell STA before returning. The probe owns
only newly generated guest-local fixtures. Application callers cannot inherit this export's
concrete-adapter exception; their production factory requires its own scoped admission.

The classification facade retains three exact native exports for repository tools:
`CuratedClassificationVocabulary`, `LocalClassificationVocabulary` and
`SqliteClassificationFeatureCache`. The classification probe owns vocabulary/cache construction;
the cache-export tool owns its cache connection. These exports allocate no resource or process and
preserve existing public tool paths. Replacement changes those constructors and their port
implementations together, retaining vocabulary admission, bounded cache reads and caller retirement.
Their adapter contract tests remain the behavior owners. No neighboring export or facade function
inherits this admission; application consumers resolve these aliases to their native definitions.

`classification_worker` exposes native worker contracts; the worker and journal binaries delegate
to their existing adapter process entrypoints. The boundary loader reads every Cargo binary target,
including feature-gated targets, and distinguishes a binary's own `crate::` paths from the library's
public aliases. Native entrypoints cannot call back into application or bridge policy. The existing
production synchronization owner retains observation startup and shutdown; no separate public
observer wrapper selects another factory.

## Validated catalog session

`catalog_session.rs` is the application resource host for validated catalog sessions. Its registered
state/decision types and validation-flight methods retain the concrete, shared session proof.
Validation and refresh functions share one flight per catalog path, including its original failure,
bounded retry delay and invalidation result. Expensive validation runs outside the registry and
entry-transition locks; a late flight cannot replace a newer owner or an invalidated proof.

The registered catalog and read-executor factories select connections from that proof. Foreground
scan acquisition rechecks the current session identity before protecting it; ordinary refresh cannot
replace a session owned by an active scan. Idle recovery keeps its current-session and scan-exclusion
checks around acquisition and the supplied `ForegroundScanRecovery` operation. The callback borrows
only the recovery port and cannot access unrelated SQLite operations. Maintenance retains its separate
reservation and proof-retention contract in [catalog ownership](catalog-and-migrations.md).

`CatalogLaneSession` exposes owned lane connections from the retained validated proof. Live and
queue-drain workers retain the same shared session through their original thread lifetime and require
only the repository ports they use. Its SQLite binding delegates the existing lane-open operation;
database identity, schema proof and write admission remain in that operation. Session validation and
replacement stay with the resource host above, independently of worker cancellation and retirement.

`JournalCatalogSession` supplies the namespace reader from that same shared proof. The native binding
retains the session and opens each bounded identity lookup in the Journal lane. Journal workers keep
registration guards through capture; reader construction adds no validation flight or runtime owner.

`library_synchronization/production/poll_catalog` owns the retained poll connection and
`WorkerCatalogSession` binding. The latter retains the existing epoch/path check and shared proof;
poll checkout binds that proof before worker scheduling. Its cleanup factory captures the current
session before admission, then opens Recovery inside the worker after cancellation is checked.
Stale proof refresh stays with the process-session owner and preserves foreground-scan protection.

`PollCatalogOwner` admits one checkout, revalidates the native connection and compares proof identity
before reuse. Its exact registered checkout, retained-resource and install/session edges own native
resources only. Checkout drop returns the original connection after errors or unwind; explicit
retirement closes it outside the transition lock before publishing `Retired`. Replacement must keep
that ordering, path binding, WAL retention and worker-session lifetime.

SQL, schema/row validation, database identity and connection preparation remain in the adapter.
The host admission covers only the registered concrete session/catalog/executor references. Replacing
persistence substitutes those proof and connection resources together, preserving shared waiters,
foreground protection, lane selection and invalidation. It grants no business-use-case exemption and
does not expose SQLite or engine types through domain, ports or bridge contracts.

## Catalog read capabilities

`catalog_session::open_catalog_reader` exposes `CatalogReader`, retaining the same validated-session
factory and recovery lane. The owned reader outlives its input path and exposes only typed read
operations. Each operation returns Ame values after retiring its read connection; no caller-supplied
callback, SQL, connection or transaction crosses the port. SQLite protocol retry, deadlines and
session proof remain in `sqlite_catalog/read_retry.rs`. Replacing persistence implements this port
and changes the existing session factory together.

`application/classification_selection.rs` borrows `ClassificationSelectionReader` from the existing
Live-lane catalog binding. SQLite retains the annotation transaction's displayed-revision and
selection checks, and the item-state transaction's configuration, source and human-decision rules.
The read port returns Ame annotations and item states while preserving that connection's lifetime.

`load_catalog/production::source_state` selects the stateless `SourceStateProbe` after preview
reconciliation and recovery scheduling. `load_catalog/root_availability.rs` projects fresh root
evidence and messages without probing media content or changing catalog identity/counts. An
unavailable root does not prevent later roots from being inspected. Replacing this probe does not
move preview recovery, catalog publication or source-read authority.

## Storage resources and layout

`storage/production.rs` selects settings through `StorageSettingsRepository`, cache capabilities
through `PreviewCacheStorage`, and default locations through the existing OS directory provider.
The cache adapter owns versioned layout, format recognition, algorithm identity, read-only accounting,
namespace acquisition and root preparation. Accounting does not traverse nested directories.
`PreviewCacheDirectory` retains the original root/ancestor guards; an absent root grants no later
deletion authority. Cleanup traversal/deletion, recovery, budgets and cross-database activation remain
application policy. Default-path selection creates no directory or database.

Each settings call owns its original connection lifetime. Configuration validation, transition
admission, restart activation and cross-database pending-root retirement keep their existing order.
Replacing storage changes these factory selections and ports together, without moving those rules
or acquiring a second process-wide configuration owner.

`domain/path_display.rs` owns the pure Ame path-display projection shared by storage, catalog reads,
scan events and diagnostics. It has no filesystem or platform dependency and grants no access
authority; stored access paths remain intact. Its existing string contract needs no adapter port.

## Classification execution resources

`NativeClassificationRuntime` owns the background dispatcher, foreground search thread, their catalog
connections and the process-ownership token. `NativeExecutor` and its registered resource types own
the feature repositories and prepared matcher; both executors borrow the same image/text scheduler.
`open_features` selects
both repositories behind `SharedImageFeatures`; the port-based owner coordinates compatible reuse
and publication under [ADR 0040](../0040-durable-discovery-features.md). Replacing storage changes this
factory and its resource alias while preserving the two independent budgets and lifetimes.
Cache acquisition is lazy; constructing the runtime
does not start an encoder. The existing [residency contract](../0034-versioned-classification-contracts.md#worker-ownership-and-protocol)
owns per-worker demand loading, cancellation, idle retirement and replacement.
[ADR 0041](../0041-resource-aware-analysis-scheduling.md) owns shared encoder admission. Closing the
host cancels and joins foreground/background work, then retires both encoders before releasing process
ownership; unproven retirement prevents another runtime.

`production::recover_semantic_features` selects the derived-store recovery adapter after
`while_retired` acquires that same ownership token. The desktop lifecycle joins both executors before
maintenance and reopens them afterward. Replacing storage preserves healthy data, rejects unproven
file ownership and retains SQLite's locked-file behavior; recovery never acquires catalog write or
source-media mutation authority.

`classification_runtime/production::encoder_factory` selects the CPU or DirectML adapter from the
persisted preference and available hardware. Its device observer exposes only Ame-owned status.
`model_transition` replaces an incompatible known pipeline through the existing configuration
transaction before dispatcher startup. [ADR 0039](../0039-gpu-classification-execution.md) owns
device admission, precision, feature identities and installation migration.

`production::analysis_scheduler` selects the native encoder factory and capacity provider.
`NativeCapacity::capacity` reads Windows capacity, while its pressure input comes from existing
foreground signals. Domain policy chooses concurrency; the generic application scheduler owns leases
and worker residency. Replacing capacity measurement changes this provider without moving admission
policy into the platform adapter. `production::semantic_readers` supplies the existing guarded
fingerprint/image readers through their ports; the semantic use case retains publication authority.

`analyze_classification_content` and `verify_classification_source` are source-resource entrypoints:
they select the guarded image or exact reader and the existing catalog/background admission, then
invoke use cases through Ame ports. The [dispatch contract](../0036-durable-classification-dispatch.md#durable-execution-and-compatibility)
owns acquisition order and publication. Held source/namespace leases outlive publication or rollback;
text work uses the shared encoder scheduler without double-admitting image source reads.

Replacing the native selections preserves those lifetimes and the existing use-case contracts.
SQL, inference protocols and engine types remain outside domain/ports/bridge. Only the registered
construction, host-state and borrowed-resource edges are admitted; configuration, discovery and
installation receive no neighboring-file exemption.

## Classification discovery resources

`classification_runtime/production::vocabulary` selects the bundled managed vocabulary through
`ManagedClassificationVocabulary`. Its metadata remains Ame-owned catalog and naming-context values;
bounded concept reads retain the existing vocabulary contract. `CategoryDiscovery` accepts this
typed constructor and the expansion, merge and retirement repository ports. Its notification-scoped
reconciliation, sampling, reduction and completion state stays together in the existing owner.

Vocabulary/context loading remains lazy within the admitted pass. Changed-authority handling,
completion and explicit release retire those resources without admitting a new pass for the same
notification. The dispatcher releases maintenance resources and retires its executor before returning
a plan. Replacing the vocabulary selection or repository implementations preserves that lifecycle,
catalog revision, cancellation, bounded sampling and the existing prepared-plan publication rules.

## Classification installation resources

`classification_model_installation/production.rs` selects the installation repository, pinned model
package and OS model-storage directory. Configuration, desktop startup and installation recovery use
the same repository and model-package contracts. Installation paths and download intent generations
are Ame-owned values; a token acquires authority only through the repository's current-intent checks.
Persistence retains the immediate transaction that compares selection, completes the matching intent
and checks cancellation before commit. Merely constructing either adapter opens no resource.

The model-package adapter owns compatibility, artifact selection and verification. Its prepared
resource implements `PreparedClassificationModel`; the application retains that complete owner
through publication or disposal. File handles, namespace guards and the staging lease remain inside
the adapter and retire before the application releases installation ownership. Recovery precedence,
explicit download authority, bundled read-only verification and runtime-retirement admission stay
with their existing workflow owners. The three factories admit no neighboring policy function.

Replacement must preserve the [managed installation contract](../0034-versioned-classification-contracts.md#managed-model-installation),
including digest/length admission, guarded source/storage identities, bounded transfer and durable
intent recovery. The prepared contract exposes selection paths without transferring or releasing its
guards; publication cannot detach those resources or extend their authority to another request.

## Scan source resources

`scan_library/production.rs` selects discovery, media inspection, publication namespaces and source
state through `ScanFileDiscovery`, `MediaInspector`, `PublicationNamespaceFactory` and `SourceStateProbe`.
The discovery owns the pinned configured root and lends
each directory cursor for traversal. `ScanDirectoryEntries::finish` consumes an exhausted cursor
only after rejecting read failures and revalidating the directory identity. The same native cursor
retains its directory handle through completion or drop; cancellation cannot turn partial coverage
into a completed binding.

`domain/source_discovery.rs` owns file-visit outcomes shared by scan and change reconciliation.
Native admission retains no-follow/no-recall access and held-handle metadata. Application traversal
retains checkpoint, window, cancellation and progress order. `PublicationNamespace` owns the stronger
root/ancestor capability through admission, child consolidation or final publication. Its factory
preserves expected-identity admission; its owner retains and drops the original native guard.
The stateless source probe revalidates file evidence without acquiring that namespace authority.

The scan catalog factory delegates to the validated session's foreground-scan acquisition and Recovery
lane, exposing scan repository ports and the existing journal capability contract. The retained initial-import
write scope restores its connection settings before terminal retirement; scheduling reclamation still
follows catalog drop. Root binding, resume admission and child retirement remain adapter transactions.

Replacing these selections preserves root acquisition order, borrowed cursor lifetime, completed
identity evidence and source-safe inspection. Only the registered factories may select their
concrete adapters; persistence, final validation and publication receive no neighboring exemption.

## Incremental source resources

`incremental_library_changes/production.rs::{publication_namespaces,media_inspector}` selects
`PublicationFileSource` and `GuardedMediaInspector`. Incremental changes and preview reconciliation
retain the admitted root/ancestor namespace through preparation, final source checks and catalog
publication or rollback. `MediaSource` opens the same root-relative, no-recall reader; inspection
consumes it without cloning a handle or reopening a mutable absolute path. Decoder limits, metadata
engine identity and retryable/terminal failure classification remain adapter-owned.

Replacing these selections preserves acquisition and retirement points, queue lease authority,
revision-rebase limits and transaction order. Only the two factories select concrete resources;
evidence policy, rename reconciliation and publication remain application responsibilities. Existing
namespace-sharing, source-rewrite, cancelled-rebase and preview-recovery fixtures cover this boundary.

The same selectors serve authoritative scopes, indexed startup and metadata recovery.
`PublicationDirectorySource` lends directory visits from the admitted namespace. Inventory opening
and each absence/final-publication phase retain their separate guard scopes. `IndexedRecoveryRepository`
owns connection-local hints, binding the existing recovery namespace and atomic candidate publication;
its implementation delegates to the existing spool and queue transactions. Index observations grant
no absence authority, and cancellation or a replaced lease cannot publish partial hints.

## Preview store resources

`preview/store_admission.rs` owns native store and cache-namespace composition under the existing
[budget and reclamation contract](../0005-storage-governance-and-budget.md#budget-and-automatic-reclamation).
Its exact host fields, constructors and resolution methods may name these concrete resources.
Borrowed access exposes only `PreviewStore` and `PreviewCacheAccounting`; materialization, staged
commit/discard and atomic accounting retain their original adapter implementation.
Generation or reclamation access precedes store selection. The operation retains its directory
identity across the bounded capacity handoff; each new access selects the current accounting owner.

Store inventory runs outside the registry mutex. A second short lock publishes or joins the current
owner, keyed by root, budget and namespace identity. Invalidation cannot cross admitted access, and a
retained store reference cannot preserve a retired selection's authority. Replacement substitutes
namespace and store construction together, preserving those lifetimes. This admission does not cover
preview workflow, cleanup/recovery policy or engine details in public ports.

## Preview source resources

`preview/production.rs::{sources,media_inspector}` selects `PreviewSourceFactory`,
`SourceIdentityLocator` and `SourceMediaInspector`. Selection opens no source. Initial reading follows
catalog/root admission;
metadata inspection runs only for invalidated metadata and uses the admitted source's byte-zero reader.
The factory keeps initial reading separate from publication protection: the latter binds the source
and ancestor namespace after decode revalidation and remains held through catalog publication or
rollback. Both phases retain their original identity, revision, no-follow/no-recall checks and errors.

Replacing these selections preserves the two acquisition points, held-handle reading, final
revalidation and retirement order. Native files and namespace proofs remain adapter-owned; use cases
receive only the reader, evidence and revalidation contracts. Only these factories select concrete
implementations; mismatch reconciliation and publication policy receive no neighboring exemption.

For a missing source, the identity locator returns current paths within the expected root using
metadata only. The Windows binding in `windows_identity_journal/metadata_location` retains identity
scheme, volume, root, file and non-directory validation without enumerating the source tree. Other
platforms retain unavailable identity lookup. Application chooses the exact/relocated path work and
keeps request-generation admission, namespace guards and catalog publication authority. Replacing
the locator preserves those checks and cannot authorize media mutation or placeholder hydration.

## Viewer source resources

`viewer_source::acquire_guard` constructs the native source/ancestor guard for a current catalog
selection. `ViewerSourceLease` retains that guard before its process-capacity reservation, so drop
retires native access before admitting a replacement lease. The exact factory result, acquisition
and private field edges are retained; request validation, catalog comparisons and capacity policy
receive no adapter exemption.

The existing [original-image contract](../0009-accepted-unified-gallery-ui-contract.md#image-viewer)
retains bounded acquisition, no-follow/no-recall opens and the complete namespace. Catalog admission
is rechecked after native acquisition; failure drops the unpublished guard. Replacement must preserve
that ordering, source sharing restrictions and opaque bridge lifetime without exposing native handles
or admitting an unchecked path fallback.

## Synchronization source state

`library_synchronization/production::new_production_source_state` selects `LocalSourceState` through
`SourceStateProbe` at the existing root-observation point. It acquires no retained handle and performs
metadata-only, no-recall inspection. Availability evidence and runtime projection remain Ame-owned;
replacement changes this factory and port implementation without adding source reads or hydration.

## Synchronization observation resources

`production::new_production_synchronization` selects `production_library_change_source_factory`
through `LibraryChangeSourceStarter`. Selection creates only the callback; each observer starts its
source from the admitted root request and owns bounded draining, stop and joined retirement through
`LibraryChangeSource`. Application retains restart, root-generation and publication decisions.

`ProductionSynchronization::runtime` binds the opaque `LibraryChangeIngress::Reservation` associated
type from the retained SQLite catalog. `LibrarySynchronizationRuntime` and `ObserverHandoff` remain
generic over that reservation; they neither inspect it nor acquire a database transaction. Handoff
retains the unpublished plan and ordering place across writer contention, retires admission before
other retries, and releases it on stop or root removal without claiming publication.

The adapter validates catalog identity, root generation and lane before zero-wait writer admission;
its complete enqueue transaction owns commit/rollback and restores the connection timeout. Replace
the observation factory through its source port, and replace the reservation binding together with
the poll catalog's ingress implementation. These two exact bindings admit no native calls in other
coordinator methods and expose no database or watcher types through ports or bridge contracts.

## Journal boundary resources

`JournalRuntimeSession::boundary_source` borrows the retained journal session and caller inside the
adapter. `scan_library/resumption::new_production_resume_boundary_source` selects an independently
owned connection. Both expose `ports/journal_boundary` contracts; selection acquires no resources.
The adapter owns root registration, request/response conversion and operation-error translation.
Boundary results carry the adapter's protocol version, including LiveOnly results; opening and
closing publication retain that provenance without importing the broker's wire-version constant.

`JournalBoundarySource::Root` retains the native root identity handle through baseline publication or
resume verification. Opening and closing captures release that root without closing their borrowed
session. Resume acquisition preserves root description, caller admission and connection order;
`JournalBoundarySession::close` runs after verification, including a failed probe or verification,
and its error retains precedence. Connection retirement precedes the root guard's drop.

Replacement changes these source bindings and their port implementations together. Preserve the same
root identity, bounded query timeout, no-recall opens, cancellation/publication checks and close
authority. Baseline admission and scan-coverage decisions remain in application and persistence;
this admission covers only the independent resume factory outside the runtime binding below.

## Journal runtime resources

`production::new_production_synchronization` selects `production_journal_runtime_factory`;
`ProductionSynchronization` binds its default connection type to `NativeJournalConnection`.
The coordinator and scheduling functions are generic over `JournalRuntimeConnection`; workers
consume `JournalRuntimeSession`. These two exact native bindings admit no neighboring native calls.

The adapter retains the original connection and caller identity. Connection begins after watcher
startup, then obtains the caller; failed identity acquisition attempts to close that connection before
exposing LiveOnly availability. Session selection returns only an authenticated retained reader. Opening and
closing workers borrow its boundary source, while a volume worker consumes a cloned session with
the same caller. No worker opens another connection or gains runtime-close authority.
An unavailable connection still supplies its protocol version through the connection port for
LiveOnly capability publication. This metadata access neither connects nor acquires a resource.

Application retains once-only connection admission, status projection, root/volume rotation,
cancellation, deadlines and worker joins. Close runs in its existing worker after core retirement;
catalog retirement still precedes the close attempt and retains error precedence. The adapter maps
close errors into the same `ScanError`; a failed or timed-out close retains the original runtime and
prevents replacement. This boundary adds no scheduler, reconnect policy or shutdown registry.

Replace the native type and factory together through these ports. Existing watcher-before-journal,
close-failure, late-completion, baseline and cross-process recovery fixtures cover their lifetimes.

## Journal volume resources

`JournalRuntimeSession::volume_source` selects the retained session and caller inside the adapter
through `PersistentJournalVolumeSource`. Selection performs no I/O. The page worker retains each
opaque root registration through capture and catalog publication; the adapter creates the same
named or identity-based reader from those registrations and the catalog namespace port.

`adapters/persistent_journal` owns broker requests, native evidence conversion, bounded reads,
operation-error translation and trim diagnostics. Domain owns journal read-window/recovery rules
and shared deterministic identity generation. Application owns capture validation, failed-read
budgets, cancellation, transactions and publication. Moving the reader does not move those authorities.

Replacement changes this session binding and the port implementation without exposing authorization DTOs
or raw handles to the page worker. Preserve per-root failure isolation, captured window limits,
rename/lineage identity, namespace lookup, pending-read cancellation and borrowed-session lifetime.
Root handles retire at the original post-publication point, before the reader releases its session.

## Indexed recovery resources

`library_synchronization/production/recovery_work/automatic::new_production_search_index` selects
Windows Search through `SearchIndex`. Selection remains inside the admitted Recovery worker, after
root generation and scan-state revalidation, and only for indexed preparation. The stateless value
acquires no resources; its bounded read owns and retires the COM apartment and cursor, including on
cancellation or failure. Search results remain hints subject to existing namespace and source checks.

The worker opens the Recovery lane through `CatalogLaneSession`; its catalog implements
`IndexedRecoveryRepository` and `LibraryChangeQueue`. Retained-source handoff, lease deferral,
optional-index failure handling and joined retirement keep their existing owners. Replacement changes
the factory and `SearchIndex` implementation without moving recovery policy or source authority.

## Root location resources

`library_synchronization/production/root_location::new_production_root_location_recovery` selects the
attribute-only native locator through the recovery owner's required callable contract. Ordinary
construction takes that contract explicitly; test defaults use the same production factory.

`ports/root_location::LocatedLibraryRoot` owns the recovered path and a `PublicationNamespace` guard.
The native locator retains its existing volume/identity checks and complete ancestor acquisition;
boxing the guard preserves its drop lifetime. `RootLocationRepository` binds the existing SQLite
recovery transaction, including identity revalidation before acquisition and before commit, generation
checks, cancellation, writer preemption and atomic path/queue publication. The application retains
retry timing, worker cancellation and joined retirement. Replacing the locator or persistence must
preserve these boundaries without searching other volumes, hydrating sources or modifying media.

## Catalog reclamation resources

`catalog_reclamation/production.rs::{maintenance,capacity}` select the existing SQLite maintenance
adapter and the native capacity probe through `CatalogSpaceRepository` and `CatalogCapacity`.
The maintenance value retains only its catalog path; each adapter operation owns its admitted
connection, write permit and interruption registration. The stateless capacity probe performs only
the read-only platform query covered by [ADR 0005](../0005-storage-governance-and-budget.md#catalog-database-page-reclamation).

The application initializes the operation before selecting resources, then executes staging cleanup,
inspection, mode conversion, bounded page release and checkpointing through those ports. Capacity
admission remains specific to an eligible legacy rewrite; `FULL` mode keeps its online conversion.
The operation registry and catalog-session maintenance owner retain cancellation, preemption, proof
retention and terminal publication. Replacement changes the two factory selections and port
implementations without moving those authorities or admitting SQL into application code.

## Similarity analysis composition

`similar_images/production.rs::{fingerprint_reader,image_reader}` select `ExactContentReader` and
`SimilarImageReader`. The application source executor acquires shared background admission before
either reader, retains the source lease through publication, then retires it before admission.
`SimilarImageAnalysis` owns a single pending run, cancellation, foreground suspension and progress;
its bridge object exists before asynchronous dispatch so cancellation can precede execution.
The repository factory selects the validated Recovery-lane catalog. SQLite owns atomic publication,
run leases and bounded matching continuations; neither factory owns those policies or SQL.
The execution matching owner advances up to 128 windows or 100 ms between source reads, checking
cancellation, suspension and lease renewal between windows. Remaining work retains its continuation
and drains after source exhaustion; the time budget is checked between bounded database calls.
Bridge discovery excludes the internal adapter module; clients use the API session wrapper and
domain progress values.

## Duplicate and organization composition

`exact_duplicates/production.rs::{content_reader,source_state}` select native implementations through
`ExactContentReader` and `SourceStateProbe`. The former composes the existing guarded reader and
shared background admission. Its fingerprint lease retains source/namespace handles through
publication and releases them before the admission lease. The latter is a stateless, metadata-only
probe; it grants no content read, hydration or source mutation. Replace either implementation through
its port without changing application analysis or availability policy.

`exact_duplicates/availability.rs` owns root/file precedence. Offline, missing and inaccessible roots
return before file probing. Available and unknown roots retain file identity, size, modification time
and revision revalidation. Placeholder failures remain distinct from missing or changed content.
Duplicate review and gallery-location reads release their catalog snapshot before this inspection.

`organization::open_catalog` and `exact_duplicate_review::open_catalog` retain their named catalog
factories. They select storage and the validated session's Live and Recovery lanes respectively.
Callers consume existing repository ports; the factories own no query, transaction or policy.
Replacing persistence changes this construction seam and the port implementations together, while
preserving session validation, lane admission and caller-owned handle retirement.

`OrganizationBatchSession::open` remains the bounded selection resource constructor. It acquires a
capacity permit and shared cancellation control before constructing its repository. Failure drops
the permit; a successful `ActiveSelection` retains it with the repository. Subsequent methods consume
`OrganizationBatchRepository`. Close signals cancellation before waiting for repository retirement,
and repeated close cannot release capacity twice. Adapter replacement retains this complete lifetime.

Only these named factory edges are admitted. Their neighboring business functions, new adapter
dependencies, SQL and engine types receive no exemption. Boundary tests reject same-file policy
dependencies, missing decision/lifetime/replacement metadata, duplicate admission and stale entries.
