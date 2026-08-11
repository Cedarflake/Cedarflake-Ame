# ADR 0004: Admit narrow dependencies for the R0 vertical slice

- Status: Accepted
- Date: 2026-08-07

## Context

R0 needs a typed Dart/Rust bridge, Flutter state management, an operating-system directory picker,
SQLite persistence, bounded raster decoding, path traversal, application storage discovery, and
stable derived identifiers. Implementing those established capabilities inside Ame would add risk
without improving the product workflow.

Dependencies must remain behind Ame-owned contracts. Generated bridge code, database handles,
decoder errors, and package-specific state must not enter the Rust domain or Flutter presentation
models.

## Accepted dependencies

| Capability | Dependency | Version | License | Admission reason |
| --- | --- | --- | --- | --- |
| Typed Dart/Rust bridge | `flutter_rust_bridge` | 2.12.0 | MIT | Stable v2 release, active project, generated typed streams and errors, narrow replaceable boundary |
| Flutter state | `flutter_riverpod` | 3.4.2 | MIT | Mature testable state container; used only for presentation and interaction state |
| Directory picker | `file_selector` | 1.1.0 | BSD-3-Clause | Flutter-maintained desktop plugin with a small platform capability surface |
| SQLite | `rusqlite` | 0.40.1 | MIT | Mature direct SQLite binding, explicit transactions, bundled library for predictable packaging |
| Raster previews | `image` | 0.25.10 | MIT or Apache-2.0 | Pure Rust decoders, explicit allocation and dimension limits, selectable format features |
| Large JPEG previews | `jpeg-decoder` | 0.3.2 | MIT or Apache-2.0 | Decoder-level DCT scaling behind the preview adapter, without adding a native runtime |
| Application paths | `directories` | 6.0.0 | MIT or Apache-2.0 | Cross-platform application data and cache directory discovery |
| Derived identifiers | `blake3` | 1.8.6 | CC0-1.0 or Apache-2.0 | Versioned fast stable identifiers without using absolute paths as asset identity |

The Flutter, Riverpod, file-selector, bridge, SQLite, image, and jpeg-decoder license texts were
inspected from the resolved local packages before this decision was recorded. Transitive
dependencies remain captured in `pubspec.lock` and `rust/Cargo.lock` and require review before
distribution.

## Considered alternatives

### Manual Dart FFI

Manual FFI would reduce one generator dependency but would require Ame to own serialization,
streaming, error conversion, and ABI compatibility. It is retained as a bridge replacement option,
not selected for R0.

### `file_picker`

`file_picker` supports more platforms and features. R0 needs only one native directory choice, so
Flutter's narrower maintained `file_selector` package reduces platform surface.

### SQLx or an ORM

Async database layers are useful for remote databases and larger mapping domains. Ame currently
needs explicit local SQLite transactions and query control; `rusqlite` is smaller and keeps the
application policy visible.

### Native imaging libraries

libvips, ImageMagick, and platform codecs support more formats and may be faster. They also add
native packaging and crash-isolation requirements. R0 admits only selected pure Rust decoders while
the supervised worker boundary remains unimplemented.

## Boundary rules

- `flutter_rust_bridge` generated types exist only in `rust/src/frb_generated.rs` and
  `lib/src/rust`; Ame maps them at the API and Flutter scanner adapters.
- Riverpod does not own catalog or scan policy.
- `file_selector` is wrapped by `DirectoryPicker`.
- `rusqlite`, `image`, `jpeg-decoder`, and `directories` remain inside Rust adapters or application
  composition.
- Image default features are disabled. Only BMP, GIF, ICO, JPEG, PNG, TIFF, and WebP are compiled
  into the R0 adapter.
- jpeg-decoder default features are disabled so its optional Rayon pool cannot introduce nested
  preview concurrency; unsupported color models and decode failures fall back to the existing
  `image` path.

## Validation evidence

- Rust compilation and Clippy with warnings denied pass on stable Rust 1.97.1.
- Rust tests include a Chinese-path raster fixture and verify that source bytes are unchanged,
  previews remain outside the source tree, and a completed scan becomes the active catalog.
- Flutter analysis and controller/widget tests pass with strict casts, inference, and raw types.
- The generated bridge produces typed scan events, cancellation, and structured scan errors.
- A release Rust dynamic library builds successfully.
- Windows Debug integration tests exercise the real `file_selector` dialog, typed Rust bridge,
  raster preview adapter, SQLite publication, and Riverpod presentation flow end to end.
- Visual Studio C++ desktop and CMake components are registered, and both Debug integration tests
  and a Windows Release runner launch pass on the admitted toolchain.

## Consequences and risks

- Public Rust API changes require bridge regeneration and generated-code review.
- Bundled SQLite increases binary size but removes a system SQLite dependency.
- In-process pure Rust decoders still require hostile-fixture and memory-budget testing.
- jpeg-decoder is in maintenance mode, so it remains a private optimization that can be removed
  without changing preview cache identity or application contracts.
- The current adapter intentionally reports HEIF/HEIC and AVIF as unsupported rather than admitting
  a native codec prematurely.
- Package upgrades require contract tests, license review, and regenerated bridge artifacts.

## Replacement strategy

Each dependency can be replaced at its admitted boundary. A bridge replacement preserves Rust
application use cases and Flutter domain models. A persistence or preview replacement preserves
catalog semantics and cache identity versioning. A directory-picker replacement preserves the
`DirectoryPicker` contract. No dependency-specific schema or type becomes Ame's public model.
