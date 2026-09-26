# R2c evidence publication attribution

Status: **Native replacement failure reproduced; responsible mechanism remains unresolved**.

The 2026-09-22 [bounded method](../plans/r2c-closeout.md#c02-native-replacement-attribution)
investigates C02's current full-lint failure without rerunning the UI or complete gate. Product
source is `16c215e4d05b8f5ba53ae82fab8cd6db1f45c76c`. No product or tracked verification script changes
result from this attribution. The previous raw `build/diagnostics` directory is absent from the
current checkout; its tracked conclusions remain historical, not newly inspected raw evidence.
The current failure transcript remains `.build/c05-preview-feedback/lint-transcript.log`.

The Windows contract identifies error 1175 as failure to remove the replaced file while preserving
both original filenames; it does not identify the responsible actor.
([Microsoft ReplaceFileW reference](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-replacefilew))
The experiment queries current file users through Restart Manager without invoking shutdown or
restart. A successful empty list cannot exclude an earlier transient user or a filesystem filter.
([Microsoft RmGetList reference](https://learn.microsoft.com/en-us/windows/win32/api/restartmanager/nf-restartmanager-rmgetlist))

## Direct replacement and holder observation

Run `e05db7320df342e58dca77a5c9064854`, child PID 5508, records 896 ms of child work. Its controlled
non-delete-shared reader produces error 32 and native status `0xC0000043`; Restart Manager correctly
returns the owning child PID. All 30 production .NET record writes and reads succeed.

The separate native arm succeeds through initial publication and 26 replacements, then fails on
replacement 27 with error 1175. The retained destination contains attempt 27 and the separate draft
attempt 28; both have ordinary Archive attributes. Restart Manager reports no current user. The
captured native status `0xC0000100` also occurs on successful replacements, so it cannot identify the
failing operation's native cause. This reproduction excludes a failure unique to PowerShell/.NET
binding but does not establish a leaked reader, denied permission, external application or filter.

The parent incorrectly reads `Process.ExitCode` from a process obtained by ID; it returns null.
The command is therefore failed, with confirmed process exit, closed Job and no cleanup failures.
The child observations remain evidence, but the overall run is not described as passing. Subsequent
preparation uses the existing Job owner's retained process handle to read the exit code.

## Controlled mapped-view comparison

The first mapping preparation, `4cbec60bbab744f2b90126d6f84b3a7b`, exits one before any observations:
PowerShell converts the null mapping name to an empty string. Its 299 ms child record and original
script remain retained. Passing the existing explicit `NullString` representation corrects only
the diagnostic argument; it does not change the product or repeat an observed mapping experiment.

Corrected run `7fceae06cb494b9b92a8475d8a61645c`, child PID 36684, completes in 353 ms and exits zero.
Replacement succeeds with only a mapping object, with a live view plus original handles, and with
a live view after closing both the original file and mapping handles. The live views continue
reading the original bytes while the destination contains replacement bytes. All files use
explicit delete sharing. This rejects the tested mapped-view hypothesis; it does not claim that
every possible image section or external mapping configuration is equivalent.

All three admissions retain their files under ignored `.build/c02-replacement-attribution/`.
Both failed and successful commands confirm owned process exit, Job closure and no cleanup failure.
No original images, application catalog, permissions, services or filesystem filters were changed.
Both permitted causal experiments are consumed. No retry, alternate publication protocol or causal
repair is justified by these results. Full lint/Daily and C02 remain open; the separate eight-second
native UIA timeout has not been attributed to this file failure.
