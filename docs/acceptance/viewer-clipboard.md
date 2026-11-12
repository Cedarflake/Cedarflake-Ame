# Viewer image clipboard verification

Date: 2026-11-12

Scope: the single-image viewer's `Ctrl+C`, More-menu image copy and transient result notice.
The [viewer contract](../product/gallery/image-viewer.md) owns behavior;
[ADR 0001](../architecture/0001-flutter-rust-desktop-stack.md#viewer-image-clipboard) owns the
bounded Windows transfer. Viewing rotation is included; zoom/pan do not crop the copied image.

## Observed results

- An isolated viewer used the production viewer, platform binding, channel and Windows runner,
  with a decoded 600 × 400 fixture supplied through the existing image cache. No catalog or
  source-media reads were involved.
- Menu copy displayed `图片已复制到剪贴板` after the native write. The clipboard contained a
  600 × 400 bitmap with matching corner colors, alpha 128 in the translucent region and alpha 0
  in the transparent region.
- After a clockwise quarter-turn, copy produced 400 × 600 pixels with the corresponding corners.
  Pasting into Microsoft Paint preserved the rotation and transparency. Registered PNG is
  published before the bitmap fallback because Paint ignored alpha when only CF_DIBV5 was offered.
- The focused viewer regression checks decoded pixels, the `Ctrl+C` binding, one in-flight write,
  menu copy after rotation, failure/retry and success-only feedback with its shared lifetime.
  Its unavailable source-reader fixture also rejects an extra original-file read.
- Native shortcut automation supplied the same missing scan code for Ctrl and C. Flutter
  synthesized a modifier release before C; that input did not exercise a real simultaneous
  Ctrl+C. Hardware-key use remains unobserved; the binding is covered by the focused regression.

## Applicable checks

- `quality_test_flutter.ps1` for `library_image_viewer_test.dart` and
  `ame_provider_assembly_test.dart`: 19 passing cases.
- Scoped Dart formatting and analysis with informational diagnostics fatal: passed.
- `quality_verify_dependencies.ps1`: passed, 606 modules and no frozen migration exceptions.
- Windows Debug production/viewer builds and native image paste: passed.
- `integration_test_windows_runner.ps1`: three runner lifecycle and two real-engine retirement
  cases passed, including retirement of the clipboard channel before the engine.

The temporary viewer entrypoint, pixels and keyboard diagnostics remain outside tracked source.
No dependency, source mutation, migration or file copy/cut/paste workflow was introduced.

## Review follow-up

The copy notice now participates in the viewer's existing feedback selection. A newer organization
result or a new file operation retires it, including persistent copy errors. The focused viewer case
checks that a later Favorite result appears and its dismissal does not reveal the old copy error.
Viewer and organization-widget suites passed all 34 cases; scoped formatting and analysis passed.
The native clipboard path is unchanged, retaining the paste and runner-lifecycle evidence above.
