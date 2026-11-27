# ADR 0043: Coordinate application updates outside the catalog

- Status: Accepted
- Date: 2026-11-25

## Context

Ame needs one update workflow for installed and portable distributions. Its catalog, analysis
workers, file operations and Windows journal service have independent lifetimes. Downloading an
application package must not take a catalog lock or authorize termination of a source-file operation.
The Settings design and automatic-download preference can remain stable when the package engine
changes.

## Decision

The updates feature owns checking, the selected release, one download, channel preferences and
restart intent. Presentation consumes Ame-owned release/progress/failure values. Application ports
describe checking, a retained package, preference storage and coordinated restart. Adapters retain
package formats, URLs, filesystem storage and engine handles. Production bootstrap connects the
platform update implementation and the existing shutdown owners.

Keep update preferences in a separate versioned `shared_preferences` record under ADR 0013.
Default to automatic checks and the installed release's channel. Manual checks bypass the automatic
schedule. Automatic checks start after the usable application is shown and never open a dialog.
Changing channels retires that channel's pending result and download before it can be applied.
Changing the automatic policy retains an already requested download.

Compare versions using `semver` 1.0.28, licensed under MIT or Apache-2.0. It provides the mature
SemVer parser and precedence rules; third-party values remain internal. Keep Flutter's numeric build
separate and respect ADR 0027's MSI upgrade order within a major/minor line. Stable excludes
prereleases. A channel change never authorizes a version downgrade or catalog rollback.

Use HTTPS and verify the expected SHA-256 and length before reusing or applying a package. Code and
feed signing are excluded from this workflow. Releases and their metadata share the existing trusted
publication owner. No downloaded command line, arbitrary executable path or installer property is
accepted from release notes. Notes are display data.

An active partial download holds its parent directory namespace and an exclusive file handle.
Open reparse points without following them and reject linked partial files before truncation or
writing. The Windows adapter uses read-only `GetFileInformationByHandle` to inspect link count;
its unsafe call borrows a live `File` handle and reads the output structure only after Windows
reports successful initialization. These handles remain local to the adapter and are never exposed
to presentation or the bridge.

Restart is an explicit action on a verified package. The restart adapter joins the application's
file-operation and shutdown owners before handing control to a replacement process. A download or
check cannot invoke shutdown. Updating never moves source files or replaces the catalog, preferences,
organization decisions or operation history.
Normal upgrades preserve the unified data layout and configured custom locations. ADR 0045 owns
the Alpha transition to common defaults; update engines do not import earlier data directories.

### Installed and portable replacement

This engine selection applies to unpackaged MSI and portable execution. Packaged Store execution
uses the separate source and lifecycle in [ADR 0045](0045-microsoft-store-distribution.md); it never
enters these replacement engines or their startup hooks.

Retain ADR 0027's WiX transaction for the installed application and service. Download the matching
architecture's MSI, then use a shipped, unelevated native handoff helper to wait for the application
to exit and invoke Windows Installer with elevation. Windows Installer retains upgrade, repair,
uninstall and rollback ownership. The helper restarts the ordinary application after success,
cancellation or a rolled-back failure; it never runs the desktop application elevated. Preserve
installation-directory and shortcut choices. There is no separately installed service package.
The helper statically links its C runtime so it can run from the handoff directory without adjacent
application DLLs. Production package verification admits only its Windows system-library imports.

Use Velopack 1.2.161 (MIT) only for portable replacement. Its native Rust SDK and packaged Update.exe
own the portable directory transaction and recovery. The distribution tool is a build dependency;
the application requires no .NET runtime. Reject installed Velopack layouts. Keep its types inside
the Windows update adapter. Initial packaging and later full update packages use the same admitted
candidate payload. Portable updates never install or modify the journal service.

The stock 1.2.161 Windows directory transaction is not admitted unchanged: after moving `current`
aside, a failed deployment can remove the retained directory without restoring it. The distribution
must carry a narrowly patched engine that restores the previous directory on failed deployment and
only retires that backup after successful activation. Source, patch and build-tool provenance remain
pinned independently of the official packaging CLI; fault-injection coverage owns this boundary.

Both formats use a two-phase handoff. The helper acknowledges a fully hashed package with held
read/namespace leases before shutdown begins. The parent publishes an atomic commit only after all
registered shutdown owners have joined. Without that commit, the helper reopens the previous
application. It invokes portable `apply --noRestart` after process exit, avoiding the SDK's bounded
parent-wait behavior. The helper records the actual result before restarting the ordinary desktop
session. It never terminates the parent as part of updating.

