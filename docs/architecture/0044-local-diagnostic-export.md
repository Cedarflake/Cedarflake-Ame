# ADR 0044: Export bounded local diagnostics independently of catalog startup

Status: accepted
Date: 2026-11-26

## Context

Support needs build identity, task phases and native error codes when startup or normal work fails.
Copying exception messages or raw process logs can reveal personal paths, image names and account
data. A diagnostic feature that requires a healthy catalog cannot explain catalog startup failures.

## Decision

The diagnostics feature owns a typed snapshot, export lifecycle and replaceable collector/platform
ports. Bootstrap projects existing application state into these contracts and owns the controller
outside native acquisition. Disposing a failed provider container does not dispose diagnostics.
The final window shutdown joins export work and closes the journal before native shutdown completes.

The platform adapter uses the existing native save picker. It writes a ZIP containing `summary.txt`
and `diagnostics.json` (format 2), with a timestamped suggested name. The JSON is the machine-readable
equivalent of the summary. No upload is performed. Collection does not read source images or copy
catalog contents; live application state and existing application queries for bounded configuration
and runtime descriptors supply facts.

Admitted fields contain build/OS/runtime identity, CPU model, OS-usable physical memory capacity,
installed graphics adapter models and driver versions, current-window display scale, aggregate
counts, task phases, model selection and
compiled supported catalog/settings schema versions. Schema versions describe this build, not an
inspection of a database that failed to open. Runtime preference and observed runtime are separate;
unavailable observations remain unavailable. Events contain timestamps, components, phase identifiers,
application error identifiers and namespaced numeric OS errors. Raw messages, stacks, paths, personal
computer names, hardware serials, file/album/root names, identifiers, image bytes, catalog records
and credentials are excluded.
Existing application exception handlers retain their behavior.

Environment collection starts only for an export and runs alongside existing state reads. The
Windows runner exposes one read-only platform channel independent of catalog/native-library startup.
It reads the processor model, `GlobalMemoryStatusEx` and the present display devices' description and
driver-version properties through SetupAPI. No GPU context, WMI process, source-file read or network
request is started. One worker owns at most eight graphics entries and fixed 256-character native
buffers; another request cannot start a concurrent worker. Runner shutdown joins it before retiring
the engine. The pinned Windows messenger retains and locks cross-thread replies.

Flutter's `FlutterVersion` supplies its framework version and engine revision; the binding's current
implicit view supplies display scaling. Hardware reads have a two-second presentation deadline;
unavailable observations retain the remaining report. Graphics adapters keep their own driver
versions and describe installed hardware, separately from the observed analysis runtime. The report
adds this bounded adapter list in format 2; event-journal storage is unchanged.

The journal lives in application support storage, outside source trees. It retains at most 256 events
and 128 KiB. Writes are coalesced, with one active write and one latest snapshot. An unavailable journal
does not prevent startup or current-state export. Collection has a five-second bound; optional state
reads have shorter bounds. Archive text totals at most 256 KiB and is encoded off the UI isolate.

Export cancellation retires pending collection promptly. File writing uses an owned temporary sibling
and checks cancellation before the rename commit. A successful rename is the completion boundary;
late cancellation or leftover-temp cleanup cannot turn a saved report into a failed export. The
controller joins active writing during shutdown. Error presentation exposes structured codes rather
than arbitrary exception strings.

Settings uses the public diagnostic controller and save-label contracts through registered feature
edges. Bootstrap composes its feedback with existing workbench feedback. Startup failure renders the
same feedback and export action, and its clipboard action uses the same safe error projection.

## Dependencies and alternatives

`archive` 4.7.1 (MIT) supplies ZIP encoding and `path_provider` 2.1.6 (BSD-3-Clause) supplies the
application-support directory. Flutter registers their distributed license notices. Neither type
crosses the platform port. These packages avoid a handwritten ZIP format or OS-dependent storage
path assumptions. Raw-log bundling was rejected because its privacy boundary is not stable. A new
task center or external telemetry service is unnecessary for local export.

## Consequences and validation

Safe identifiers explain known failures while intentionally excluding arbitrary stack traces and
path-specific details. Native process crashes can only leave previously recorded events; this is
not a crash-dump collector. Removing the feature needs no catalog migration and leaves only the
small application-support journal. Format changes require a new explicit export format version.

Regression coverage exercises archive contents, size bounds, cancellation/commit, destination failure,
provider recreation, safe error projection and localized settings/feedback. The scoped
[acceptance record](../acceptance/r10-diagnostics.md) records actual execution and desktop evidence.
