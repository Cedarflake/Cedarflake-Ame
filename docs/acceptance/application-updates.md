# Application update verification

Dates: 2026-11-25–26

Scope: installed and portable application updates under
[ADR 0043](../architecture/0043-application-update-coordination.md). The
[roadmap](../roadmap.md#application-updates) owns delivery status.

## Client and native boundaries

Local verification used the `codex/app-updates` worktree based on
`22816d57fb17dd0008d503788e05f3f837cd2577`. The product implementation was submitted as
`adf2540708e482f2b051a24e4b2eb9251c552659`; local package receipts retain their pre-commit source
provenance. Hosted results must identify their checked commit separately.

Full hosted verification passed on 2026-11-26 for
`86b6b61ac03c38c0d7ac4dcda70ee5f970ee4033` in
[run 45693847746](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/45693847746).
Static checks, all four Rust partitions, Flutter tests, Windows scan, Windows accessibility,
Windows build verification and the aggregate Windows Gate passed. Release and synthetic workloads
were not selected for this run. The subsequent installed-handoff fixture correction and cloud
execution are recorded separately below.

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

The download-progress follow-up on 2026-11-26 uses the current download, verification or stopping
state as the dialog title, with a localized percentage in the existing left-aligned position.
The official Material 3 bar
interpolates between received-byte snapshots over 200 ms; reopening uses the current value and
reduced-motion settings bypass interpolation. Ten focused controller/presentation checks passed,
including intermediate progress frames, total-size changes, reopening, reduced motion and all
three languages at enlarged text. The original hosted run above precedes this follow-up; the
integration run below covers it.
The isolated Profile comparison rendered the production left-aligned percentage and retained
active progress after dismissal. Separate ignored preview copies exposed centered percentages
and the updated viewer slider for visual comparison. The accepted result retains status titles,
left-aligned percentages and the existing viewer slider.

The preference-dialog follow-up on 2026-11-26 uses the shared short-form dialog. Six focused
widget checks passed, including policy/channel selection, persistence and closing at 150% text
in all three languages. The affected analyzer check and Windows Debug build passed. In the
isolated client, changing automatic checks to manual checks updated the About summary immediately;
closing the dialog retained the original Settings scroll position. No update was installed.

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
[upgrade/repair/rollback evidence](https://github.com/Cedarflake/Cedarflake-Ame/blob/e5874f1bed869075019a6626768d7aaf2c227dcb/docs/acceptance/r10-upgrade-compatibility.md). The new native MSI handoff received
separate affected-path verification. Its first disposable guest
stopped during the old version's VC runtime prerequisite: a new 180-second fixture bound expired
after the minimum-runtime MSI succeeded, while the additional-runtime MSI was still active. No Ame
installer or update helper had started. The fixture now uses the existing baseline owner's
600-second prerequisite bound and retains an explicit user-authentication handoff when needed.

The next guest installed Alpha 3 successfully. Its token-only preflight then exposed a PowerShell
process-observation issue: both ordinary and administrator probes exited successfully and emitted
valid JSON, but `ExitCode` was null without an early retained process handle. Retaining `.Handle`
produced exit code 0 for both probes, in 107 ms and 96 ms respectively. The fixture now retains that
handle and copies its JSON before reporting a failure; the product helper did not require a change.

The default Sandbox policy has `EnableLUA=0`. A separate ordinary-user probe used the shipping
helper's `ShellExecuteEx` flags and `runas` verb with the system `whoami.exe`. Windows accepted the
launch, but the child remained unelevated and no permission prompt appeared. This environment cannot
establish the installed handoff's elevation behavior. All owned Sandbox guests retired; these runs
installed no host product or service.
The ignored `application-update-token-probe/diagnostic.json` and
`application-update-runas-probe/result.json` retain the observed tokens and actual API result.

Installed-handoff verification used an authorized clean `cloud-primary` test computer
with normal UAC. The observed upgrade returned helper exit code 0, admitted the target application's
exact hash and version, and reopened its window in the same ordinary account and session. The user
marker and competing-launch rejection evidence remained intact. The broker was configured for manual
start and was stopped; the fixture's persistent `Running` assertion conflicted with the broker's
ten-second idle retirement. The corrected fixture requests demand start from its ordinary worker token
and waits up to five seconds for `Running`, recording the before and after states. The corrected run
returned helper and installer result 0, reopened `0.1.0-beta.1+4`, and verified the broker's transition
from `Stopped` to `Running`. The actual upgraded window rendered the empty gallery and responded to
Settings navigation. The final worker receipt (`2991cec1379d41bb8d26bd93bd3902d9`) records an
unelevated reopened application, preserved user marker, rejected competing launch and normal
application closure. The owned application and service were subsequently uninstalled. This run
uses the same shipping helper and MSI hashes above; only its service assertion differs from the
fixture in the hosted commit.

Two disposable profiles remained loaded after their processes exited, preventing normal profile
removal. After a normal restart, cleanup on 2026-11-26 verified the sealed owners, original elevated
controller, absent logon sessions and unloaded profiles. Both accounts and profiles, four marked
fixture trees and their stored test credentials were removed; the receipt reports `passed-cleaned`.
The upgrade and cleanup receipts were retained locally before removing the transferred packages,
protected cleanup code and current and earlier fixture records. The final directory inventory was
empty, and the cleanup sessions confirmed terminal closure. No startup task was registered. The ignored
`application-update-cloud-retire-result.json` and
`application-update-cloud-retire-archives-result.json` retain the cleanup evidence.

## Review boundaries

The Dart update controller has 298 production lines; the native application coordinator has 184;
the native platform facade has 220; handoff preparation has 263 and its process-authority owner has
141. These owners contain no inline tests. The controller's dedicated test file has 316 lines,
the native coordinator's has 90 and the handoff process tests have 136. The Windows helper entry point has 139
production lines and its session owner has 93. Portable package assembly has 157 lines. These owners
separate release discovery, transfer, process authority, package replacement and presentation; no
catalog workflow takes update ownership.

No real source root, installed host service or existing user catalog is a mutation fixture. Release
publication and legacy Alpha migration remain outside this integration.

## Integration — 2026-11-26

[PR 74](https://github.com/Cedarflake/Cedarflake-Ame/pull/74) merged
`003807911078bb59eec50b143858c345ed1cdbbd` as
`81f579bfdb276b68f95a2f7ee7570eeb30e57110`. Full
[CI run 45736459157](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/45736459157) passed for
that head, including the progress follow-up and revised installed-handoff fixture. The merge tree
matches the checked head. Local `main` was synchronized and `codex/app-updates` was deleted locally
and remotely.

The automated request for independent update signatures was resolved under ADR 0043's accepted
exclusion of code and feed signing. The official repository/release channel remains a trusted
boundary; HTTPS and same-release hashes do not protect against its compromise.
