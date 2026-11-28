![Cedarflake Ame](docs/images/ame.png)

<p align="center">English | <a href="./README.zh-Hans.md">简体中文</a> | <a href="./README.ja.md">日本語</a></p>

Ame is a local-first Windows photo library for everyday use, with image browsing, intelligent
search, automatic classification, and duplicate cleanup. It works with existing image folders and
supports timeline browsing, favorites, albums, and file management.
It is free and open source under [Apache-2.0](LICENSE).

![Ame gallery, image viewer, search, albums, duplicate groups, and file organization](docs/images/ame-showcase.webp)

## Download

Ame is currently in **Alpha**. Its main features are implemented, with performance and stability
being refined for Beta. Each [release](https://github.com/Cedarflake/Cedarflake-Ame/releases) provides
two packages for **Windows 11 x64**:

- **MSI installer**: includes the image-analysis models, runtime, and background library-update
  service. Installation requires administrator permissions.
- **Portable ZIP**: runs without installation. Image-analysis models download when first needed;
  no Windows service is installed.

The [Windows 11 previews](https://github.com/Cedarflake/Cedarflake-Ame/actions/workflows/quality_windows_arm64.yml)
include MSI packages for x64 and ARM64 and a portable ZIP for ARM64. Switching from an installed
x64 MSI to ARM64 requires uninstalling the x64 version first.

Ame supports in-app updates, with automatic checking and downloading options. Older builds without
the updater require a manual upgrade.
Release and preview packages are unsigned. See the [release notes](docs/releases/README.md)
for changes and known limits.

## Features

- **Library browsing**: photo wall, square grid, timeline, and multiple folders in one library.
- **Automatic synchronization**: file-change monitoring and background library updates.
- **Image search**: filename, path, natural-language and image-based search, with combined filters.
- **Smart albums**: local classification using SigLIP 2, with GPU acceleration on supported devices.
- **Favorites and albums**: favorites, virtual albums, custom album icons, and saved organization progress.
- **Duplicate cleanup**: exact and visually similar groups, comparison, adjustable similarity,
  and group cleanup.
- **File management**: copying, moving, renaming, deletion, operation history, and recovery of
  supported interrupted operations.
- **Image viewing**: zoom, rotation, side-by-side comparison, and thumbnail navigation.

## Run from source

Ame is still in development. To build it, use Windows 11 x64, Flutter **3.50.0**, the Rust toolchain
in [rust-toolchain.toml](rust-toolchain.toml), and Visual Studio 2022 with the **Desktop development
with C++** workload, Windows SDK, and CMake tools.
For Windows on Arm, see the [ARM64 build instructions](docs/acceptance/quality/commands.md#build-on-windows-arm64).

```powershell
flutter pub get
flutter run -d windows
```

The interface is built with Flutter and Material 3. Rust handles the catalog, file tracking,
previews, image analysis, and file operations, with SQLite storing the library.

For more detail, see the [roadmap](docs/roadmap.md),
[development guide](docs/development/repository-layout.md),
[build and test commands](docs/acceptance/quality-gates.md), or the
[architecture notes](docs/architecture/README.md).

## License

Copyright 2026 Cedarflake Ame contributors.

Except where otherwise noted, Cedarflake Ame is licensed under the
[Apache License, Version 2.0](LICENSE) (`Apache-2.0`).
Third-party code, dependencies, model weights, and resources retain their own licenses and notices.

End-user terms are described in [EULA.md](EULA.md).
Information handling is described in the [Privacy Statement](PRIVACY.md).
