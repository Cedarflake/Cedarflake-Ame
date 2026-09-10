# C05 browsing diagnosis

Status: bounded diagnosis stopped; core blank/gray browsing cause unresolved.

## Source and outcome

The 2026-09-10 diagnosis starts from `507db08f897839aeafb2dc011ef7332ad50fadfa` on
`codex/r2c`, after the approved 120-minute supplement. Product source remains unchanged.
The [execution plan](../plans/r2c-closeout.md#approved-browsing-continuation) owns the scope and
30-active-minute unknown-cause stop. Earlier [native observations](r2c-live-gap-recovery.md#c05-browsing-findings--unresolved)
remain valid observations; the controlled widget results below do not disprove them.

| Observation | Current evidence | Disposition |
| --- | --- | --- |
| Entire wall blank after a timeline click until another scroll | Preserved native capture and pre-removal catalog; controlled direct clicks and delayed-manifest interleaving both materialize visible target tiles | S1 unresolved; no causal repair |
| Persistent gray cards without retry after deletion/jump | Reported native observation; detail slots and pending previews both produce gray surfaces | S1 unresolved; no proof yet of which owner failed |
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

No C05 native client, source mutation, decoder run, Debug rebuild, lint or full Daily is claimed.
Only synthetic in-memory assets were used in the widget runs. Frozen source files and all retained
C04 catalogs remain untouched. The stopped C04 client lifetimes are not reused or extended.

## Stopping decision

Conservatively charge the full 30-minute diagnosis allowance, including five delegated inspection
minutes, setup corrections, source inspection and result analysis. The persistent native blank/gray
cause remains unknown at that boundary. Stop expansion before product changes and retain the two
lower-impact presentation defects separately. Counts and passing controlled navigation do not
establish usable native browsing. A replacement native-evidence method is proposed in the execution
plan; it does not become authorized merely because repair time remains.

The independent evidence review uses two active minutes and confirms this limited conclusion.
The width assertion stops at hidden frame zero before the return assertions execute; the rail
assertion also stops at its first failing frame. Neither test proves the duration of the reported
native symptom. A final scoped documentation check is recorded in the cycle accounting.
