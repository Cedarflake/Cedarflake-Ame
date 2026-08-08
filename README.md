# Cedarflake Ame

Cedarflake Ame is a local-first Windows photo-library application for understanding very large
personal image collections. It builds a derived catalog and bounded preview cache without modifying
source media or requiring a second full copy of the library.

The current implementation contains the accepted R0 technical-validation slice and the R1
multi-root, progressive-query, recoverable traversal, storage-governance, lazy-preview, and
capture-time-evidence slices, including incremental path reconciliation and a repeatable synthetic
large-library acceptance gate. It connects a Flutter Material 3 desktop interface to a Rust core
through generated, typed `flutter_rust_bridge` contracts.

## Current workflow

The working vertical slice supports:

1. selecting multiple local directories, one at a time, through the operating-system picker;
2. bounded, read-only discovery in Rust with disk-backed, windowed enumeration for very wide
   directories;
3. skipping cloud-only Windows placeholders instead of hydrating them;
4. isolating unreadable or unsupported files as per-item issues;
5. probing image dimensions without full pixel decoding and publishing preview work as pending;
6. parsing bounded EXIF capture-time evidence behind a versioned metadata adapter without inventing
   a timezone when the source omits one;
7. staging distinct library-root, asset, and location records in a Rust-owned SQLite catalog;
8. recording optional Windows volume-and-file identity evidence without treating it as content
   identity or duplicate evidence;
9. staging locations in bounded transactions and using indexed active-snapshot reconciliation so
   scan publication and cancellation do not degrade into full-table nested scans;
10. preserving one logical asset across a same-volume rename or in-place edit while invalidating
   derived results when observable file state changes;
11. giving a replacement at the same path a new asset identity and removing absent locations only
    after atomic publication;
12. revalidating indexed source size, modification time, and available file identity before
    publication;
13. publishing each root atomically without replacing previously completed roots;
14. streaming typed progress, assets, issues, completion, cancellation, and stale state to Flutter;
15. reloading the last completed bounded catalog at application startup and after every scan;
16. preserving the previous trusted gallery while a replacement scan is incomplete;
17. loading further catalog rows through revision-protected keyset cursors rather than page
    numbers or deep SQL offsets;
18. persisting the current directory, pending-directory frontier, and deterministic entry
    checkpoints so an unexpectedly interrupted scan resumes at application startup without walking
    previously completed directories;
19. replaying staged locations and issues idempotently without publishing partial recovery state;
20. pausing a scan from the upper action area, preserving its private checkpoint, and resuming only
    after an explicit user action;
21. requesting previews only for lazily built gallery tiles, cancelling queued off-screen work,
    and generating at most two previews concurrently;
22. displaying and configuring catalog storage, versioned preview storage, and the preview budget
    from the upper action area;
23. freezing active storage for the lifetime of the process and requiring restart for changes;
24. stopping new preview publication safely when its atomically enforced capacity is exhausted;
25. reporting each configured root as available, missing, inaccessible, or offline without walking
    or hydrating its contents;
26. rebuilding a visible preview when its rebuildable cache artifact has disappeared.

Validation scans currently stop after either 2,000 visited directory entries or 500 accepted
images. A limited result is labeled as such and is not presented as a complete library scan.

## Safety guarantees

- Source files are opened only for metadata and format detection during discovery; full decoding is
  deferred until a gallery item requests a preview.
- Source files are never renamed, moved, deleted, or rewritten.
- Catalogs and previews are stored in operating-system application data and cache directories, not
  inside imported folders.
- A cancelled, detached, or failed scan never replaces the last completed catalog for a source.
- Unexpectedly interrupted `running` scans resume automatically, `paused` scans wait for an explicit
  resume action, and user cancellation remains terminal.
- A file changed by another process makes the scan stale instead of publishing mixed-time state.
- A file whose Windows identity changes during a scan is treated as replaced, even when size and
  modification time still match.
- OneDrive and other files marked offline or recall-on-access are recorded as issues and skipped.
- Locked files are isolated as per-item open failures, and missing or changed files prevent the
  incomplete scan from being published.
- The Windows runner is long-path aware, and the scanner is verified with a source path longer than
  260 characters.
- Image decoding has explicit dimension and allocation limits.
- Raw EXIF parsing has a 4 MiB limit, retained capture fields have a 64-byte limit, and malformed
  metadata remains an isolated issue rather than invalidating a readable image.
- Storage paths that overlap an imported source root are rejected.
- Existing catalogs cannot be relocated without a future explicit migration workflow, and storage
  updates never move or delete existing data automatically.

## Architecture

The code is split into Ame-owned boundaries:

