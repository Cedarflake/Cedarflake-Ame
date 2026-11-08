# Standalone Material migration

Date: 2026-11-08

The [roadmap](../roadmap.md#current-execution-queue) owns delivery status;
[ADR 0001](../architecture/0001-flutter-rust-desktop-stack.md#standalone-material-library) owns the
package, compatibility and rollback decision.

## Scope and provenance

Baseline: `27bde80b`; branch: `codex/material-ui-migration`. The official installed Dart fix
`migrate_design_widgets` previewed and then migrated 312 files: 150 production files, 149 test files
and 13 integration files. Its unconstrained dependency was replaced with `material_ui` 1.10.0;
the resolved Cupertino dependency is 1.1.6. Both installed packages retain BSD-3-Clause notices.
Other package versions, Flutter 3.47.8/Dart 3.13.7, Skia, native contracts and storage are unchanged.

The shared Chinese localization delegates now come from the standalone Material package. Used
Riverpod consumers and window-manager gesture/platform APIs do not read legacy Material theme or
localization state. No compatibility bridge or replacement control was introduced.

An attempted application language-version increase to Dart 3.13 changed unrelated formatting and
rejected an existing generated Freezed formal parameter. The application retains its prior language
mode; the new package constraints and Flutter minimum enforce the compatible SDK. Generated output
was not edited. Only owned files were formatted after restoring that language mode.

## Verification

The presentation static entrypoint passed: formatting, dependency checks for 547 modules with zero
frozen migration exceptions, and Dart analysis with no issues. The existing dependency fixture now
admits both standalone libraries in presentation and rejects them in adapters, including a private
Cupertino import. Existing core-layer rejection and composition policy remain.

All 56 focused checks passed across the existing application/localization, theme, menu, checkbox,
filter, search and dependency-boundary suites. The 11 existing image-viewer checks also passed,
including wheel animation, actual-size persistence and panning bounds. Diff review found 310 Dart
files changed only by import replacement and formatting; the remaining Dart owners are localization and the boundary
rule/fixture. No generated bridge or native source changed.

The first Windows build stopped at the existing memory guard during Rust compilation. The previous
successful build's complete input digest was verified against the unchanged baseline, and both
native DLL copies matched its `24f7f44d` SHA-256 prefix and isolated-storage configuration. The current
Flutter payload was then rebuilt with `flutter_assemble` and installed into that verified Debug
bundle under the same resource guard. This local check reuses native output; fresh complete builds
remain the hosted CI owner's responsibility.

The isolated native client passed dark/light theme switching, shared menus, the bottom filter tray,
Chinese date selection, applying an empty date result and clearing back to 368 images, checkbox
selection and Escape, viewer next-image/Escape, and maximize/restore/minimize. Windows IME candidate
composition retained the gallery; committing the Chinese text submitted the search. In this
intentionally model-free profile, acknowledging the missing-model feedback cleared the search.
The client closed normally with exit code 0; all 368 retained preview hashes were unchanged, and
the scratch profile was retired. No Flutter exception or layout-overflow diagnostic was logged.

A second isolated run at `ab512fd8` assigned quarterly dates from 2022 through 2025 to 368 temporary
preview copies. Timeline clicks and drags reached October 2022, 2024 and 2023 without reverting after
release; gallery scrolling updated the indicator. Viewer wheel zoom, slider dragging and actual-size
reset changed the rendered scale. Filmstrip dragging reached both ends without changing the main
image; selecting ordinals 365 and 1 displayed the matching image and retained the strip position.
Closing and reopening the viewer preserved the gallery location. This client also exited normally,
retired its scratch profile and preserved all 368 retained preview hashes, with no Flutter errors.

Native image-panning input did not produce an observable displacement in the automation tool. The
existing gesture/bounds test passed; sustained desktop image dragging remains unconfirmed.

## Integration disposition

[PR 54](https://github.com/Cedarflake/Cedarflake-Ame/pull/54) integrated reviewed head `da113d4d`
into `main` as `f907b033` on 2026-11-08 after explicit authorization to merge before CI completion.
The merged file tree exactly matches the reviewed head. Review against `27bde80b` found no confirmed
new defect; the desktop panning verification limit above remains.

At merge, the [implementation run](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/42448216115)
at `ab512fd8` had passed Static, Flutter Tests, Windows Scan and Windows Accessibility; Windows Build
Verification and all four Rust groups were still running. The
[PR run](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/42449217822) at `da113d4d` was also
running. Only acceptance documentation differs between these heads. Their Actions results retain
the eventual hosted outcome; local native-output reuse does not establish a fresh Windows build.
