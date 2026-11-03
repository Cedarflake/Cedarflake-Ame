# Repository layout

This layout keeps product ownership visible while preserving Flutter, Rust, and
`flutter_rust_bridge` tool conventions.

## Top-level directories

| Path | Ownership |
| --- | --- |
| `.github/workflows` | Category-prefixed hosted quality and release workflow entrypoints |
| `lib/app` | Application bootstrap, shared presentation primitives, and desktop window integration |
| `lib/features` | Product features, each split by domain, application, adapters, and presentation as needed |
| `lib/src/rust` | Generated Dart bridge code; do not reorganize or edit by hand |
| `rust/src` | Ame-owned Rust domain, application, ports, adapters, and bridge API |
| `test` | Tests arranged to mirror their owning Dart source area |
| `integration_test` | Cross-layer Flutter and Windows integration workflows plus their private support files |
| `tool` | Stable repository command entrypoints used by contributors, agents, and CI |
| `docs` | Roadmap index, product contracts, scoped execution plans, architecture, acceptance evidence, and development documentation |
| `rust_builder` | Tracked Flutter/Rust build scaffold required by the local path dependency; vendored Cargokit sources are outside Ame-owned Dart linting |
| `windows` | Flutter Windows runner and generated plugin registration |

## Documentation ownership

- `docs/roadmap.md` is the sole stage, priority, blocker, and exit-decision index.
- `docs/product` owns stable user workflows and product requirements, not implementation status.
- `docs/plans` owns detailed execution of a roadmap-linked scope, not an independent stage queue.
- `docs/architecture` owns accepted technical decisions and replacement boundaries.
- `docs/acceptance` owns verification contracts, dated results, failures, and historical provenance.
- `docs/development` owns contributor workflows and repository/tool organization.

Keep the roadmap and documentation index as discovery entrypoints. Link to the owning record instead
of copying its complete implementation history into the roadmap.

## Dart ownership

