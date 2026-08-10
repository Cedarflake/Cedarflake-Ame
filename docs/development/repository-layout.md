# Repository layout

This layout keeps product ownership visible while preserving Flutter, Rust, and
`flutter_rust_bridge` tool conventions.

## Top-level directories

| Path | Ownership |
| --- | --- |
| `lib/app` | Application bootstrap, shared presentation primitives, and desktop window integration |
| `lib/features` | Product features, each split by domain, application, adapters, and presentation as needed |
| `lib/src/rust` | Generated Dart bridge code; do not reorganize or edit by hand |
| `rust/src` | Ame-owned Rust domain, application, ports, adapters, and bridge API |
| `test` | Tests arranged to mirror their owning Dart source area |
| `integration_test` | Cross-layer Flutter and Windows integration workflows plus their private support files |
| `tool` | Stable repository command entrypoints used by contributors, agents, and CI |
| `docs` | Architecture decisions, acceptance contracts, and development documentation |
| `rust_builder` | Tracked Flutter/Rust build scaffold required by the local path dependency |
| `windows` | Flutter Windows runner and generated plugin registration |

## Dart ownership

```text
lib/
├─ app/
│  ├─ bootstrap/
│  ├─ presentation/
│  └─ window/
├─ features/
│  ├─ library/
│  │  ├─ adapters/
│  │  ├─ application/
│  │  ├─ domain/
│  │  └─ presentation/
│  ├─ settings/
│  │  ├─ adapters/
│  │  ├─ application/
│  │  └─ presentation/
│  └─ storage/
│     ├─ application/
│     └─ domain/
└─ src/rust/
```

- Put a platform implementation beside the feature or application contract it implements.
- Put cross-feature application bootstrap in `lib/app`, not in a global adapter bucket.
- Put reusable visual primitives in `lib/app/presentation` only when more than one feature owns no
  better home for them.
- Keep tests under the corresponding `test/app` or `test/features` path.

## Test ownership

- Keep unit, widget, and application tests under `test`, mirroring the source area they verify.
- Keep device-backed and cross-layer workflows under Flutter's `integration_test` convention.
- Put scripts and fixtures used only by an integration workflow under `integration_test/support`.
- Add a top-level `test_driver` directory only when a checked-in `flutter drive --driver` command
  requires a host-side driver, such as a web or performance workflow. Desktop integration tests
  executed with `flutter test integration_test/... -d windows` do not require one.

## Stable tool entrypoints

Quality and acceptance scripts stay directly under `tool` because their paths are public repository
commands documented in `AGENTS.md` and acceptance contracts. Internal helpers may move into a
support directory when more than one helper exists, but entrypoint paths must remain stable.

Script names begin with an ownership category so related commands sort together:

- `quality_*` for formatting, linting, daily verification, and their shared implementation;
- `integration_*` for device-backed integration workflows;
- `acceptance_*` for authorization-bound real-library checks and their guardrail tests;
- `performance_*` for explicit benchmarks;
- `release_*` for packaging and release-candidate verification;
- `bridge_*` for generated bridge maintenance.

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
