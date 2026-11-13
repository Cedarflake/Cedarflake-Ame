# ADR 0001: Flutter Material 3 with a Rust application core

- Status: Accepted
- Date: 2026-08-07
- Last amended: 2026-11-08

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
- Flutter's first-party `material_ui` components and Material 3 theme system;
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

## Viewer image clipboard

An explicit viewer copy uses the Ame-owned `LibraryImageClipboard` port and a Windows adapter.
Flutter's text clipboard API cannot publish images. Flutter encodes one decoded frame after applying
the viewing rotation, using its built-in PNG encoder. The adapter sends equivalent PNG and
straight-alpha RGBA representations to the runner's `cedarflake_ame/image_clipboard` channel;
the runner publishes registered `PNG` and top-down sRGB `CF_DIBV5` clipboard formats.
PNG preserves transparency in consumers that ignore the bitmap's alpha channel.
This bounded OS transfer is separate from the Rust catalog/preview bridge. It does not add a
second source read, external encoder, persistent cache, model dependency or file-copy workflow.

One copy may be in flight per viewer, with a 32-megapixel limit checked before encoding and
again at the native boundary. Oversized images fail explicitly instead of silently reducing
quality. Switching or closing retires unresolved frame requests. The runner allocates and
validates its bitmap before opening the clipboard, releases failures, and transfers allocation
ownership only after successful `SetClipboardData`. The viewer reports success only after that
write returns. Clipboard contention remains a retryable user action without a blocking retry loop.

