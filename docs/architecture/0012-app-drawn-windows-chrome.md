# ADR 0012: Draw the Windows title bar inside Ame

- Status: Accepted for validation
- Date: 2026-08-08

## Context

The default Flutter Windows runner exposes the system-painted blue title bar above Ame's Material 3
shell. It is visually disconnected from the application and prevents the topmost surface from using
the same theme, spacing, and interaction states as the rest of the product.

Replacing the title bar must not mean reimplementing Win32 resizing, window state, taskbar, focus,
or drag behavior. The native frame still owns those platform responsibilities, while Flutter owns
only the visible application chrome. A separate application-title row also duplicates the identity
and controls already present in the unified library global bar, consumes vertical gallery space, and
visually splits one window into two stacked headers.

## Decision drivers

- one visually continuous Ame surface without duplicate application labels;
- correct minimize, maximize, restore, close, drag, double-click, resize, shadow, and taskbar behavior;
- Material focus, hover, tooltip, keyboard, and semantics for app-drawn controls;
- no permanent Win32 fork in the generated Flutter runner;
- a narrow, replaceable platform adapter with widget tests that do not require a native plugin.

## Considered options

### Keep the native title bar

This retains the smallest dependency surface but cannot satisfy the accepted app-drawn presentation.

### Fork the Windows runner

Handling non-client calculations, hit testing, DPI, resize borders, snapping, and Windows version
differences directly in Ame would remove a Dart dependency but create a high-risk permanent Win32
maintenance surface.

### `window_manager`

Version 0.5.2 is MIT licensed, supports Windows, is actively released, and has a widely used public
API for hidden title bars, window state, dragging, and resizing. Its Windows implementation keeps the
native window and changes the non-client presentation through documented window messages and DWM.

## Accepted decision

Admit `window_manager` 0.5.2 behind `AmeWindowActions`.

- The Windows bootstrap selects `TitleBarStyle.hidden` and hides system caption buttons.
- The normal library window uses one 64-pixel global bar. Its application identity, search,
  draggable empty regions, and minimize, maximize or restore, and close controls share that surface.
  Import remains owned by the Library row in the sidebar and Settings is pinned to the sidebar
  bottom, so neither action competes with window chrome.
- The image viewer replaces that global bar with one 64-pixel image toolbar rather than opening a
  second window or stacking another title row. The absolute-path region is draggable, while Back,
  information, More, and the same three caption controls remain separate interactive targets.
- The Windows desktop shell does not wrap this surface in Flutter `SafeArea`. The native desktop
  frame already owns screen work-area constraints, while `SafeArea` can inset app-drawn caption
  controls from the client edge when the window is maximized. The caption group ends at the right
  client boundary while the close button and its circular state layer retain an 8-pixel visual inset.
- The global bar anchors identity, search, and caption controls independently in one full-width
  stack. A loose flex allocation must not own the centered search region because once the search bar
  reaches its maximum width, unused flex space would strand the caption controls before the right
  edge on wide or maximized windows.
  Caption controls stay flush with the top-right window edge rather than creating a second title row.
- The package `DragToMoveArea` owns drag and double-click maximize behavior inside non-interactive
  regions. Ame does not recreate those gestures or place a drag target over search and buttons.
- Following Kazumi's preference for framework-owned controls, Ame uses Material `IconButton` for
  all three visible actions and does not reproduce custom Windows caption painting or copy Kazumi
  code. The pinned Flutter SDK does not yet expose a native `Symbols` class, so every caption glyph
  comes from `material_symbols_icons` 4.2960.0 (Apache-2.0), backed by Google's official Material
  Symbols fonts. The glyphs share one rounded style, weight, optical size, and fill. Their nominal
  sizes compensate for different intrinsic font bounds so their visible sizes remain balanced. The
  dependency is pinned so an upstream font refresh cannot silently change window chrome.
- `AmeWindowCaptionControls` and `AmeWindowDragRegion` are app-owned reusable chrome components. The
  unified library composes them without importing plugin types or native window commands.
- `AmeWindowFrame` remains only as the minimal fallback frame for bootstrap failures, where the
  feature-owned global bar is unavailable.
- `WindowManagerActions` is the only adapter that invokes window commands or listens to native window
  state. Presentation consumes an Ame-owned action contract and a `ValueListenable<bool>`.
- The application retains the native resize frame, window shadow, taskbar entry, keyboard focus, and
  platform window lifecycle. It does not call `setAsFrameless`.
- Ame does not intercept native close requests. Window placement is saved from bounded move, resize,
  maximize, and restore events; the close button delegates directly to the platform so preference
  persistence can never delay or block application exit.
- Startup awaits `waitUntilReadyToShow` without an asynchronous callback, then restores normal
  bounds, restores maximized state, and shows the window in that order. `window_manager` 0.5.2
  declares the optional ready callback as `VoidCallback` and does not await a returned `Future`, so
  placement work must not be scheduled inside that callback.

## Validation gates

- widget tests prove that each visible caption control invokes the matching Ame action, the maximize
  or restore presentation follows adapter state, and the normal library has only one top bar;
- Flutter analysis and the existing gallery widget suite pass;
- a Windows Release build completes with the generated plugin registration;
- runtime inspection confirms there is no native blue title bar, the app-drawn drag region works,
  double-click toggles maximize, all three Material controls work, and resize borders and shadow remain;
- constrained-width inspection confirms the fused bar remains usable at the 800-pixel minimum width
  and caption controls never overlap search.

The decision becomes Accepted after runtime interaction and visual inspection pass. A compile-only
result is insufficient.

## Consequences and risks

- startup must initialize the plugin before showing the window to avoid a native-title-bar flash;
- package and Windows-runner upgrades require a caption, resize, DPI, and snap regression check;
- app-drawn controls must preserve focus visibility, tooltips, semantics, and Windows-familiar close
  hover feedback;
- the window chrome remains independent from gallery business state even though it is composed into
  the same visual surface as global library actions;
- bootstrap failures use a separate fallback row because no feature shell exists at that point.

## Replacement strategy

`AmeWindowActions` isolates presentation from the package API. A later Flutter or native window API
may replace `window_manager` without changing the gallery, catalog, or domain. If hidden-frame behavior
regresses, restore `TitleBarStyle.normal` while retaining the rest of the application shell.
