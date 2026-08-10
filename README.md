# Cedarflake Ame

Cedarflake Ame is a local-first Windows photo library for large personal image collections. It
brings folders from different disks and cloud-backed locations into one continuous library without
requiring a second complete copy of the source collection.

Ame first builds an understandable view of a library, then supports deliberate organization through
separately controlled file operations. Catalogs, previews, analysis evidence, and user decisions are
stored apart from source media. Ame is not a backup service, cloud synchronization client, or pixel
editor.

## Product model

Ame treats a personal image collection as one library rather than a collection of unrelated tools:

- imported folders are sources that scope the same gallery;
- folder, date, search, sort, layout, and analysis state compose within one browsing canvas;
- the gallery loads continuously while the backend reads bounded, revision-safe windows;
- one logical asset may have more than one physical file location;
- exact duplicate evidence, similarity evidence, and later local classification remain distinct;
- file-changing operations are introduced as separate workflows only after they can be proposed,
  reviewed, revalidated, logged, and explicitly authorized.

The interface follows an accepted Microsoft Photos-like information architecture while using
Flutter Material 3 components and interaction behavior. Sources and albums belong in the sidebar,
search remains global, and contextual gallery actions replace separate feature pages.

## Design principles

### Local first

Cataloging, previews, metadata, and future analysis run locally. Source paths and derived data do
not need a hosted service.

### Separate analysis from file operations

Indexing, browsing, and analysis do not silently change source media. Move, copy, rename,
recycle-bin, and delete operations are planned organization capabilities with separate review,
current-state revalidation, explicit authorization, operation history, and a recovery strategy where
applicable. Cloud-only placeholders are detected without automatically downloading them.

### Large-library by design

Discovery, catalog queries, gallery rendering, and preview work are bounded. Long-running work is
observable, cancellable, recoverable, and isolated from individual corrupt or inaccessible files.

### Evidence over assumptions

Ame distinguishes a logical asset, its physical locations, source-state evidence, content identity,
and versioned analysis results. A path or file extension alone is not treated as permanent identity
or proof of image content.

### Replaceable engines

SQLite, media inspection, metadata, duplicate analysis, similarity, classification, and platform
integration sit behind Ame-owned contracts. External libraries may provide mature capabilities,
but their types and storage formats do not define the product domain.

## How it works

1. Rust discovers supported media and records structured per-file issues without failing the whole
   library.
2. A Rust-owned SQLite catalog publishes completed source state atomically and exposes bounded
   query windows.
3. Derived previews are generated on demand into a capacity-limited cache outside source folders.
4. Generated `flutter_rust_bridge` contracts connect the Rust application layer to Flutter.
5. Flutter renders the unified Material 3 gallery, viewer, source navigation, and settings without
   accessing the filesystem or database directly.

The core concepts remain separate: `LibraryRoot`, `Asset`, `AssetLocation`, source-state evidence,
versioned analysis runs, durable user decisions, and reviewed operation plans.

## Current state

Ame is under active development. The repository contains a working Windows desktop application with
non-mutating multi-root indexing and browsing, bounded gallery queries, lazy previews, source
scoping, local search and sorting, date navigation, selection, and an image viewer. Scans are
resumable and preserve the last trustworthy completed catalog when replacement work is cancelled,
interrupted, or stale.

Move, copy, rename, recycle-bin, and delete controls are not yet available; they remain planned
organization capabilities and will be introduced only with reviewed plans, revalidation, operation
history, and explicit authorization. Exact duplicate review, perceptual similarity, and local
classification are also not yet available. These future capabilities must use the same catalog,
safety, and replaceability boundaries rather than forming separate applications.

## Architecture

The repository keeps presentation, application policy, and replaceable infrastructure separate:

| Area | Responsibility |
| --- | --- |
| `lib/app` | Flutter bootstrap, shared presentation, and Windows desktop integration |
| `lib/features` | Feature-owned Dart domain, application, adapter, and presentation code |
| `rust/src` | Rust domain, application use cases, ports, adapters, persistence, and bridge API |
| `test` | Dart tests arranged to mirror their owning source area |
| `integration_test` | Cross-layer Windows and Flutter integration workflows |
| `tool` | Stable formatting, quality, integration, acceptance, performance, bridge, and release commands |
| `docs` | Architecture decisions, acceptance contracts, and development documentation |

The complete ownership map is documented in
[`docs/development/repository-layout.md`](docs/development/repository-layout.md).

## Independent implementation

Ame is implemented independently. Lap and other photo-library applications may be inspected as
external product, architecture, performance, and failure references. Their source code, UI
components, assets, schemas, and internal types are not copied or vendored into this repository.

## Development

### Requirements

- Windows 11 x64;
- Flutter stable with Windows desktop support;
- stable Rust with the `x86_64-pc-windows-msvc` target;
- Visual Studio 2022 with Desktop development with C++, MSVC, Windows SDK, and CMake tools;
- the bridge-generation tools declared by the repository.

Install Dart dependencies and run the application:

```powershell
flutter pub get
flutter run -d windows
```

After changing public Rust bridge types, regenerate the typed bridge:

```powershell
.\tool\bridge_generate.ps1
```

Run a focused Flutter test through the repository's serial, lock-aware entrypoint:

```powershell
.\tool\quality_test_flutter.ps1 `
  -TestPath test\features\library\presentation\unified_library_screen_test.dart
```

Run the daily quality gate:

```powershell
.\tool\quality_verify_daily.ps1
```

Windows packaging and real-library acceptance have separate, explicitly invoked gates. Their
requirements and safety constraints are documented rather than folded into ordinary development
commands.

## Project documentation

- [Architecture decisions](docs/architecture/README.md)
- [Repository layout](docs/development/repository-layout.md)
- [Quality gates](docs/acceptance/quality-gates.md)
- [Read-only real-library acceptance](docs/acceptance/read-only-real-library.md)
- [Project engineering contract](AGENTS.md)
