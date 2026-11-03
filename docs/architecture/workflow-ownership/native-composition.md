# Native production composition

Status: binding scoped requirements under [ADR 0037](../0037-feature-module-dependency-boundaries.md).

Read this contract when changing a retained native factory or its resource lifetime. Exact admitted
source/symbol/target edges and regression owners live in
[`rust_boundary_policy.json`](../../../tool/dependency-boundaries/rust_boundary_policy.json).
Retained entries are technical composition boundaries; `activeIds` remain unfinished migration.
Both kinds reject new or stale dependencies. The fixed legacy inventory is never regenerated.

## Validated catalog session

`catalog_session.rs` is the application resource host for validated catalog sessions. Its registered
state/decision types and validation-flight methods retain the concrete, shared session proof.
Validation and refresh functions share one flight per catalog path, including its original failure,
bounded retry delay and invalidation result. Expensive validation runs outside the registry and
entry-transition locks; a late flight cannot replace a newer owner or an invalidated proof.

The registered catalog and read-executor factories select connections from that proof. Foreground
scan acquisition rechecks the current session identity before protecting it; ordinary refresh cannot
replace a session owned by an active scan. Idle recovery keeps its current-session and scan-exclusion
checks around acquisition and the supplied operation. Maintenance retains its separate reservation
and proof-retention contract in [catalog ownership](catalog-and-migrations.md).

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

`load_catalog/production::source_state` selects the stateless `SourceStateProbe` after preview
reconciliation and recovery scheduling. `load_catalog/root_availability.rs` projects fresh root
evidence and messages without probing media content or changing catalog identity/counts. An
unavailable root does not prevent later roots from being inspected. Replacing this probe does not
move preview recovery, catalog publication or source-read authority.

## Storage resources and layout

`storage/production.rs` selects settings through `StorageSettingsRepository`, cache layout/accounting
through `PreviewCacheStorage`, and default locations through the existing OS directory provider.
The default byte budget remains application policy. The cache adapter owns its versioned child path
and read-only accounting of recognized current/legacy entries; it does not traverse nested directories
or delete files. Default-path selection creates no directory or database.

Each settings call owns its original connection lifetime. Configuration validation, transition
admission, restart activation and cross-database pending-root retirement keep their existing order.
Replacing storage changes these factory selections and ports together, without moving those rules
or acquiring a second process-wide configuration owner.

`domain/path_display.rs` owns the pure Ame path-display projection shared by storage, catalog reads,
scan events and diagnostics. It has no filesystem or platform dependency and grants no access
authority; stored access paths remain intact. Its existing string contract needs no adapter port.

## Classification execution resources

`NativeClassificationRuntime` owns one dispatcher, its catalog connection, executor and process
ownership token. `NativeExecutor` and its registered residency/resource types own the feature cache,
prepared matcher and native encoder factory. Cache acquisition is lazy; constructing the runtime
does not start an encoder. The existing [residency contract](../0034-versioned-classification-contracts.md#worker-ownership-and-protocol)
owns demand loading, cancellation, idle retirement and replacement. Closing the host retires the
executor before releasing process ownership; unproven retirement prevents another runtime.

`analyze_classification_content` and `verify_classification_source` are source-resource entrypoints:
they select the guarded image or exact reader and the existing catalog/background admission, then
invoke use cases through Ame ports. The [dispatch contract](../0036-durable-classification-dispatch.md#durable-execution-and-compatibility)
owns acquisition order and publication. Held source/namespace leases outlive publication or rollback;
text work retains its separate per-request admission without double-admitting image execution.

Replacing the native selections preserves those lifetimes and the existing use-case contracts.
SQL, inference protocols and engine types remain outside domain/ports/bridge. Only the registered
construction, host-state and borrowed-resource edges are admitted; configuration, discovery and
installation receive no neighboring-file exemption.

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
