# C05 browsing diagnosis

Status: preview-demand correction has focused evidence; native blank/gray browsing remains unresolved.

## First diagnosis: source and outcome

The 2026-09-10 diagnosis starts from `507db08f897839aeafb2dc011ef7332ad50fadfa` on
`codex/r2c`, after the approved 120-minute supplement. Product source is unchanged at that checkpoint.
The [execution plan](../plans/r2c-closeout.md#approved-browsing-continuation) owns the scope and
30-active-minute unknown-cause stop. Earlier [native observations](r2c-live-gap-recovery.md#c05-browsing-findings--unresolved)
remain valid observations; the controlled widget results below do not disprove them.

| Observation | Current evidence | Disposition |
| --- | --- | --- |
| Entire wall blank after a timeline click until another scroll | Preserved native capture and pre-removal catalog; controlled direct clicks and delayed-manifest interleaving both materialize visible target tiles | S1 unresolved; no causal repair |
| Persistent gray cards without retry after deletion/jump | Preview-demand re-admission now has a failing-before/passing-after application regression; detail slots and pending previews both produce gray surfaces | Native incident unresolved; application correction below does not establish screenshot causality |
| Transient whole-wall thumbnail errors after deletion | Reported native observation; retained logs do not identify all visible requests at that instant | Unresolved; do not infer harmlessness or decode failure |
| Time rail disappears during publication | Deterministic first-frame failure when only catalog revision changes | S2 presentation defect recorded; no proved connection to persistent blank/gray |
| Layout flashes after closing the viewer | Hidden gallery width changes from 940 to 1020 logical pixels at a fixed 1280 by 800 viewport | S2 presentation defect recorded; no proved connection to persistent blank/gray |

## Controlled boundary evidence

The fixture uses the actual screen, viewport, layout and navigation owners with a controlled
2012-item catalog, twelve mixed aspect ratios and two date groups in 2026 and 2012. It is an
interleaving/geometry fixture, not a replacement for the frozen 10000-image native workload.
Its preview adapter deliberately never completes: it cannot verify pixels, preview recovery,
decoder behavior, actionable failures, or cold/warm preview convergence.

The final four-test run has two passing navigation cases and two failing presentation cases.
Direct navigation covers both ends and two interior positions without a wheel event. The
interleaving case publishes revision two, delays its manifest, requests ordinal 1730 and verifies
that the target is visible after manifest completion. Both cases also require visible location
slots to be real photo tiles rather than unloaded detail placeholders.

Earlier assertions in the interleaving fixture were invalid and remain in the raw evidence:
the inner rotated Material Slider reverses its value, so 0.86 selected near the top. Correcting
that input to 0.14 issued the expected ordinal-1730 request. A subsequent assertion incorrectly
required the retained window to begin after 1500; normal previous-page prefetch moved its start
to 1230. The final assertion inspects the visible target itself and passes. Neither earlier
failure establishes lost navigation intent, blank browsing or a product regression.

The viewer case catches the width change on the first hidden frame. The screen removes its
80-pixel timeline child while the gallery remains laid out inside an IndexedStack. This proves
unnecessary hidden reflow; it does not capture the exact user-observed return frame or establish
a source-size change. The revision case catches an absent slider on the first publication frame:
the screen rejects the previous revision's layout snapshot, while the time-navigation owner clears
its stable metrics and renders no rail until replacement geometry arrives. Keeping stale geometry
without identity proof is not an admitted correction.

Read-only preview inspection also finds possible demand re-admission questions after superseded
completion and explicit root-authority restoration. Neither was reproduced in the bulk-delete
workflow. The restoration caller belongs to explicit root scan completion, not ordinary live
removal. No speculative queue retry, source-guard relaxation or cache invalidation is implemented.

## Retained artifacts and verification limits

Final raw output is in ignored
`build/diagnostics/r2c-p2-raw-f9b3cda5730642229d12537ad8a8898d/`:

- `stdout.bin`: SHA-256 `53F166B9D2E98B0F485CC1408AD38ED3C73401EDA61B2356DFC476D47CEFF509`;
- `outcome.json`: SHA-256 `EA7AAE201B4D0BF583365CDCA749D95E6731EB9F1219F0B75BCB46A077B70080`;
- owned capture duration 16151 ms, exit one, process retired, Job closed, no cleanup failure.

Earlier outputs remain in the same diagnostics directory under raw-run suffixes
`b3dcf8aa99364cd3902968f3b8abd861`, `28e4257427614b6392a5432a18cac30c`,
`32f3def9d37741c6b15e65ff96f8c1bf`, `f73ce817ad0d4d8389f48525a2b0b3ee` and
`9559df8828fc4dec8169b913f8e4f683`. They retain evolving assertions and failures rather than
forming five independent product reproductions.

The exact diagnostic source is preserved as ignored
`build/diagnostics/r2c_c05_browsing_continuity_test.dart`, SHA-256
`BE32704F7BBD0A9E5F28BC24138F2571E7EAF06460C93CBA74CBF4EE7F1D6C83`.
It originally ran at `test/features/library/presentation/library_browsing_continuity_test.dart`;
its relative support import requires that original location. It is retained outside the regular
test inventory because its two unresolved assertions intentionally fail, not disabled or marked
passing. There are no C05 production changes or committed test exclusions.

At this first checkpoint no C05 native client, source mutation, decoder run, Debug rebuild, lint or full Daily is claimed.
Only synthetic in-memory assets were used in the widget runs. Frozen source files and all retained
C04 catalogs remain untouched. The stopped C04 client lifetimes are not reused or extended.

## Stopping decision

Conservatively charge the full 30-minute diagnosis allowance, including five delegated inspection
minutes, setup corrections, source inspection and result analysis. The persistent native blank/gray
cause remains unknown at that boundary. Stop expansion before product changes and retain the two
lower-impact presentation defects separately. Counts and passing controlled navigation do not
establish usable native browsing. A replacement native-evidence method is proposed in the execution
plan; it does not become authorized merely because repair time remains.

The first independent evidence review uses two active minutes and confirms this limited conclusion.
The width assertion stops at hidden frame zero before the return assertions execute; the rail
assertion also stops at its first failing frame. Neither test proves the duration of the reported
native symptom. A final scoped documentation check is recorded in the cycle accounting.

## Renewed native observation and preview-demand correction

The renewed functional instruction admits the execution plan's 76-minute pivot from `4b02218`.
The owned Debug entry copies the production bootstrap and delegates to real Rust catalog,
manifest and preview ports, adding bounded request and visible-tile observations. It is an
instrumented entry, not an unchanged performance baseline. Build capture
`r2c-p2-raw-8403e029003c4a3880f0b605b701b765` passes in 39340 ms. A read-only SQLite backup of
the closed C04 catalog creates a separate C05 catalog; previous catalogs and caches are preserved.
A separate generated twelve-image stimulus is prepared but never imported.

The one admitted session runs from 04:27:26.907 to its fixed 04:57:26.907 UTC deadline.
Computer Use can inspect the window but rejects every click as simultaneous input, including
fresh screenshot, fresh accessibility element and one runtime reset. The user confirms no window
operation. The tool's underlying cause is unproved; no user interference or product-input failure
is inferred. No root removal, stimulus import, timeline jump or bulk change executes. Startup
records 50 real preview completions and twenty visible tiles with image pixels; this does not
exercise the incident sequence.

Run `c59c2edaf6a64bdfaa32443f440829bf` ends at the parent's 1800-second deadline, after 1800518 ms,
with its process retired, Job closed and no cleanup failure. This is a blocked observation,
not a failed repaired replay. Memory monitoring retains 6782 samples, sampled peak private bytes
333885440, kernel peak commit 350994432 and minimum available system memory 5483483136; its
monitor reports no resource failure. The second lifetime cannot be started after the shared
deadline. Client receipts, trace, preparation provenance and session lock remain in the ignored
C05 fixture identified by `build/diagnostics/r2c-c05-client-root.txt`.

Remaining diagnostic work uses finite preview completions at the application boundary. A real
timeline publication first trims pending preview work and replaces `state.assets`; visible demand
arrives after layout. A second navigation or query can restore the same visible assets before
that demand is published. A pending request may already have been cancelled, or an active result
may have been correctly rejected while its asset was absent. The coordinator nevertheless skips
an identical demand description, so the current tile remains pending without a queued request.
These production call sites establish reachability, not causality for the captured native frame.

Two focused tests fail on the prior source: cancelled pending work never starts again, and a
rejected completion prevents unchanged visible demand from loading again. Five existing tests
pass. Capture `r2c-p2-raw-3941f3f13c724ad7903af29fc55207c9` preserves both failures, exit one,
9322 ms and successful owned retirement. The correction removes coordinator-level description
deduplication. Each actual demand publication reaches the existing queue, which alone knows
whether compatible pending/active work or a stored result satisfies it. No timer, forced retry,
cache reset, concurrency increase or source/publication-guard weakening is added.

The final focused capture `r2c-p2-raw-cdc61fc7e61f450282bbb4d49051a6c2` passes all 100 tests:
controller 56, coordinator 10, queue 29 and store five, in 29534 ms, with empty stderr and clean
retirement. New negative cases submit unchanged demand twenty times and retain one active/pending
decode, no repeated ready decode, and one notification for decode/root-unavailable failure.
Explicit Retry still reaches ready. Existing queue cases preserve source-generation rejection
and concurrency bounds. Independent causal inspection and code review each take two active
minutes and find no blocking issue in this narrow correction.

Full lint capture `r2c-p2-raw-1f14669dc83949f899784020e4cf3327` passes in 152668 ms, including
format checking, repository guardrails, Clippy and Dart analysis. Its process retires and Job
closes with no cleanup failure; stderr contains only Cargo's successful completion message.
The corrected normal `lib/main.dart` Debug entry also builds successfully in 35196 ms, capture
`r2c-p2-raw-a319ba37ec8140508e6b785fe64e17bb`, with empty stderr and clean retirement. Its Dart
kernel SHA-256 is `42F3BE3E9B45E0E54D9FA9674E2A5438CF753EB97F9A1D85F6EC2324F424CF92`.
This replaces the scratch diagnostic executable payload; no new application lifetime is launched.

After native retirement, a full read-only source check verifies all 10000 frozen files against
their exact path roster, SHA-256, file identity, size, creation/modification times and date
distribution. The retained C04 512 files and newly prepared C05 twelve files also match their
identity/hash/size/modification-time ledgers. The C05 copied catalog still publishes exactly
10000 and 512, has SQLite integrity `ok` and no foreign-key violation. Evidence is the new
fixture's `c05-final-integrity.json`; no old evidence is overwritten. This proves fixture
preservation and structural integrity, not an application FULL reopen or successful navigation.

The changed production coordinator shrinks from 277 to 254 lines, with zero inline-test lines;
its dedicated tests grow from 231 to 385 lines. Queue (700 production/1061 dedicated-test lines)
and store (248/173) retain their existing ownership and are not extended. There is no dependency,
schema, bridge, source-media or licensing change. The instrumented Debug artifact predates the
repair. A repaired native replay, full Daily and complete browsing acceptance remain unverified;
the original blank wall, transient whole-wall failures, rail and viewer observations remain open.
