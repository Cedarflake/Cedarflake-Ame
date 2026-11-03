# Project organization: dependency baseline inventory

This inventory describes source revision
[`f74146a4`](https://github.com/Cedarflake/Cedarflake-Ame/tree/f74146a4e88d72325c576504825399df109774aa).
The [execution plan](project-organization.md) owns the rollout and the
[roadmap](../roadmap.md#project-organization-after-r5) owns its position and disposition.
Target dependency rules remain in [ADR 0037](../architecture/0037-feature-module-dependency-boundaries.md).
Source paths below describe that fixed baseline; target paths describe the corresponding migration.

## Rust composition and boundary inventory

The native inventory is fixed in
[`legacy_rust_dependencies.json`](../../tool/dependency-boundaries/legacy_rust_dependencies.json).
Every entry names its exact source, function/type, resolved dependency and rule, with a responsibility,
reason, lifetime, regression owner and disposition. The mutable
[`rust_boundary_policy.json`](../../tool/dependency-boundaries/rust_boundary_policy.json) selects
active IDs; a resolved dependency retires its ID without rewriting the baseline.

The initial inventory contains 279 edges: 26 retained composition edges, one borrowed preview-source
capability and 252 application integration edges for O7 classification. The latter remain individual
dependencies, not a construction exemption for their files. O7 preserves actual runtime hosts and
extracts a port only where the named use case mixes policy with implementation. No native product
code moves during O1.

| Current named owner | Resource and lifetime | Target / review boundary |
| --- | --- | --- |
| `catalog_session::CatalogSessionState`, `CatalogValidationDecision`, `CatalogValidationFlight`; `CatalogValidationFlight::{wait,complete}` | Shared validated session and joined validation result per catalog path | Retain these resource hosts in `application/catalog_session.rs`; do not admit unrelated functions in that file. |
| `catalog_session::{open_catalog,open_catalog_for_foreground_scan,open_catalog_reader}` | Lane-scoped catalog handles and a bounded read executor backed by the validated session | Retain the factory seam and foreground scan's current-session identity check. |
| `catalog_session::{validated_catalog_session,validated_catalog_session_with,validated_catalog_session_with_exemption,refresh_stale_catalog_session,refresh_stale_catalog_session_for_scan,with_recovery_catalog_if_idle}` | Validation, stale-session replacement and idle recovery acquisition | Preserve shared waiters, scan exclusion, failure backoff and retirement. |
| `classification_runtime::executor::{NativeEncoderResidency,FeatureResources,NativeExecutor}`; `NativeExecutor::{new,feature_resources,analyze}` | Encoder factory, residency and lazily acquired feature cache | Retain the native runtime host; analysis policy continues through its existing application collaborators and ports. |
| `library_synchronization::production::ProductionSynchronization`, `ProductionSynchronization::validated_catalog_session`, `new_production_synchronization` | Catalog-session host, change-source factory and identity-journal factory | Preserve one production host and the existing joined shutdown. Scheduling/polling functions in the same file are individually inventoried, not covered by this admission. |
| `ports::PreviewStore::materialize` → `std::fs::File` | Borrow of an already admitted source handle during encoding | Retain ADR 0025's held-source capability; this grants no path-opening or additional filesystem operation. |

Legacy integration ownership is recorded as 53 scan, 49 preview/viewer, 89 continuity, 36
classification, four exact-duplicate, two organization and 19 storage/read edges. Each has its exact
source and symbol in the fixed inventory. Use those entries to choose an affected native seam during
O7; do not turn a directory count into a bulk rewrite or a new product prerequisite.

The parser checks `domain`, `ports`, `application` and `api`; adapter declarations and re-exports
are loaded for name resolution. Test-only declarations are excluded, while all potentially enabled
platform/feature branches remain visible. API use of the generated `StreamSink` is the existing thin
bridge contract; domain/application use remains rejected. Generated FRB internals are not parsed.
Imports, grouped/renamed imports, module globs, local aliases, public re-exports and explicit macro
paths resolve to named dependencies. The Rust compiler and existing behavioral tests retain type
inference, method dispatch and resource-lifetime verification.

The gate is `rust/tests/dependency_boundaries.rs`, included by the existing Rust `other` partition.
`cargo run --locked --manifest-path rust/Cargo.toml --example dependency_inventory` prints a read-only
JSON report; it has no baseline update operation. Use the repository tool lock for local invocation.

## Contracts, adapters and composition

Keep each existing Provider identity with its application state owner. A contract moves into its
feature's `application/ports`; native implementations and DTO/error conversion move into
`adapters/rust`, while OS/plugin implementations move into `adapters/platform`. Application
projections, paging, publication and lifecycle state stay with their current invariant owners.
Only the binding modules listed below select external implementations.

Names in the source column are files within the stated feature's current `application` directory,
unless another location is given. Port filenames retain the capability's existing basename.

| Slice / capability | Baseline contracts and implementation owners | Target and retained authority |
| --- | --- | --- |
| O2 library directory selection | `adapters/directory_picker.dart`: `DirectoryPicker`, `FileSelectorDirectoryPicker`, `directoryPickerProvider` | Port `ports/directory_picker.dart`; platform adapter `platform/file_selector_directory_picker.dart`; declaration in application; `app/bootstrap/library_platform_bindings.dart` selects it. `LibraryPrimaryScanLifecycle` retains admission. |
| O2 library platform actions | `library_platform_actions.dart`; `adapters/windows_library_platform_actions.dart`, `windows_shell_reveal.dart` | Port `ports/library_platform_actions.dart`; implementations under `adapters/platform`; the same library platform binding owner supplies the Provider. Preserve Clipboard, Explorer and path validation. |
| O2 storage | `storage_settings.dart`: `StorageSettingsGateway`, `StorageDirectoryPicker`, both native implementations and Providers | Ports and platform/Rust adapters split by capability; `app/bootstrap/storage_bindings.dart`. Storage settings and catalog-reclamation application coordination remain distinct from their rendered Settings section. |
| O2 archive picker | Settings `organization_archive_picker.dart`: `OrganizationArchivePicker`, `PlatformOrganizationArchivePicker` | Settings port and platform adapter; `app/bootstrap/settings_bindings.dart`. Existing inspection/confirmation authority stays with archive application owners. |
| O2 preferences | Settings `ame_preferences.dart`, `album_preferences.dart`, `adapters/shared_preferences_ame_store.dart`; library `library_view_preferences.dart`; existing window preference contract | Pure preference values, store ports, state declarations and platform persistence receive separate owners. Settings bindings retain initial values and one shared preference store; named cross-feature consumers are registered explicitly. ADR 0012's desktop contracts remain at their existing boundary. |
| O3 catalog reads | `library_catalog.dart`: catalog, stable asset/query-anchor and folder contracts, `RustLibraryCatalog`, Provider declarations and mapping functions | Port `ports/library_catalog.dart`; `adapters/rust/rust_library_catalog.dart`; catalog Provider/application projections stay in application. `app/bootstrap/library_read_bindings.dart` supplies catalog and folder capabilities. |
| O3 catalog mapping | `library_asset_mapping.dart`, `library_gallery_query_mapping.dart`, `library_gallery_collection_mapping.dart`, `library_metadata_filter_mapping.dart` | Same named mapping owners under `adapters/rust`; Ame domain query/value types remain in domain. No generated DTO enters a port. |
| O3 layout manifests | `library_layout_manifest_catalog.dart`: manifest catalog/loader/request, Rust catalog, application loader and Provider family | Native catalog goes behind its port; application loader, chunk policy, request identity and family keys remain together. Bind the concrete catalog through library read bindings. |
| O3 previews | `library_previewer.dart`: `LibraryPreviewer`, `RustLibraryPreviewer`, failures and Provider | Separate port and Rust adapter; retain `LibraryPreviewCoordinator`, `LibraryPreviewQueue` and their publication generations. Read bindings select the preview engine. |
| O3 original-image reads | `library_source_reader.dart`, `rust_library_source_reader.dart`: reader/lease/failure, Rust lease wrapper and global source-read scheduler construction | Port and opaque lease remain Ame contracts; adapter owns native handles. Read bindings own the single scheduler instance; callers retain their existing acquisition/close authority. |
| O4 scan | `library_scanner.dart`: `LibraryScanner`, `LibraryRootRelocationScanner`, `RustLibraryScanner`, Provider | Separate ports/adapter and `app/bootstrap/library_scan_bindings.dart`; keep primary admission, serialized command/run lifecycle, explicit relocation and terminal refresh in their existing owners. |
| O4 synchronization | `library_synchronization.dart`: contract, Rust polling/runtime translation, tickets, Provider and inert implementation | Separate port/native mapping and binding. Preserve `app/bootstrap/library_synchronization_lifecycle_owner.dart`, its startup future, single polling owner and joined shutdown. Production assembly cannot silently select the inert implementation. |
| O5.1 exact analysis | `library_exact_duplicates.dart`, `library_gallery_duplicates.dart`, `library_duplicate_review.dart` | Their interfaces become ports and Rust classes become adapters; `app/bootstrap/library_duplicate_bindings.dart` supplies concrete services. Keep analysis scheduling/publication, annotations and review windows in application. |
| O5.2 organization | `library_organization.dart`, `library_sidebar_order.dart`, `library_review.dart` | Ports for organization/sidebar/review; Rust adapters retain prepared native handles and mapping. `app/bootstrap/library_organization_bindings.dart` supplies services; controllers retain command targeting and durable-result publication. |
| O5.2 multi-step organization | `library_organization_batch.dart`, `library_organization_archive.dart`, `library_organization_cleanup.dart`, `library_organization_conflicts.dart` | Task/batch/stream contracts move to ports; Rust task wrappers and event conversion move with adapters. Retain application request/state owners, cancellation, opaque task disposal and Provider family identities. Use the same organization binding owner. |
| O5.3 classification | `library_classification.dart`, `library_classification_configuration.dart`, `library_classification_review.dart`; existing `adapters/rust_library_classification*.dart` | Separate service/session/maintenance ports from state Providers; move existing adapters into `adapters/rust`; `app/bootstrap/library_classification_bindings.dart` selects them. Preserve current runtime and automatic lifecycle owners. |
| O5.3 model installation | `library_model_installation.dart`; `adapters/rust_library_model_installation.dart` | Session/service contracts become ports; adapter keeps native installation handles. The classification binding owner supplies them; `LibraryModelInstallation` retains explicit download authorization, progress, cancellation and recovery. |
| O6 presentation | `presentation/unified_library_screen.dart`, settings `presentation/ame_settings_page.dart`, library `presentation/widgets/library_exact_duplicates_page.dart` | Move entries into each feature's `presentation/pages`; view-only coordinators into `presentation/coordination`; keep framework widgets in `presentation/widgets`. Cross-feature pages/private widgets are composed through declared navigation or application contracts. |
| O6 shared visuals and cache images | `app/presentation/ame_theme.dart`, `ame_runtime_platform_io.dart`; library photo/viewer/filmstrip renderers | Translate feature settings at composition; retain the named desktop platform boundary. Validate narrow derived-cache rendering admission separately from original-source reads. Preserve Flutter-owned image rendering and source leases. |

`main.dart` and `app/bootstrap/ame_startup_orchestrator.dart` remain startup composition. The
feature/capability binding modules compose into that startup flow; they do not create another
shutdown registry, classification runtime, synchronization loop or source-read scheduler.
Required bindings and partial-acquisition cleanup receive production-roster coverage in O2 onward.

## Exact edges, consumers and regression owners

The immutable [Dart legacy inventory](../../tool/dependency-boundaries/legacy_dart_dependencies.json)
records 124 exact rule/source/target entries against the baseline. Thirteen named responsibility
profiles attach a reason, existing regression files and removal slice to every entry. Entries also
include edges exposed through existing barrel exports. The
[active policy](../../tool/dependency-boundaries/dart_boundary_policy.json) selects the still-active
IDs; closing a boundary removes its IDs in that same change.

| Responsibility | Baseline entries | Removal |
| --- | ---: | --- |
| Preferences and their cross-feature consumers | 23 | O2 |
| Directory selection and platform actions | 7 | O2 |
| Storage and Settings coordination | 7 | O2 |
| Catalog reads and DTO mappings | 14 | O3 |
| Preview and held-source reads | 5 | O3 |
| Scan and synchronization | 7 | O4 |
| Exact duplicates | 6 | O5.1 |
| Organization and archive/cleanup consumers | 41 | O5.2 |
| Classification declarations and adapter cycle | 6 | O5.3 |
| Cross-feature presentation composition | 3 | O6 |
| Derived-cache image rendering admission | 3 | O6 |
| Platform detection | 1 | O6 |
| Shared theme inputs | 1 | O6 |

The classification review cycle is exactly
`adapters/rust_library_classification_review.dart` ↔ `application/library_classification_review.dart`.
Its contract/Provider split and bootstrap binding must remove both edges' ownership conflict, not
conceal it behind an export.

The read-only internal report (`tool/dependency-boundaries/verify.dart --inventory`, under the
repository tool lock) lists each handwritten module's declarations, resolved imports, direct
callers and directly importing tests/support. It reports current source for migration work and has
no baseline-writing mode. The normal quality command validates the frozen Git blob fingerprint,
baseline revision, exact active IDs, stale exceptions and existing regression owners.

## Physical reviewability

These baseline counts include ordinary source lines. Inline test lines are zero for these Dart
owners. Dedicated test/support lines count distinct files that directly import each owner; shared
test files can appear for multiple owners, so the column is not additive. The primary scan cases
enter through `LibraryController`, including the workflow/control/refresh suites, and therefore
have zero directly importing test lines in this table. Counts identify review scope, not rewrite
thresholds or acceptance results.

| Baseline owner | Production | Inline tests | Direct test/support |
| --- | ---: | ---: | ---: |
| `lib/features/library/presentation/unified_library_screen.dart` | 1896 | 0 | 991 |
| `lib/features/library/presentation/widgets/library_gallery_wall.dart` | 1542 | 0 | 6826 |
| `lib/features/library/application/library_viewport_controller.dart` | 1301 | 0 | 2340 |
| `lib/features/library/presentation/widgets/library_time_navigation.dart` | 818 | 0 | 3412 |
| `lib/features/library/application/library_synchronization.dart` | 761 | 0 | 6573 |
| `lib/features/settings/presentation/widgets/storage_settings_section.dart` | 739 | 0 | 1209 |
| `lib/features/library/application/library_primary_scan_lifecycle.dart` | 697 | 0 | 0 |
| `lib/features/library/application/library_preview_queue.dart` | 696 | 0 | 3554 |
| `lib/features/library/domain/gallery_layout_manifest.dart` | 680 | 0 | 10199 |
| `lib/features/library/application/library_update_controller.dart` | 603 | 0 | 5791 |
| `lib/features/library/application/library_catalog.dart` | 548 | 0 | 24213 |
| `lib/features/library/presentation/widgets/library_gallery_header.dart` | 526 | 0 | 2479 |
| `lib/features/library/presentation/widgets/library_navigation.dart` | 524 | 0 | 1882 |
| `lib/features/library/presentation/widgets/library_source_navigation_tile.dart` | 517 | 0 | 2123 |
