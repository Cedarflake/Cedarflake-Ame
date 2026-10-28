# ADR 0001: Flutter Material 3 with a Rust application core

- Status: Accepted
- Date: 2026-08-07
- Last amended: 2026-10-28

## Context

Ame is a Windows-first local photo-library application. It needs a polished Material 3 interface,
continuous lazy gallery rendering, a Rust-owned large-library core, typed progress streams, and a
maintainable boundary between presentation and filesystem or analysis work.

The repository is empty, so this decision is not constrained by existing application code. Lap uses
Rust, Tauri, Vue, and SQLite, but it is a GPL reference application rather than an Ame dependency.
Matching Lap's framework would not create legal or implementation reuse.

## Decision drivers

- first-party Material 3 components and theming;
- stable Windows desktop support;
- lazy sliver primitives suitable for a continuous photo wall;
- a strict Rust domain and application boundary;
- typed asynchronous progress and cancellation;
- testability and replaceability of the desktop bridge;
- long-term maintainability by a small project.

## Considered options

### Flutter and Rust

Flutter provides first-party Material 3 and lazy sliver primitives. Rust remains responsible for the
catalog, filesystem discovery, task orchestration, persistence, and analysis. The integration cost is
an additional generated FFI boundary and a larger Flutter toolchain.

### Tauri, React, and Material UI

Tauri provides a natural Rust command boundary and convenient Windows packaging. Material UI is a
mature React library but does not currently provide a complete Material 3 implementation. A large
photo wall also requires explicit DOM virtualization and WebView2-specific performance testing.

### Tauri, Vue, and Material Web

Vue is productive and close to Lap's frontend stack, but Ame cannot reuse Lap's GPL components.
Google's Material Web component project is in maintenance mode and is not a suitable foundational
dependency for a new long-lived application.

### Rust-native UI frameworks

Current Rust-native toolkits do not provide a comparably mature Material 3 component and desktop
accessibility ecosystem for this product.

## Decision

Use:

- Flutter stable and Dart for the Windows desktop presentation layer;
- Flutter's first-party Material 3 components and theme system;
- Riverpod for presentation and ephemeral interaction state only;
- a Rust workspace for domain, application, persistence, task, and adapter code;
- `flutter_rust_bridge` behind a narrow generated bridge layer;
- Windows 11 x64 as the first supported and measured target.

The Rust core must compile and test independently from Flutter. Generated bridge types must not enter
domain or application crates. Flutter must not access SQLite, enumerate source directories, compute
duplicate identity, or run media analysis directly.

Bridge calls exchange bounded Ame-owned DTOs. Preview and full-media bytes do not cross FFI as
unbounded buffers; the catalog returns identities, metadata, and paths to bounded derived previews.

Dependency versions are locked after the selected toolchain passes its applicable client and hosted
checks. The admitted Windows SDK and renderer are specified below.

## Windows SDK and renderer

Pin Flutter stable 3.47.8 with Dart 3.13.7. The framework revision is
`737ba8f885dc774b53b442759999a312c9461e4e`, and the matching engine revision is
`cf636b1dfb818dd6a3ff16e390e93b1a036969c8`. Keep the hosted workflow pins, local toolchain and
dependency lock consistent. Flutter retains its BSD-3-Clause license; this upgrade introduces no
additional renderer dependency or source-media authority.

Flutter 3.47 makes Windows Impeller available by default. Ame explicitly selects Skia using
`DartProject::set_impeller_switch(ImpellerSwitch::Disabled)`. The matched stable SDK displays the
gallery and viewer correctly on local-primary and cloud-primary, but Impeller increases measured
transition cost and memory use on cloud-primary's Microsoft Basic Render adapter. Skia preserves
the usable software-rendered path. There is no device-name heuristic, engine patch, new setting or
automatic renderer retry.

