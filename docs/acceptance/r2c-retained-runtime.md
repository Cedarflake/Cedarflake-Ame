# Retained-library runtime observations

Source: `dcce10e3ce7f2f62a61a82adcdede64c1608f501`; observed 2026-09-25.
Status: startup recovery completed; paging correction passes focused, lint, Daily static and
complete Flutter checks. Recovery cost, the exact retained-client incident and remaining native
candidate gates remain open.

## Existing recovery after startup

The existing Debug process started at 08:14:13.866 UTC. Read-only catalog evidence binds the
`cloud-primary` generation-4 recovery to a watcher coverage gap authorized on 2026-09-08 at
23:48:01.191 UTC. It completed and retired its authority on 2026-09-25 at 08:32:22.466 UTC,
1088.599 seconds after this process started. The retained operation was not newly authorized by
this diagnostic session. Its cumulative inventory contains 50515 entries and 49685 candidates.

This process completed 12211 metadata-inventory path candidates across 193 persisted publication
timestamps. The median inter-publication interval was 5.999 seconds; P95 was 8.538 seconds.
These intervals include unseparated work and scheduling costs, not measured source-read durations.
Later terminal snapshots report synchronized state, zero pending/retry work and zero gaps.
The persisted journal capability remains `live_only`; completed recovery does not prove persistent
coverage across a future closed-process interval.

All diagnostic SQLite connections used `mode=ro` and `query_only`; summary queries had 350 ms
progress deadlines. No scan, source enumeration, source content read or source mutation was
initiated. A 64-item production path-prior SQL sample took 2.057 ms in total (63 records present),
which does not account for the observed batch intervals. On the exact retained root/generation,
the current explicit-recovery claim-count subquery took 16.278 ms; a read-only claim-driven
comparison returned the same zero in 0.045 ms. This is a query-cost finding, not proof of the
18-minute recovery cause or authorization to weaken root/generation ownership. An earlier count
probe selected an unrelated state row, and a subsequent single-registered-root assertion failed;
both are excluded from target performance evidence. The final samples bind the previously
verified root identity and generation explicitly.

The finite recovery now completes, but its cost remains unaccepted. No unchanged real-library
replay or broad optimization is implied by these observations.

## Timeline navigation, upward scrolling and Retry

Severity: S1 browsing-position continuity; source or durable-data harm is not evidenced.

The reported sequence is a successful middle-timeline seek, upward scrolling, partial Retry
preview feedback, then an unsolicited jump toward an earlier timeline position, specifically
not the top. The report explicitly confirms synchronization had already finished. Treat this as
an unresolved C05 functional browsing incident. The retained terminal tail
contains successful preview materializations and synchronized roots, but lacks the error and
navigation frames needed to establish the precise cause. Its lack of a Retry event is not a
negative reproduction result.

The next/previous expired-cursor boundary is independently reproduced: both directions originally
called an unanchored first-page reload. In a loaded middle window, both regressions failed because
no visible identity was supplied. Recovery now uses the existing query projection and refresh
coordinator to retain the current identity, resolved ordinal and within-row fractions. Continued
scrolling retires the old read and captures the later position after the gesture ends. This proves
that boundary, not that it caused every part of the retained-client report.

Independent review exposed two integration risks in the first correction. A page recovery must
retain its original publication authority rather than inherit a committed scan's obligation to
follow a later query. A held recovery followed by an actual failed user query reproduced the loss
of `LibraryQueryFailed`; the correction preserves that failure without another read. Also, passive
synchronization must obtain query admission before entering the shared gallery projection. The
screen no longer captures a position before a request that may be rejected as busy. Viewer
reconciliation remains inside the mounted projection. A gesture-delayed passive request retains
its original publication authority; the red/green supersession test proves it cannot start a late
read after that authority is retired.