The acknowledged helper retains recovery ownership until the parent process exits, even if the
parent disposes its update handle. Failed preparation retires the helper before removing its
temporary directory. If the helper exits before activation, prepare and acknowledge a replacement
in recovery-only mode; that mode can reopen the previous application but cannot apply a package.

Before acknowledging readiness, the helper retains the application's existing per-user single-instance
mutex object. It keeps that object alive through parent exit and package replacement, so another launch
cannot begin catalog or file-operation work while the portable engine retires processes from its root.
Release the retained object immediately before reopening the ordinary application. Startup hooks keep
their existing order; the helper reuses the runner's mutex identity rather than adding another lock.

Portable preparation extracts the target engine from the verified full package into the owned
handoff directory. That executable receives its own SHA-256/length check and no-write/no-delete
lease before the parent exits. Running it from that temporary location lets the engine synchronize
its own root copy after successful replacement. Each subsequent update uses its target package's
engine rather than depending on the version already present in the portable root.

Release notes use `flutter_markdown_plus` 1.0.12 (BSD-3-Clause), the maintained continuation of
Flutter's discontinued renderer, backed by `markdown` 7.3.1 (BSD-3-Clause). Notes are bounded to
64 KiB and rendered in the existing dialog's bounded scroll area; images and executable HTML are
not loaded. This dependency owns Markdown parsing and layout, not update commands or URLs.

Ame owns release discovery, package identity, bounded HTTPS transfer and cancellation for both
formats. Validate leaf filenames before giving them to Velopack. Verify retained full packages
before SDK use and again before applying; do not accept the SDK's cache shortcut as proof. Do not
use delta packages until reconstructed payload validation has its own evidence. These requirements
cover the pinned version's upstream issues 1135 and 1175.

Run Velopack's startup hooks before the Windows single-instance lock, with automatic application of
pending updates disabled. A no-argument C ABI function in the Rust library is the runner's sole
startup entry. It accepts no pointers, returns an integer status and contains panic unwinding. The
runner loads only the library beside its own executable and retains the module through process
exit. This narrow unsafe export and function-pointer call do not expose a mutable native resource
to Dart. Ordinary installed and development launches have no Velopack layout and continue normally.

Package replacement begins only after the joined shutdown. Neither engine owns user data. The
portable engine may retire its own old application payload and package cache; it cannot receive a
catalog or source-media path. A failed package check leaves the running version usable.

## Alternatives and consequences

Replacing the complete installed product with Velopack would separate its per-user application
from the protected per-machine service, add a second maintenance lifecycle and remove existing
installation-directory choices. Using WiX for portable files would require installation. A bespoke
portable updater would duplicate replacement, interruption and rollback machinery. The two native
adapters preserve each distribution's lifecycle behind the same application contract.

Keeping engine types in Settings would couple the accepted update interface to installation layout.
Adding update work to catalog synchronization would mix unrelated publication and shutdown authority.
A feature-owned session avoids both, at the cost of one explicit bootstrap binding and retained
download lifetime.

Cached packages still need verification. A library's successful download call alone does not admit
retained bytes or prove a reconstructed package. The selected adapter must establish those checks
before returning a ready state.

## Validation and replacement

Cover channel/version boundaries, superseded checks, channel changes during download, cancellation,
retry, preference persistence and joined restart. Verify the actual installed and portable upgrade,
failed-update recovery and retained user data with disposable versions. Replacing the update engine
changes its adapter and distribution tooling together while preserving the Settings and notification
contracts. Ordinary commits provide source rollback; published artifacts are immutable.

## References

- [Semver crate](https://docs.rs/semver/1.0.28/semver/)
- [Semantic Versioning](https://semver.org/)
- [Windows Installer command line](https://learn.microsoft.com/en-us/windows/win32/msi/command-line-options)
- [Velopack 1.2.161](https://github.com/velopack/velopack/releases/tag/1.2.161)
- [Velopack startup hooks](https://docs.velopack.io/integrating/hooks)
- [Velopack filename admission](https://github.com/velopack/velopack/issues/1135)
- [Velopack cached-package verification](https://github.com/velopack/velopack/issues/1175)
