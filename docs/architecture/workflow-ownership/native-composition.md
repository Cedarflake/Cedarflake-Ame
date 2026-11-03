# Native production composition

Status: binding scoped requirements under [ADR 0037](../0037-feature-module-dependency-boundaries.md).

Read this contract when changing a retained native factory or its resource lifetime. Exact admitted
source/symbol/target edges and regression owners live in
[`rust_boundary_policy.json`](../../../tool/dependency-boundaries/rust_boundary_policy.json).
Retained entries are technical composition boundaries; `activeIds` remain unfinished migration.
Both kinds reject new or stale dependencies. The fixed legacy inventory is never regenerated.

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
