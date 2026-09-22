# R2c native query interactions

Status: selected native Debug interaction passes; Release and folder-page revision acceptance remain open

## Scope and invariant

The [execution plan](../plans/r2c-closeout.md#native-query-interaction-during-publication) owns this
bounded UX-03A and partial UX-03B run. Production source is `a7885dd7d83a5f73cef70aab23879a094ab747e6`.
The diagnostic uses the actual Windows Debug picker, production catalog/scanner/watcher, native
pointer/keyboard input and real image decoding. It copies only the derived generated catalog;
the retained 10000-image mixed-size/historical-date source and 512-image source remain immutable.
A new GUID-owned source contains 24 initial images plus 180 additions at one-second intervals.

Acceptance requires current page, query, revision, total and layout identities, unique membership,
current rendered thumbnail pixels, and two selected roots plus search/sort/folder/page milestones
while additions remain active. Final memberships must be exactly 10000, 512 and 204, with synchronized
freshness, full source integrity and normal owned-process retirement. The parent limit is 900 seconds,
post-stimulus convergence 300 seconds, normal close six seconds, client memory 2 GiB and system reserve
2 GiB. This does not establish Release input or the separate 200-child folder revision/page transition.

## Preserved diagnostic failures

- Run `6825168508234cb6b843f7a9994a5cc3` rejects a valid search at 228499 ms. Production searches absolute
  or relative paths; the diagnostic checked only relative paths. All 180 additions converge to 204,
  source checks pass, and the process exits normally in a 405827-ms parent lifetime. Its observer stops
  at failure, so later native sorting/folder actions cannot establish complete frame evidence.
- Run `600f2c8bb126409c907089354b8d0d5b` corrects search scope and retains three positive/negative tests.
  It observes current pixels at page offset 676 during publication at 542120 ms and native search
  `p` at 576792 ms. Name sorting at 625954 ms triggers an invalid bucket-sum assertion: production
  `gallery_snapshot.rs::load_timeline` deliberately returns an empty date-bucket list with
  the retained total for filename sorting. The observer records 27671 frames, including 23814 with
  current pixels and zero Retry frames before failure. Sort occurs after additions; folder selection
  is not reached. The combined workflow fails and is not accepted from these partial milestones.

The second process exits with code zero, owned Job closed, no cleanup error and a 657509-ms parent
lifetime. Its 2481 memory samples record 1117638656-byte peak working set, 1216200704-byte sampled
private peak, 1217253376-byte kernel peak commit and 5037711360-byte minimum system availability.
The final 49.874-second integrity check verifies all 10716 sources, their expected bytes/identity/dates
and exact active catalog membership. The original 10000/512 sources are unchanged. Failed-result
processing precedes the close-duration calculation; the process receipt has no six-second close pass.

## Corrected evidence method

The diagnostic timeline validator distinguishes chronological grouping from filename totals and
always validates query/revision identity. Nine positive/negative tests cover all four sorts, unknown
dates, empty queries, wrong totals, unexpected filename buckets and stale identity. The three search
scope cases preserve absolute/relative matches while rejecting wrong roots, folders and nonmatches.

Independent method review also requires matching manifest total/query/revision, current page ordinal
membership, matching rendered tile root/scan/source generation, settled loading ownership at every
publication milestone, and permanent failure for an incoherent catalog snapshot. Source verification
compares retained modification times against the stimulus ledger and preserves a pre-run identity,
modification/creation-time receipt. These checks supplement the existing full-byte verification.

The focused pre-run independent recheck finds no further execution blocker in these corrected
boundaries. Actual query-change events must additionally place each control action within the
addition interval; a publication milestone alone proves the state was displayed, not when it changed.

## Complete selected native lifetime

Run `a81e91922cc547c98130acadc8a8a8b3`, app PID 40388, completes the corrected workload. The real picker
imports 24 images; the fixture then creates exactly 180 more over 183.266 seconds. The first published
addition is visible at 126837 ms, and final 204-image membership at 309231 ms. Actual query changes and
rendered milestones occur between them:

| Native action | Query starts at (ms) | Current pixels at (ms) | Published stimulus count |
| --- | ---: | ---: | ---: |
| Keyboard search `p`, committed with Enter | 145300 | 145501 | 43 |
| Change capture-date sorting to filename sorting | 173764 | 173903 | 71 |
| Select `baseline-b`, retaining search and filename sorting | 187056 | 187135 | 84 |
| Select the retained 10000-image root, then scroll beyond the first window | 197685 | 282391 at window offset 789 | 177 |

The search matches the fixture paths; the selected baseline folder contains exactly 12 images.
Six distinct queries display pixels with matching page/layout/source identity. Expanding the source
before and after publication returns the two original folders at revision 620, then all four folders
at revision 980. The latter read completes at 324469 ms. This is a folder-list replacement result,
not the separate 200-child cursor/revision test. The diagnostic's `pageReads` counter is zero because
it counts cursor/time-anchor entrypoints; the actual visible-window path is established by the
rendered offset, 500-item window and matching layout ordinals.

The full result records 12383 observed frames, 10050 with current pixels, 10099 with coherent layouts,
zero Retry frames and no diagnostic failure. Ready is recorded at 324468 ms; after the second folder
read completes, native window close exits in 336.6001 ms. The 355011-ms parent finishes with exit code
zero, app and Job retired, and no cleanup failures. Source checks before (48.959 seconds) and after
(48.720 seconds) verify all expected identities, bytes and dates. Final active catalog memberships
are exactly 204, 512 and 10000 with no extras or duplicates; the retained source trees are unchanged.

The 1332 resource samples record 820031488-byte peak working set, 863072256-byte sampled private peak,
871305216-byte kernel peak commit and 5185347584-byte minimum system availability. All admitted
resource and normal-close bounds pass. The diagnostic builds in 25.5 seconds with formatting and
fatal-info analysis passing. Nine timeline-rule tests and the unchanged three scope-rule cases
provide focused diagnostic regression evidence; no product source is changed by this slice.

| Admitted artifact | SHA-256 |
| --- | --- |
| Windows Debug EXE | `2A24C91530C05D88B29ACC4E6FCF73199C237DEB04159B232A9FD7E43BD98DCE` |
| Diagnostic Dart kernel | `1EE853A7BFAA60CA5DCAE36D3B6BC4DD092C979FA9615BE97B8E582A68DE6CAB` |
| Product Rust DLL | `77B193EA7CA1B5CEEC033038E3806490890E5DBB3E818AAD72B062D0A46F50C2` |

The selected UX-03A native Debug interaction passes. Loading frames during navigation are retained;
this does not establish zero transient relayout, Release input, committed-removal Retry, the complete
folder-page revision path, real-library acceptance or final-source Daily/Windows gates. Earlier
failed lifetimes remain failures and are not overwritten by this result.

The final independent recheck accepts this selected Debug evidence after checking the native action
times, current-pixel milestones, exact source/catalog membership and matching run/PID resource/exit
receipts. It retains every limitation above and does not accept all of R2c. Documentation checks
confirm valid UTF-8, existing local link targets, unchanged admitted artifact hashes and clean
`git diff --check`. No new heavy product gate is claimed for these documentation-only changes.
