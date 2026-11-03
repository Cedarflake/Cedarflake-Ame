# ADR 0037: Feature module dependencies and production composition

- Status: Accepted for validation
- Date: 2026-11-02

## Context

Ame already separates product features and exposes application interfaces. Some Dart files also
contain native implementations, bridge mappings and default Provider bindings. Other interfaces
live in adapter files, so their consumers import an implementation directory to obtain a contract.
These arrangements make dependency ownership inconsistent even when callers use interface types.

The target must preserve Flutter/Riverpod and Rust conventions, existing resource lifetimes and
the invariant owners established by [ADR 0025](0025-invariant-owned-workflow-modules.md). Directory
changes must make those boundaries inspectable without introducing another service framework.

## Decision

Organize product code by feature, then by responsibility within the feature. A module has one
named responsibility, typed inputs and outputs, and a discoverable owner for its state and resources.
Use subdirectories for established capabilities with several collaborators. Do not create empty
layers, a directory per class, or a generic `services`, `helpers` or `utils` bucket without an owner.

The [repository layout](../development/repository-layout.md#dart-ownership) owns physical locations.
This decision owns dependency direction, public contracts, composition and exceptions. The
[roadmap](../roadmap.md#project-organization-after-r5) owns activation; validation of a migrated
boundary does not establish conformance for the rest of the repository.

### Dart dependency rules

| Owner | Allowed dependencies | Responsibility boundary |
| --- | --- | --- |
| Domain | Dart value/collection primitives and explicitly shared pure domain contracts | No presentation, Provider, adapter, generated bridge or platform I/O dependencies |
| Application ports | Ame domain contracts and Dart asynchronous primitives | Define required operations and results; no concrete implementations, Provider bindings or engine DTOs |
| Application use cases and state owners | Domain, ports and cohesive application collaborators; existing Riverpod/foundation state primitives where needed | Coordinate client workflows and publication; no widget construction, native bridge imports or concrete adapter construction |
| Adapters | Implemented ports, Ame values, adapter-local helpers and admitted external libraries | Own native calls, DTO/error conversion and implementation resources; no dependency on application controllers or presentation |
| Presentation | Ame domain values, named application entrypoints, declared Provider handles and presentation collaborators | Own layout, input and transient view state; no direct native bridge, database or source-filesystem operations |
| Production composition | Application contracts, lifecycle owners and concrete adapters | Select and connect implementations and establish resource ownership; no business policy, SQL or duplicated task state machines |

Dart application orchestration does not take durable catalog or analysis policy away from Rust.
Flutter layout, painting, input and accessibility stay with their framework owners. Existing desktop
window integration retains its platform boundary; a feature must use its declared contract rather
than reach into a plugin implementation.

Dependency rules cover imports, exports, conditional imports/exports and their transitive public
surface. A barrel file cannot make an adapter dependency into an application dependency. Use explicit
exports for the small public surface; avoid feature-wide exports that expose every internal owner.
Keep the module graph acyclic. Resolve a cycle through ownership or an existing port, not dynamic
lookup, a service locator, duplicate types or another global mutable registry.

### Contracts and cross-feature use

Create a port for an actual external capability, replacement boundary or independently testable
workflow dependency. Ordinary pure functions and internal collaborators need no interface solely
for naming symmetry. Keep port inputs/results in Ame types; translate bridge values and platform
exceptions inside the adapter, including opaque native handles with an explicit close contract.

A feature may consume another feature's explicitly exposed domain contract or application entrypoint.
It must not import that feature's adapters, pages or private lifecycle helpers. Declare the exported
entrypoint and its consumer in the affected boundary's documentation when that relationship is new.
State publication and cancellation stay with the owning feature; callers cannot acquire authority
by mutating another controller's internal state.

The Settings organization-record surfaces consume the library's named archive and cleanup
controllers, their request/state projections, and archive/cleanup inspection/failure values.
`libraryArchiveControllerProvider` owns export, inspection, confirmed restore and retry;
`libraryCleanupControllerProvider` owns inspection and confirmed retirement. Settings renders their
state and submits intents without owning task handles or publication. Album preferences use
`libraryAlbumProvider` for a bounded saved-album lookup. Exact consumer-to-contract registrations
live in `tool/dependency-boundaries/dart_boundary_policy.json`; this admits no other library internals
or cross-feature widget imports.

Cross-feature assembly belongs in application bootstrap. A business workflow spanning features
still has one application owner and narrow collaborators. Promote a component to shared ownership
only when independent consumers need the same stable behavior; shared visual primitives cannot
depend on feature modules. A common directory must not become a second implementation of feature
policy. Page count alone does not justify splitting a feature into packages or Rust crates.

### Provider and resource composition

Keep Provider declarations beside the application or presentation state they expose. Port files
contain contracts only. These declarations may compose other application owners through injected
contracts, but cannot select concrete external implementations. Pure value defaults are permitted.

Production bindings live in feature-specific modules under `lib/app/bootstrap`; `main.dart` invokes
the resulting startup flow. Do not replace scattered bindings with one growing universal container.
Tests override the same Provider identities or inject the same constructor contracts. Test factories
belong in test support and cannot become production fallback implementations.

Every resource has one explicit lifetime owner. Preserve Provider identity, family keys, scope,
caching, automatic disposal, startup order and joined shutdown when relocating declarations or
bindings. Construction failure releases already acquired resources. Missing required production
bindings fail at assembly, before the affected workflow starts; they cannot silently select a fake,
an unguarded native path or a second runtime. Validate normal and failure bootstrap with the same
production binding roster used by the application.

`AmeProviderAssembly` owns the single root Riverpod container. Create its lifetime owner before
acquiring background resources so the existing reverse-order shutdown joins those resources before
disposing the container. Feature binding modules supply typed overrides and required-port rosters;
assembly reads those required ports before publishing the container through `UncontrolledProviderScope`.
Failed validation disposes the unpublished container. `acquireAmeApplication` retires already acquired
resources through the existing shutdown coordinator and preserves both acquisition and cleanup
failures. This owner contains container lifetime and validation only; feature modules retain concrete
service selection and application owners retain workflow policy.

`LibraryReadBindings` supplies the same catalog to startup loading and the catalog/folder Providers.
It selects native preview and source-reader adapters with one foreground-read activity contract.
The application source-scheduler Provider constructs and retires its scheduler from the reader port;
neither image widgets nor image providers construct a replacement native reader. The existing image
copy owner retains acquired-lease closure, including after container or widget retirement.

`LibrarySynchronizationBindings.production()` selects the native lifecycle gateway and the
application polling controller, then binds that same controller to the retained bootstrap lifecycle
owner. The application constructor accepts only an Ame gateway; generated policy still controls
production cadence, and test-only constructors retain timing/failure injection. The inactive
synchronization fixture exists only in test support.

`LibraryDuplicateBindings` selects exact-analysis/query, gallery-annotation and duplicate-review
ports. Its exact service is shared with `LibraryExactAnalysisBindings`, which constructs the one
analysis controller and scheduler using the existing foreground-read authority. The runtime binding
owns joined, idempotent close followed by notifier disposal; application owners retain admission,
cancellation and publication policy. Required production Providers cannot fall back to an absent
scheduler or a second controller. Test support explicitly selects inactive scheduling where needed.
Duplicate adapters translate native failures into Ame-owned code/message values.

`libraryOrganizationBindings` supplies organization, batches, review, archive, cleanup, conflicts and
sidebar-order ports. Native task wrappers own bridge handles and event conversion; the existing
archive/cleanup controllers retain cancellation, terminal-result admission, handle retirement and
post-commit publication. Required binding does not create an operation or grant confirmation.

### Presentation ownership

Pages compose surfaces and connect user intent to application entrypoints. Widgets render bounded
input and own local framework interaction. Presentation coordinators own view-only lifetimes such
as selection geometry, viewport continuity and popup sequencing; catalog authority, persistence and
background execution remain in application owners.

Widgets and coordinators must not import pages. A page composes another feature through an explicit
navigation destination or application contract, not that feature's internal widget implementation.
Keep feature-specific widgets in their feature. Place a visual primitive in `lib/app/presentation`
only when its inputs and behavior are independent of feature policy.

Assign a class by its responsibility rather than its `Controller`, `Service`, `Page` or `Widget`
suffix. Moving a file does not justify moving cancellation, query-generation or source-lease
authority to a different layer. Preserve complete lifecycle owners under ADR 0025.

### Rust ownership

Retain the existing domain, ports, application, adapters and bridge API modules. Domain models and
port signatures expose Ame contracts. Business use cases consume ports; adapters own engine calls,
SQL and persistence implementation. Bridge APIs translate requests/results into application calls.

Production factories may construct concrete adapters in explicitly identified composition functions
or runtime hosts, including the catalog-session boundary. Such admission is scoped to named owners,
not every file under `application`. Factories own construction and retirement, while use cases retain
policy and adapters retain transactions. A factory exemption does not admit SQL, engine types in
public contracts or concrete dependencies in unrelated application code.

The validation inventory must name the admitted Rust composition functions and their lifetimes.
An existing construction call alone is not evidence that the surrounding module is a composition
root. Establish the affected seam when responsibilities conflict; do not mechanically move every
production factory, split crates or change bridge visibility to satisfy a directory pattern.

### Future changes and exceptions

Before adding a capability, identify its feature, layer, public contract, state/resource owner and
proving regression. Extend the narrow existing owner or introduce an independent owner under
ADR 0025. Do not place new behavior in a large facade merely because it already imports the inputs.
Update paths, public exports, production bindings, tests and necessary documentation together.

The dependency check must report the source, resolved target and violated rule. A temporary exception
names an exact file or symbol, the dependency, its owning responsibility, reason, focused coverage
and a removal condition tied to an execution slice. No directory-wide exemptions or automatic
acceptance of new violations are allowed. Remove an exception in the same change that closes it.
Permanent exceptions require an accepted amendment explaining their invariant and replacement path.

Existing violations are migration work, not templates for new code. The execution plan inventories
them against a fixed baseline; each migrated slice prevents their return. The roadmap determines
when this target applies to new work and when repository-wide enforcement becomes active.

## Alternatives

- Keep interfaces and implementations together everywhere: reduces initial navigation but leaves
  application consumers coupled to bridge imports and makes production construction difficult to audit.
- Use global type-based directories for all features: separates file kinds but spreads one workflow
  across the repository and weakens feature ownership.
- Require an interface for every function or a package for every capability: adds forwarding and
  public-surface cost without a concrete replacement or lifecycle boundary.
- Move files only: improves naming while preserving reverse dependencies and mixed state authority.

Feature-local layers with explicit composition retain the current product boundaries and make
dependency rules enforceable without those costs.

## Consequences and risks

More explicit bindings and a limited set of new directories increase initial navigation. Relocating
Provider declarations can change identity or disposal if duplicated; startup construction can also
lose a cleanup path. Adapter extraction must preserve exception conversion, opaque-handle ownership
and source guards. Tests, tool references and generated-code boundaries move with the affected owner.

This decision admits no new package, model, schema, archive format or engine. Original-media safety,
licensing, runtime resource budgets and approved UI behavior retain their existing owners. A change
to any of those contracts needs its separate decision and verification rather than a layout label.

## Validation and rollback

Validate dependency checking with admitted and rejected imports, exports, conditional paths,
cross-feature cycles and narrow exception removal. Keep parser behavior and rule configuration
testable; text matching against one import spelling is insufficient. Include handwritten Dart and
the scoped Rust boundaries while excluding only genuinely generated or admitted vendored output.

For each migrated boundary, prove production/test assembly, adapter substitution and the affected
success, failure, cancellation, disposal and stale-completion paths. Follow
[quality-gate ownership](../acceptance/quality-gates.md#delivery-and-verification) for automated and
client verification. Change this decision to Accepted only after its structural and behavior gates
have evidence for the delivered scope; code organization alone does not establish product acceptance.

Each delivery slice includes moves, imports, bindings, tests and documentation in one reversible
unit. Revert the owned slice through an ordinary follow-up commit if its behavior cannot be retained;
preserve catalogs, decisions and source media. Replace this target only through a superseding ADR
that records the ownership problem and migration path. Rollback never authorizes history rewriting.