The final focused file has 11 passing cases: both stale-page directions, scroll-before-read,
scroll-during-read, disposal, query replacement, failed newer query, admitted passive capture,
gesture-wait supersession, busy passive coexistence and ordinary read failure. These use three
in-memory assets in a 10000-item timeline, real viewport/refresh/projection owners and the actual
gallery transition. They are not a 10000-image native-client workload. Three primary-refresh and
three actual-widget preview-synchronization cases also passed after the admission composition
change. The final five-file batch passes 83 cases, including the existing stable-asset and coherent
snapshot assertions using an attached fixed projection, four committed-position widget cases and
three preview-synchronization widget cases. A separate actual `AmeApp` widget regression passes
with 120 loaded records in the middle of a 10000-item timeline: it holds the stale-page recovery,
delivers a real synchronization-stream notification through the screen, then verifies the same
visible image identities and rectangles after recovery. It uses one generated one-pixel preview,
not native decoding or real-library media. The earlier seven-file pass remains historical to the
final admission composition.

No source-media, schema, bridge or decoder behavior changed. Existing owners retain the lifecycle:
the refresh coordinator admits work, the projection captures position, and the viewport rejects
retired reads. The affected handwritten production sizes are controller 351, viewport 1362 and
screen 2117 lines, with zero inline test lines. The new dedicated application and widget regression
files are 423 and 263 lines; their shared fixed-projection fixture is 14 lines. Existing controller
and coherent-snapshot test files are 3640 and 454 lines after adapting their anchor setup, with
their original identity and atomicity assertions preserved.
The reused refresh, projection and gallery-transition owners remain 136, 77 and 213 lines.
The larger physical decomposition debt remains with the roadmap; no new state flags were added.

Read-only inspection of current registered published roots found 110 persisted failed previews:
102 nonempty `.jpg` entries with `image_format_unsupported`, six nonempty `.jpg` entries with
`image_decode_invalid`, and two zero-byte `.png` entries with `image_format_unsupported`. These are
stored classifications, not a fresh source inspection or proof that those entries were visible
during the report. The 256-row-capped query returned 110 rows in 86.192 ms. No paths or filenames
are included in the evidence. This finding does not establish the transient Retry cause or justify
suppressing feedback, changing format support, or opening originals.

Partial, sanitized terminal buffers are retained under
`.build/r2c-retained-runtime-20260925/terminal-captures.json`. They do not contain complete process
stdout or wall-clock event timestamps. The existing generated-data and hosted passes do not
certify this new retained-library sequence.

Focused red/green logs and the sanitized failure summary are in the same ignored evidence directory:
`stale-page-before.log`, `stale-page-after.log`, `query-failure-before.log`,
`query-failure-after.log`, `passive-projection-before.log`, `passive-projection-after.log`,
`passive-wait-before.log`, `passive-wait-after.log`, and `preview-failure-groups.json`.
Final focused results are `final-focused-corrected-import.log` and `middle-gallery.log`;
`source-manifest.json` binds the affected Dart sources and fixtures by SHA-256.
The first lint wrapper stopped on Cargo's normal stderr because PowerShell merged it into its
terminating error stream; it did not reach the analyzer. The corrected wrapper runs the unchanged
canonical entrypoint in a separate process with separate stdout/stderr. Its first analyzer pass
found two old test call sites still supplying removed anchor parameters and one redundant import.
The tests now attach an explicit projection without weakening their original assertions. The new
shared fixture initially imported the anchor from the wrong module; that failed compilation is
retained in `final-focused.log`, followed by the corrected passing run. Final lint passes formatting,
Clippy with denied warnings, all tool guardrails and Dart analysis with no issues; its logs are
`lint-final.log` and `lint-final.log.stderr`. Native Daily and unsigned verification share the Debug
output still used by the existing retained-library process, so they cannot overwrite it while it
remains running. Daily's static component passes: 1570 Rust cases, 19 explicitly ignored cases,
three broker binary cases and all 16 asynchronous bridge contracts. The Rust suite took 1142.11
seconds; its existing mixed-load P0 latency assertion also passes. Those ignored cases retain their
separate explicit gates, and this pass does not explain historical C01/C02 failures. Daily's Flutter
component also passes all 98 test files with no failed files; `daily-flutter.log` and its separate
stderr retain the full run. Both components ran serially on the source bound by the manifest.
Native Daily and fresh unsigned Windows checks remain pending while the existing user process
holds their shared Debug output. That process was neither stopped nor updated, and still runs the
preceding build. Hosted checks must bind the new commit; an earlier green head is not evidence for
this correction. These partial gates do not establish complete Daily or real-client acceptance.
