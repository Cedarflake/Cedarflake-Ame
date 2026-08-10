# ADR 0015: Versioned Windows x64 portable distribution

- Status: Accepted
- Date: 2026-08-11

## Context

Cedarflake Ame needs a distributable Windows build before the later installer, update, rollback,
signing, and supply-chain work is ready. Flutter's Windows Release output is a directory containing
the executable, Flutter and plugin DLLs, the Rust bridge DLL, and runtime data. Shipping only the
executable would produce an unusable application.

The application currently has one primary user and one measured platform. A release pipeline does
not need a human approval environment, but it must not publish an artifact until the complete
release-candidate gate has passed. Original media, local catalogs, caches, machine-specific paths,
and identity data must never enter a release artifact.

## Decision drivers

- create one artifact that can be downloaded and run without installation;
- keep the complete Flutter Windows runtime bundle intact;
- maintain one stable application and artifact identity across later packaging work;
- reject incomplete or path-unsafe archives before publication;
- avoid ARM and installer claims that cannot be tested in the current environment;
- keep write permission out of ordinary quality and candidate-verification jobs.

## Considered options

### Portable ZIP only for the current stage

Package the complete Windows x64 Release directory beneath one `Cedarflake-Ame/` folder. This is
simple to inspect, does not require installer technology, and matches the currently measured
platform. It does not provide Start menu registration, uninstall behavior, or automatic updates.

### Portable ZIP and MSI now

An MSI would provide installation and uninstall semantics, but choosing an installer tool, stable
upgrade codes, cache cleanup behavior, and update interaction before those workflows are designed
would create compatibility commitments without acceptance evidence.

### Single executable

Flutter Windows is not a single-file runtime. Repacking or embedding all runtime dependencies would
add a custom loader and new failure modes without product value.

## Decision

Use these stable identities:

- display name: `Cedarflake Ame`;
- application namespace: `com.cedarflake.ame`;
- Windows executable: `cedarflake_ame.exe`;
- release tag: `v<semantic-version>`;
- portable artifact: `Cedarflake-Ame-<tag>-windows-x64-portable.zip`;
- archive root: `Cedarflake-Ame/`.

The semantic version in the release tag, Flutter application manifest without its build suffix,
and Rust package manifest must match. The current initial release line is `0.1.0`; the Flutter build
suffix remains build metadata and is not part of the tag or artifact name.

Only Windows x64 is supported and published. ARM packages are not produced or implied.

The portable archive contains the complete repository-built Flutter Release directory. Archive
verification rejects an unexpected filename, entries outside the single archive root, traversal or
absolute paths, duplicate paths, missing executable or core runtime DLLs, missing Rust bridge DLL,
missing ICU or application data, and missing Flutter assets.

A pushed `v*.*.*` tag runs the complete release-candidate gate. Only after that gate succeeds may a
separate job with repository-content write permission build the bundle, verify the ZIP, and publish
or update the corresponding GitHub Release. No GitHub environment or human approval is required.
The published-release workflow downloads the exact attachment and verifies it independently.

## Deferred decisions

R9 owns the following work:

- a per-user MSI alongside the portable ZIP;
- installer technology, package identifiers, and stable MSI upgrade codes;
- manual and in-application update workflows;
- database migration and compatibility policy;
- application and database rollback behavior;
- code signing, provenance, checksums, and broader supply-chain hardening.

The future uninstaller must remove Ame-owned cache, thumbnail, and temporary files. It must never
delete or alter original media. Durable catalogs, user decisions, and operation history require a
separate explicit retention decision before installer implementation; this ADR does not classify
them as disposable cache.

## Consequences and risks

- Users extract one directory and run `cedarflake_ame.exe` from that directory.
- Moving the directory is supported; removing it does not clean application data.
- The ZIP is larger than the executable because it intentionally includes all runtime dependencies.
- The current artifact has no signature, installer registration, automatic update, or rollback.
- GitHub Release publication has write permission only after the read-only candidate gate succeeds.
- A workflow rerun may replace the named attachment on a mutable release; immutable-release policy
  and signed provenance remain R9 work.

## Validation evidence

- `tool/release_package_portable_windows.ps1` copies and archives the complete Release directory.
- `tool/release_verify_portable_archive.ps1` validates identity, structure, and required runtime
  entries without extracting the archive.
- `tool/release_test_portable_archive.ps1` proves a valid fixture passes and incomplete or
  traversal-bearing archives fail.
- `.github/workflows/release_candidate_windows.yml` gates publication behind release verification.
- `.github/workflows/release_verify_published.yml` downloads and validates the published attachment.

## Replacement and rollback strategy

The ZIP packager is independent of installer technology. R9 may add an MSI produced from the same
verified Release directory without changing the application core or portable artifact contract.
If automated publication is unsafe, disable the publishing job while retaining candidate and local
archive verification; do not publish an unverified artifact manually as a substitute.