Complete engine, runner, Dart payload and assets must come from the same SDK and build mode. Retain
real-pixel checks alongside timing and logical-state assertions: the previous experimental Windows
Impeller probe recorded low frame costs while displaying a black client area. The
[R4 evidence](../acceptance/r4-organization.md#stable-sdk-and-renderer-selection---2026-10-28)
owns the actual comparisons, native-library provenance and verification coverage. Earlier ADR SDK
version references identify the framework sources inspected for those decisions.

The public switch is the renderer rollback boundary. A future measured improvement can remove the
explicit disablement after the affected Windows clients pass. If the SDK itself regresses, restore
the previous 3.44.9 pin and dependency lock together, remove the new public switch call, and rebuild
the complete runtime; never mix engine DLLs and Dart payloads between SDK versions. The previous SDK
revision remains available locally. Preserve the image I/O scheduling and native lifecycle checks
when changing either path.

## Validation gates

This decision becomes Accepted only after R0 demonstrates:

- a Windows debug and release-mode launch;
- a real directory choice reaching a Rust use case;
- typed progress and cancellation flowing back to Flutter;
- a Rust-owned SQLite catalog and preview cache outside the source directory;
- a lazy Material 3 gallery rendering real indexed records;
- empty, partial-error, cancelled, and completed states;
- tests and static analysis on both sides of the bridge;
- no source-media mutation.

## Validation evidence

- Windows Debug integration tests launch the real runner, open and cancel the production directory
  picker, then select a controlled directory through the native picker and complete the Rust scan.
- The completed workflow renders a real preview, publishes a Rust-owned SQLite catalog outside the
  source, isolates a corrupt image, and verifies that source bytes and entries remain unchanged.
- Rust tests cover typed progress, cancellation, stale-source publication rejection, corrupt media,
  wrong extensions, Chinese paths, and external preview placement.
- Rust formatting, Clippy with warnings denied, Rust tests, Flutter analysis, Flutter tests, and the
  Windows integration tests pass on the admitted toolchain.
- The Windows Release runner builds and launches with `flutter run --release --no-resident` without
  exiting early.

## Windows image I/O scheduling

The Windows runner raises Flutter's image I/O thread from `BELOW_NORMAL` to
`NORMAL` once, immediately after synchronous engine construction and before plugin
registration. This prevents foreground image uploads from waiting behind ordinary
CPU work on a two-core device. The process priority and existing worker priorities
are unchanged.

The pinned Flutter Windows interface does not expose the engine's thread-priority
callback. The runner therefore observes its own thread IDs before construction,
then admits exactly one newly created, still-live `BELOW_NORMAL` thread. It holds
the admitted handle through revalidation and adjustment. Missing, ambiguous or
failed observations leave priorities unchanged and emit a startup diagnostic;
there is no polling, thread suspension, engine binary patch or global OS setting.

This adapter depends on the pinned engine's startup contract. The native engine
fixture independently observes the engine's `io.flutter.io` naming event and
requires that exact thread to have normal priority. SDK upgrades must retain this
check. Replace the snapshot adapter with a public priority callback if Flutter
provides one; removing the adapter restores Flutter's scheduling defaults.

## Consequences

- The project uses Dart and Rust and must maintain a generated boundary.
- Flutter tooling and Windows C++ build prerequisites are required.
- Material 3 does not depend on a third-party web component project.
- Gallery virtualization uses Flutter slivers rather than DOM virtualization.
- Platform-specific features remain behind Rust or Flutter adapters rather than entering the domain.

## Replacement strategy

If R0 fails a gate because of a reproducible framework or bridge limitation, record the evidence in a
superseding ADR. The Rust domain, application contracts, migrations, and adapter tests must be
preserved. Tauri with a typed Rust command boundary is the primary alternative, but it is not adopted
without measured failure evidence.

## References

- <https://docs.flutter.dev/platform-integration/desktop>
- <https://docs.flutter.dev/perf/impeller#windows>
- <https://docs.flutter.dev/cookbook/design/themes>
- <https://api.flutter.dev/flutter/widgets/SliverGrid-class.html>
- <https://github.com/fzyzcjy/flutter_rust_bridge>
- <https://github.com/material-components/material-web>
- <https://mui.com/material-ui/>