The tree below is the target for the [post-R5 organization rollout](../roadmap.md#project-organization-after-r5).
Current files move through its execution slices. Dependency direction, public contracts,
composition and exceptions belong to
[ADR 0037](../architecture/0037-feature-module-dependency-boundaries.md), not a second rule list here.

```text
lib/
├─ app/
│  ├─ bootstrap/              # Feature-specific production bindings and startup
│  ├─ notifications/
│  ├─ presentation/
│  └─ window/
├─ features/
│  └─ <feature>/             # Existing library, settings and storage owners
│     ├─ domain/
│     ├─ application/
│     │  ├─ ports/
│     │  └─ <capability>/
│     ├─ adapters/
│     │  ├─ rust/
│     │  └─ platform/
│     └─ presentation/
│        ├─ pages/
│        ├─ widgets/
│        └─ coordination/
└─ src/rust/
```

| Location | Contents |
| --- | --- |
| Feature `domain` | Ame value models and stable domain rules |
| Feature `application/ports` | Interfaces for capabilities consumed by that feature |
| Feature `application/<capability>` | Use cases, application state/lifecycle owners and their Provider declarations |
| Feature `adapters/rust` | Native bridge implementations, engine DTO/error mappings and opaque-handle wrappers |
| Feature `adapters/platform` | Native pickers, Explorer actions and platform-backed preference implementations |
| Feature `presentation/pages` | Page entries and surface composition |
| Feature `presentation/widgets` | Feature-owned visual components, grouped by capability when useful |
| Feature `presentation/coordination` | View-only state/lifecycle coordination and presentation projections |
| `lib/app/bootstrap` | Separate feature-binding modules and cross-feature startup/shutdown composition |
| `lib/app/presentation` | Shared visual primitives with no feature-policy owner |

Create only directories with real content. A small capability may keep its files directly in its
owning layer; introduce capability subdirectories when several collaborators need a clear home.
Possible library groupings include catalog, previews, scanning, synchronization, duplicates,
organization and classification. Preserve the existing feature boundary unless a separate
ownership decision justifies changing it. File and directory names follow repository/language
conventions rather than a new naming scheme.

Move a native mapper with the adapter that uses it; shared adapter mappings may have a named helper
within that adapter area. Keep port failure/value types in their contract or domain owner. Keep
Provider declarations beside the state owner and production implementations in bootstrap bindings.
The contract and its platform implementation share a feature, not a source file or directory.

The platform/storage pilot demonstrates this placement in
[`library_platform_bindings.dart`](../../lib/app/bootstrap/library_platform_bindings.dart),
[`settings_bindings.dart`](../../lib/app/bootstrap/settings_bindings.dart) and
[`storage_bindings.dart`](../../lib/app/bootstrap/storage_bindings.dart). A new required external
capability declares its port and Provider in the feature, adds its concrete selection and required
Provider to that feature's binding module, and covers the actual production roster and omitted-binding
failure. Test-only memory preferences live in `test/support`; application declarations do not select
them as defaults. The same preference-store instance serves application, library and window contracts.

[`library_read_bindings.dart`](../../lib/app/bootstrap/library_read_bindings.dart) demonstrates native
catalog, layout-manifest, preview and source-reader selection. Startup reads through its catalog port,
and the same instance is bound for subsequent queries. Shared native mappings live in the library's
`adapters/rust` directory with their callers. Provider declarations do not re-export those mappings
or implementations. Test roots compose these production modules through
[`application_test_bindings.dart`](../../test/support/application_test_bindings.dart), then apply
scenario overrides by the same Provider identities.

[`library_synchronization_bindings.dart`](../../lib/app/bootstrap/library_synchronization_bindings.dart)
shows lifecycle composition: one selected gateway, one application polling controller and one
bootstrap start/close owner. Native status mappings stay beside the gateway; phase-age preservation,
failure retention and diagnostics remain application collaborators. Production does not import the
inactive synchronization fixture from test support.

[`library_duplicate_bindings.dart`](../../lib/app/bootstrap/library_duplicate_bindings.dart) groups
the duplicate ports and the separately owned analysis runtime. Native group/location/progress
conversion and failure translation stay in library `adapters/rust`; batching, stale-window
retirement, idle admission and foreground preemption remain in application. Production and test
assembly use the same Provider identities, with required production ports validated before use.

[`library_organization_bindings.dart`](../../lib/app/bootstrap/library_organization_bindings.dart)
selects the organization, review, archive, cleanup, conflict and sidebar-order ports. Archive/cleanup
task wrappers and event conversion live beside their native adapters. Sidebar move/kind/placement
values live in domain; optimistic ordering remains with its existing application owner. Settings
uses the explicit library controller and read contracts documented in ADR 0037.

Tests mirror their resulting source area. Keep an integration workflow at its canonical entrypoint
while moving only owned private support. Temporary compatibility exports belong to the migrating
slice and are removed before its closure; do not create a permanent parallel directory layout.

## Rust ownership

Retain `rust/src/domain`, `ports`, `application`, `adapters` and `api`. Cohesive capability modules
remain under their owning layer, with their tests beside the owner or under the existing test
convention. Production-composition admission follows ADR 0037's named-boundary rule; it does not
require new crates or moving generated FRB files. Existing process hosts and Windows platform
owners retain their accepted architecture and canonical entrypoints.

## Test ownership

- Keep unit, widget, and application tests under `test`, mirroring the source area they verify.
- Keep device-backed and cross-layer workflows under Flutter's `integration_test` convention.
- Put scripts and fixtures used only by an integration workflow under `integration_test/support`.
- Add a top-level `test_driver` directory only when a checked-in `flutter drive --driver` command
  requires a host-side driver, such as a web or performance workflow. Desktop integration tests
  executed with `flutter test integration_test/... -d windows` do not require one.

## Stable tool entrypoints

Quality and acceptance scripts stay directly under `tool` because their paths are public repository
commands indexed by `AGENTS.md` and defined in the
[quality command reference](../acceptance/quality/commands.md#repository-command-reference).
Internal helpers may move into a
support directory when more than one helper exists, but entrypoint paths must remain stable.

Script names begin with an ownership category so related commands sort together:

- `quality_*` for formatting, linting, daily verification, and their shared implementation;
- `integration_*` for device-backed integration workflows;
- `acceptance_*` for authorization-bound real-library checks and their guardrail tests;
- `performance_*` for explicit benchmarks;
- `release_*` for packaging and release-candidate verification;
- `bridge_*` for generated bridge maintenance.

Workflow files under `.github/workflows` use the same `snake_case` ownership prefixes. Generic
filenames such as `ci.yml`, `build.yml`, `test.yml`, and `release.yml` are not valid repository
entrypoints. The quality lint gate enforces both script and workflow filename categories.
Hosted CI additionally runs `quality_lint_workflows.ps1` with a checksum-verified actionlint binary.

Flutter-focused commands under `quality_*` share a repository-wide named mutex. Focused tests use
`quality_test_flutter.ps1`, which starts one test file at a time with Flutter concurrency fixed to
one. Direct parallel Flutter test processes are outside the supported repository workflow.

## Generated and local data

The following paths are local or generated and must not become source-layout destinations:

- `.dart_tool`, `.build`, `build`, and Rust `target` directories;
- `windows/flutter/ephemeral`;
- `.idea`, local logs, coverage, catalogs, previews, models, and source-media fixtures.

Generated bridge files remain tracked only where required by the existing FRB workflow. Rebuildable
application data and source media never belong in Git history.
