# Application update verification

Dates: 2026-11-25–26

Scope: the [application updates plan](../plans/application-updates.md), under
[ADR 0043](../architecture/0043-application-update-coordination.md). The
[roadmap](../roadmap.md#application-updates) owns delivery status.

## Client and native boundaries

Local verification used the `codex/app-updates` worktree based on
`22816d57fb17dd0008d503788e05f3f837cd2577`. These observations cover the uncommitted feature;
hosted results must identify its eventual submitted commit.

- Focused Flutter checks cover three-language enlarged-text dialogs, download dismissal and retry,
  channel changes, stale completions, preference persistence/failure and joined window retirement.
- Native checks cover release/version admission, HTTPS asset restrictions, retained package hashes,
  interrupted transfer, cancellation, range responses, exclusive partial-file leases and handoff
  authority. An acknowledged helper survives handle disposal; dead-helper recovery cannot authorize
  package application.
- Dart and Rust dependency-boundary checks passed with explicit update-feature registrations.
- The production Windows Release bundle, native window lifecycle and catalog-free Rust bridge smoke
  passed through `quality_verify_unsigned_windows.ps1`. Its ignored evidence retains payload hashes
  and source freshness. Package evidence must also include the production helper's standalone
  runtime-import check described below.

An isolated Profile window exercised the actual Settings and update widgets with controlled update
ports. Manual checking opened the compact version/size/Markdown-notes dialog. Dismissing an active
download with Escape retained it; completion changed the About action to restart and update. The
automatic-download preference persisted in the fixture store. The window closed normally. This
client check did not perform package replacement or touch a real catalog.

## Distribution and recovery

The admitted portable engine is Velopack `1.2.161+ame.rollback.1`, built from pinned upstream source
and the repository's directory-rollback patch. Three real filesystem fault cases exercised restoration
after failed deployment, retained recovery data when restoration cannot complete, and successful
activation. The static-CRT executable has SHA-256
`3224126777cb28b486a42838ce786a4cc1ab96c0b8a8ec7ac8cd056e734a6ca3`.

Focused package checks passed for the pinned tool/engine provenance, exact ZIP and full-package
payloads, Chinese/spaced filenames, tamper rejection, publication metadata and release reuse.
Unsigned-payload admission requires the shipped update helper and rejects missing runtimes and
reparse entries. These tooling checks are separate from actual application replacement.

`acceptance_prepare_portable_update.ps1` prepares two disposable application versions from the
shipping helper and pinned engines. `acceptance_run_portable_update.ps1` runs them in an owned offline
Windows Sandbox, maps immutable inputs read-only and exposes only its result directory for writes.
Its cases cover committed replacement, missing activation and dead-helper recovery, plus repeated
startup admission and retained user markers. The initial Sandbox fixture's limited scheduled task
still received an elevated token; the helper correctly rejected it before replacement. An ordinary
guest-user fixture is required for the actual handoff result. Its newly created account needed
access to the guest window station and desktop. The controller records existing ACLs and adds only
that fixture account; it does not modify host accounts or desktop permissions.

Clean-guest execution then exposed a production-only runtime dependency: the Flutter parent CMake
project enabled C++ before selecting its modern runtime policy, so its helper linked the dynamic
MSVC runtime despite the target's static-runtime property. The standalone helper build had not
exercised that parent configuration. Selecting the existing policy before `project()` produced
the intended static helper without clearing the native build cache. A production import gate
admits its six Windows libraries and rejects the runtime-dependent main application as a negative
case. The corrected Release bundle records the system-library imports and the same helper SHA-256
as the successful Sandbox fixture:
`04795e6c760013daa3deb042d12478e9f46e8a0b9298c6bba68ffce3e556f3f1`.

The corrected production-target helper passed all three ordinary-user Sandbox cases. Committed
replacement changed `1.2.3` to `1.2.4` and adopted the patched target engine at the portable root.
Missing activation reopened `1.2.3` with code 1603. Recovery after an injected helper exit also
reopened `1.2.3`, ignoring a planted activation marker. Each case observed normal parent exit,
rejected a competing launch after the parent released its own mutex handle, and retained both the
portable-root and profile markers. The owned Sandbox and connection retired after completion.
The ignored `application-update-sandbox-static-helper/result.json` and sealed fixture manifest retain
the exact helper, engine and package hashes.

The corrected production payload produced a full MSI, portable ZIP and full update package. The installer
passed 75 native policy cases, 37 database-policy rejection cases and 16 native checkbox transitions
without installation. Its manifest contains 74 payload files. The update metadata matches both
package receipts. These local artifacts are uncommitted validation builds, not published releases.

| Artifact | SHA-256 |
| --- | --- |
| MSI | `e45ecc97e7144a672fc9ce7f6c5c703b4920f5859822b8a8b9a856a64dfbd5f7` |
| Portable ZIP | `a6e48a4499ea81774dfb071417cd01c53523c5bf13cecc4c781e01c7917f2323` |
| Full portable update | `c47f63a08f7d86141d302ff64678388fdacedb5d9f0a3ffbb378ce2de80f8e61` |
| Update metadata | `ab531b24b2e14c57a8901ab0576802ec60aeb53e592266b1cc1e925bf5fecc57` |

The installed product retains the existing WiX transaction and its
[upgrade/repair/rollback evidence](r10-upgrade-compatibility.md). The new native MSI handoff still
requires affected-path verification before this feature is delivered. Its first disposable guest
stopped during the old version's VC runtime prerequisite: a new 180-second fixture bound expired
after the minimum-runtime MSI succeeded, while the additional-runtime MSI was still active. No Ame
installer or update helper had started. The fixture now uses the existing baseline owner's
600-second prerequisite bound and retains an explicit user-authentication handoff when needed.

## Review boundaries

The Dart update controller has 297 production lines; the native application coordinator has 184;
the native platform facade has 220; handoff preparation has 263 and its process-authority owner has
141. These owners contain no inline tests. The controller's dedicated test file has 316 lines,
the native coordinator's has 90 and the handoff process tests have 136. The Windows helper entry point has 139
production lines and its session owner has 93. Portable package assembly has 157 lines. These owners
separate release discovery, transfer, process authority, package replacement and presentation; no
catalog workflow takes update ownership.

No real source root, installed host service or existing user catalog is a mutation fixture. Releases,
merge and legacy Alpha migration are outside this implementation's publication authority.