Validation covers pixel/rotation correctness, successful paste into a Windows image consumer,
failure feedback and reuse of the existing decoded image. The adapter and runner channel can be
replaced by a framework image-clipboard implementation without changing the viewer contract.
See Microsoft's [clipboard ownership rules](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setclipboarddata)
and [bitmap formats](https://learn.microsoft.com/en-us/windows/win32/dataxchg/clipboard-formats).

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

## Standalone Material library

Pin the official `material_ui` package to 1.11.0; the dependency lock also records its required
`cupertino_ui` 1.1.6 dependency. Both retain Flutter's BSD-3-Clause license. Material components,
themes and translated delegates come from `package:material_ui/material_ui.dart`; framework
foundation, widgets, rendering and input primitives remain in the Flutter SDK. New UI libraries
remain presentation dependencies and cannot enter domain, application, ports or adapters.

The independent package receives component fixes without an SDK upgrade. It replaces the frozen
in-framework Material library while preserving Ame's accepted Material 3 appearance, shared menus,
tooltips, window chrome and desktop interaction. Expressive styling and replacement of the retained
Windows overlay adaptations require their own affected interaction evidence.

`ameLocalizationsDelegates` uses `GlobalMaterialLocalizations.delegates` from the standalone library,
including its Cupertino and framework delegates. A legacy dependency that renders Material widgets
may use the official compatibility bridge at that subtree; the bridge cannot convert incompatible
Material types in a dependency's public API. The currently used Riverpod consumers and window-manager
gesture/platform APIs do not require that bridge.

Keep the application's existing Dart language mode. The package constraints and Flutter minimum
enforce a compatible runtime SDK; raising the application's language version also affects generated
Freezed syntax and is a separate generator/toolchain change. Migration uses the official
`dart fix --apply --code=migrate_design_widgets`, followed by explicit package pinning and delegate
updates. Rollback restores imports, delegates and the dependency lock together; Rust contracts,
catalogs and original media are unchanged. The [migration evidence](../acceptance/material-ui-migration.md)
owns actual compatibility and desktop results.

### Loading indicator compatibility

`AmeLoadingIndicator` temporarily adapts the indeterminate renderer from
[flutter/packages PR 13429](https://github.com/flutter/packages/pull/13429) at
`9eaa17df4e0ba4b067a7203e857953e3d4bc04bd`. The installed `material_ui` 1.11.0 exports
`MaterialShapes` and `MaterialShapeBorder`, but not `LoadingIndicator` or its internal `Morph`.
Use public shape-border interpolation, its bounded morph cache and the upstream morph/rotation
timing instead of private imports, another animation dependency or a package fork.
Keep the BSD-3-Clause attribution in source and the packaged dependency-license registry.

The temporary port supports only indeterminate feedback. `AmeProgressVisibility` retains delay and
fade ownership; the renderer stops its clock in inactive/reduced-motion scopes and shares immutable
shape definitions across instances. Existing small circular and determinate progress controls remain
available. Short operations, retained-content updates and content/timeline priority are owned by
the [feedback contract](../product/gallery/feedback.md#loading-visibility-and-geometry).

The upstream PR is not a released API. Keep this renderer behind the shared Ame boundary and replace
it with the stable official widget once available, removing the temporary implementation and notice.
Check the affected light/dark, reduced-motion and loading-lifetime paths before that substitution.
Rollback restores circular feedback at the same boundary without changing request execution.

### State motion

Use the admitted package's Standard `Springs` with Flutter's `SpringSimulation` and bounded
`AnimationController`. `AmeMotion` selects only the spatial/effects roles used by current consumers,
retains the controller's displayed value and velocity on reversal, and snaps settled simulations to
their exact endpoint. Opening and closing retain the framework's completed/dismissed lifecycle.
`AmeStateTransition` handles decorative binary state and reads current reduced-motion/TickerMode
settings; feature controllers retain surface lifetime, input retirement and layout ownership.

Folder icons crossfade; expansion arrows rotate through the shared presentation components.
`AmeFlipTransition` owns decorative front/back switching for group thumbnails and sidebar selection.
It follows the draw-only Y-axis rotation in the
[Compose graphics API](https://developer.android.com/develop/ui/compose/graphics/draw/modifiers#rotation)
and the edge-on face exchange in Android's
[card-flip example](https://developer.android.com/develop/ui/views/animations/reveal-or-hide-view#CreateFlip).
The pinned `IconButton` swaps selected content immediately, and `AnimatedSwitcher` defaults to fading;
neither supplies this flip. Compose the existing state driver with Flutter `Transform` and
`IndexedStack`, retaining both faces and stable layout. Material buttons own input, selection and
semantics outside the decorative transform. This adds no dependency or copied Android implementation.
Review controls use separate spatial and effects progress so opacity cannot inherit a spatial
overshoot. Existing Material controls, themes and desktop overlay ownership remain in place.
Future official component integration replaces the corresponding driver or removes the adapter,
without moving feature state or introducing another motion engine. The
[motion evidence](../acceptance/material-motion.md) records admitted consumers and verification.

`LibraryViewerLaunch` captures one activated thumbnail's bounds and identity. The shared viewer
surface/route passes that origin with its existing animation to `LibraryViewerImageFlight`.
Flutter's `Hero` requires a Navigator route pair; Ame's gallery viewer is an in-place surface that
retains gallery geometry and owns immediate source retirement. Replacing that lifecycle solely for
motion would broaden navigation behavior. A small `RenderProxyBox` therefore composes framework
clip and transform painting around the current image. It interpolates the crop and uniformly scales
pixels without changing layout, decoding another image or moving controls. The installed SDK
provides these public rendering primitives and `HeroController` remains a `NavigatorObserver`.
The surface registry holds geometry suppliers for mounted lazy thumbnails, never pixels or catalog
rows. Return geometry resolves the current image to a mounted, visible thumbnail, including after
adjacent navigation; a missing/stale anchor uses the fade. Disposal removes that supplier immediately.
`LibraryViewerChrome` fades backgrounds and controls separately so the travelling image remains
opaque. The interaction clip opens only during transition painting and resumes for ordinary zoom.
This presentation-only adapter can be replaced by an official in-place shared-element component
without changing source, zoom, navigation or application contracts.

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

## Windows animation scheduling

The Windows runner selects the pinned SDK's public `UIThreadPolicy::RunOnSeparateThread`.
Dart frame callbacks run independently of the native window message loop; platform plugins still
receive their calls on the platform thread. The renderer, frame rate, animation simulations and
image/source lifetimes remain unchanged. This introduces no custom timer, process priority change
or engine patch.

With Flutter 3.47.8, the default merged loop delayed callbacks during card enlargement even when
build and raster work fit the display budget. The same Profile interaction with separate threads
removed the recurring middle-of-flight delay; the [motion record](../acceptance/material-motion.md#transition-performance--2026-11-13)
retains the comparison and remaining first-frame costs. Extending motion duration or reducing image
quality would change the accepted effect without repairing callback delivery.

Flutter retains this public switch for compatibility and emits a runtime notice that separate UI
threads will be removed in a future version. Keep that notice visible. Reassess the default loop
when upgrading the pinned SDK, including entry/return cadence, window state, keyboard input and
native plugin/accessibility workflows. Remove this selection when the default matches those
results; an SDK that removes the API requires an explicit migration rather than a silent fallback.

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
- <https://docs.flutter.dev/release/breaking-changes/material-ui-and-cupertino-ui>
- <https://pub.dev/packages/material_ui/versions/1.11.0>
- <https://api.flutter.dev/flutter/widgets/SliverGrid-class.html>
- <https://github.com/fzyzcjy/flutter_rust_bridge>
- <https://github.com/material-components/material-web>
- <https://mui.com/material-ui/>