- `rust/src/domain`: stable scan DTOs and structured errors;
- `rust/src/application`: scan orchestration, cancellation, budgets, and publication policy;
- `rust/src/ports.rs`: catalog, media-inspection, metadata, and preview contracts;
- `rust/src/adapters`: filesystem, SQLite, and preview-cache implementations;
- `rust/src/api`: the narrow desktop bridge surface;
- `lib/app`: bootstrap, shared application presentation, and desktop window integration;
- `lib/features/library/domain`: Flutter-owned presentation models;
- `lib/features/library/application`: catalog and scan bridge mapping plus Riverpod orchestration;
- `lib/features/library/adapters`: directory selection and Windows library platform integration;
- `lib/features/library/presentation`: Material 3 gallery states and lazy rendering;
- `lib/features/settings`: typed preferences, platform persistence, and Material 3 settings;
- `lib/features/storage`: storage bridge mapping and storage-domain contracts;
- `lib/prototypes`: isolated validation applications that production code does not import;
- `test`: tests arranged to mirror their owning Dart source area.

The complete ownership map is recorded in
[`docs/development/repository-layout.md`](docs/development/repository-layout.md). Accepted technical
decisions are indexed under [`docs/architecture`](docs/architecture/README.md).

## Lap reference policy

[Lap](https://github.com/julyx10/lap) is an external GPL product and implementation reference. Ame
uses it to compare workflows, performance risks, and failure behavior. Ame does not vendor, link,
copy, or adapt Lap source code, UI components, assets, schema, or other implementation material.

The first risk carried over from reference testing is explicit: a large-library scan must be
bounded and recoverable. Ame therefore starts with entry and accepted-image budgets, per-file issue
isolation, cancellation, and atomic publication before attempting the real 259 GB collection.

## Development setup

Required tools:

- Flutter stable with Windows desktop enabled;
- stable Rust with the `x86_64-pc-windows-msvc` target;
- Visual Studio 2022 with **Desktop development with C++**, MSVC, Windows SDK, and CMake tools;
- `flutter_rust_bridge_codegen` 2.12.0;
- `cargo-expand` for bridge generation.

Generate the bridge after changing public Rust API types:

```powershell
.\tool\generate_bridge.ps1
```

Run the verified checks serially:

```powershell
.\tool\verify.ps1
```

Run the manual 10,000-file synthetic scan, pause/resume, cancellation, catalog-growth, source-byte,
and peak-working-set acceptance gate:

```powershell
.\tool\benchmark_synthetic_library.ps1
```

The benchmark enforces 60-second cold, warm, and resumed scan limits, five-second pause and cancel
limits, a 64 MiB catalog-file limit, and a 512 MiB test-process working-set limit. These are local
debug-build regression gates, not release throughput claims.

Run the controlled guard and terminal-state regression for the prepared real-library acceptance
tool:

```powershell
.\tool\test_read_only_acceptance.ps1
```

The real-root tool must not be run merely because it exists. Its explicit authorization, isolated
storage, OneDrive acknowledgement, retained evidence, interruption recovery, and ordered execution
contract are documented in
[`docs/acceptance/read-only-real-library.md`](docs/acceptance/read-only-real-library.md).

Run the Windows end-to-end acceptance test serially with isolated catalog and cache storage:

```powershell
.\tool\test_windows_integration.ps1
```

This test opens the production directory picker, cancels it once, then imports two controlled
fixture directories through the real picker. It verifies corrupt-file isolation, external catalog
and preview placement, multi-root persistence, versioned metadata evidence,
application-state reconstruction, rendered results, and unchanged source bytes. Test storage is
removed when the script exits.

Build and run the Windows application after Visual Studio reports the required components:

```powershell
flutter doctor -v
flutter run -d windows
```

## Current limitations

- Catalog rows load progressively in bounded 500-location keyset windows. Preview generation uses a
  two-worker visible-item queue; already active decode work is not preempted when a tile scrolls away.
- Recovery replays at most the current directory up to its saved entry checkpoint. Completed and
  pending directories plus directory-entry snapshots are tracked durably in 256-entry batches and
  read through 256-entry keyset windows.
- Catalog relocation, preview migration, and old-cache cleanup are not automated. Preview-location
  changes apply after restart and existing cache roots must be retained until a verified cleanup
  workflow is implemented.
- The preview budget currently uses admission control without least-recently-used eviction.
- The preview adapter currently covers BMP, GIF, ICO, JPEG, PNG, TIFF, and WebP. HEIF/HEIC and AVIF
  need a separately evaluated decoder adapter.
- Windows file identity is local reconciliation evidence only. It may be unavailable or reused by a
  filesystem and cannot establish byte identity, cross-computer identity, or permission for a file
  operation. Cross-volume moves remain new assets until exact content evidence is available.
- The synthetic large-library gate uses 10,000 tiny local PNG fixtures. It exercises bounded catalog
  behavior and recovery but does not represent decoder cost, cloud availability, or the media mix of
  the real 259 GB collection.
- Exact duplicates, date-grouped browsing, broader metadata display, search, categories, and the
  time rail are not part of this first validation slice.
- No controlled real-library acceptance scan has been run by Ame.
