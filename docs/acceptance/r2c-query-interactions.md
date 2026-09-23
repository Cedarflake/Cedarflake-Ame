# R2c native query interactions

Status: selected native Debug query, folder membership and continuous search paths pass; final gates and remaining client variants are open

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

## Native folder window across publication

The [75-minute method](../plans/r2c-closeout.md#native-folder-paging-across-publication) runs on
production source `144ceade532610e6be9aedf13d5bdf73fb6ade55`. Run
`3541d7e32d124d6cb34c47a7c18bfe24`, app PID 33004 and parent 9748, imports 201 generated images in
201 child folders. One subsequent generated image creates `a-new`, before every initial folder.
Both native Show More actions use production catalog and folder-controller behavior.

| Observed boundary | Time (ms) | Result |
| --- | ---: | --- |
| First expanded branch | 122512 | Exactly 200 children at revision 620, cursor `folder-199` |
| Synchronized addition | 139396 | Exactly 202 images; later catalog revision settles at 622 |
| First native Show More | 188899 | Revision 622 replaces the old 200 with `a-new` through `folder-198` |
| Second native Show More | 225461 | Same revision appends only `folder-199/200`; exactly 202 and no remaining cursor |
| Root search `200` | 340428 | Exactly one matching asset with current decoded pixels |
| Select `folder-200` | 379210 | Same single asset under the selected folder and current query identity |

The replacement request has `after=null`: the controller invalidates the old branch before reading.
This proves the native UI's cross-publication replacement and same-revision append. The separate
Rust old-cursor fallback remains covered by its existing controlled tests, not by this request.
Four diagnostic boundary cases reject wrong replacement members, duplicate/missing append members,
cross-revision append and invalid cursors. Search/timeline predicates are byte-identical to the
previously verified helpers. The Debug build passes formatting/fatal-info analysis in 23.5 seconds.

All 7843 observed frames are free of diagnostic failure; 7773 contain current decoded pixels.
Normal close takes 320.9521 ms; the 391386-ms parent exits zero, closes its Job and records no cleanup
failure. Full source checks take 49.082 seconds before and 54.187 seconds after. Exact final active
catalog memberships are 202, 512 and 10000; every expected source hash, identity and date is retained.
The 1473 memory samples record 705155072-byte peak working set, 638349312-byte sampled private peak,
662622208-byte kernel peak commit and 5016555520-byte minimum system availability.

The Debug EXE and Rust DLL hashes match the preceding query run. The diagnostic kernel hash is
`72C76315FC5F1B9A6AE80BDEA2D80AE95147F6DF4ADD7D1A48FBA178ECED76DF`. Independent method and result
reviews accept only these exact membership, revision, pixel, source and lifetime boundaries.
The full 75-minute reservation is conservatively charged, bringing the reservation to 4024 minutes.

Two interaction findings remain explicit. The first Show More resets the sidebar to the top, so
another scroll is needed before the second action (C09). Search requires refocusing between
characters once each debounced query starts (C08). The one-image result proves matching membership,
not uninterrupted typing. Neither interaction is accepted by the passing folder data assertions.

## C08: uninterrupted search input

The [bounded correction](../plans/r2c-closeout.md#search-focus-during-query-refresh) targets a direct
input defect. `LibraryGlobalBar` derived SearchBar editability from aggregate `isBusy`, which becomes
true during ordinary query loading. Material's disabled field loses focus, including after a failed
query. Two connected tests fail on those focus assertions before the production change.

Search editability now has one read-only presentation policy. Ordinary query reads can be replaced
without disabling the editor; primary execution, time-navigation and committed-removal exclusions
remain. Query generations, late-result admission, native source commands and focus mechanics are
unchanged. Independent review finds and corrects an overbroad removal-kind condition: an uncommitted
unregister failure releases publication and must allow further search. A connected command-failure
case and a policy boundary cover that terminal state.

Three connected focus tests, six availability tests and eight existing retained-task interaction
tests pass. The initial test compilation referenced the wrong diagnostic field; correcting it exposes
the two actual pre-fix failures rather than treating compilation as behavioral evidence. The first
full lint invocation reaches successful Clippy output but PowerShell 5.1's outer redirection turns
native stderr into an exception. It is a failed command, not a completed lint pass. The canonical
unredirected run passes all guardrails, formatting and Clippy, then rejects one redundant import
in the new connected test. That import is removed; the complete Daily's full lint component now
passes guardrails, formatting, Clippy and fatal-info analysis. The complete canonical Daily then
reaches its final bridge and whitespace checks on production commit
`7144a1954cd6c6ce18c022df956cee87b8f3bb55`: 1519 Rust tests pass with 19 intentional ignores,
three broker binary integration cases pass, and all 650 Flutter cases in 82 files pass. Controlled
Windows scan run `35d3ca7201284773afb29312bdaf37c6` exits zero in 56515 ms. The native accessibility
run completes all ten original phases, exits zero and retires its owned process and Job without
cleanup failure. All 15 asynchronous bridge contracts and content hashes pass; the final tracked
whitespace check also passes.

The outer timer records 1900064 ms but its child exit-code field is null. Preserve that receipt
limitation: component results and native exit receipts provide the evidence; null is not a proven
zero exit code. No unchanged heavy rerun is used to replace the missing wrapper field. Original
C01/C02 failures remain unresolved by this later pass. Retained stdout and stderr SHA-256 values are
`F519F447BF1FFDEF37371BBB0E208A5CE1A9DBC22EF00A90CEC5228AB1D52963` and
`9E2FA01C07C7BC793C3215D545982044C6DAA0FF61E4CEA0C1CD8A5B63337001`; the saved native accessibility
log is `65FF7C5A7D399202C08F1ED77432C9FED347660A996C07A13D3779FABE85C9FF`.

Hosted [35778908456](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/35778908456), on
`7144a1954cd6c6ce18c022df956cee87b8f3bb55`, subsequently completes all ten required jobs and the
aggregate Windows gate successfully. Three signing-only jobs skip. The complete receipt is
`.build/r2c-search-focus-native/ci-35778908456-complete.json`, SHA-256
`DC528861D05321B72168B272F660E121EE42111A8FF6A8BA2C4E8A5B57D94703`.
This hosted checkpoint does not supply the missing local wrapper field or resolve historical
C01/C02 causality, remaining client variants or external acceptance.

Affected owners contain 15 production lines for availability and 134 for the global bar, with no
inline tests; dedicated new tests contain 150 and 129 lines. The 2327-line screen receives only an
import and policy composition call; no search lifecycle is added to it. The shared test catalog gains
one pre-commit failure injection. No schema, dependency, bridge, source-media or licensing change is
introduced. C09 retains a separate correction duty; Release and complete R2c acceptance stay open.

### Native continuous-search result

Run `1a746f1a564b4826a69b2c66d24ebede` uses the corrected production source, app PID 12524 and
parent 26900. A fresh generated-only catalog repeats the 201-child import, one-image addition,
200-child replacement and same-revision append. One pointer action focuses SearchBar, followed by
physical `2`, `0`, `0` keys without refocusing. The observed text changes at 506493, 516831 and
524587 ms; both gaps exceed the unchanged 250 ms debounce. All 166 focus-observation frames retain
focus. Each text starts and completes its own query; `200` settles to one current decoded image at
524928 ms. Selecting `folder-200` then retains that exact asset at 556181 ms.

The ready marker records 9763 frames and 9711 current-pixel frames. Observation continues until
normal close, giving final totals of 9936 and 9884 with no diagnostic or Retry failures. This
lifetime verifies uninterrupted native input after settled query reads; superseded and failed reads
retain their connected-test evidence rather than being inferred from this native sequence.

The process exits zero after 590975 ms; close-to-exit is 349.9711 ms and its Job closes without
cleanup failure. The 2225 memory samples record 699211776-byte peak working set, 672264192-byte
sampled private peak, 673009664-byte kernel peak commit and 5259980800-byte minimum system availability.
Full source checks pass before and after in 49.236 and 48.849 seconds, with exact final active
memberships 202/512/10000 and unchanged expected bytes, identities and dates.

The Debug build completes in 26.1 seconds after format and fatal-info analysis. Its EXE and Rust
DLL retain the preceding hashes; the corrected diagnostic kernel is
`65FFE4E1CA3361B1F55B7AC2575A854C0390431C9A96924EA111C1F3B969AC01`.
Independent scoped review accepts the input, pixel, source and retirement evidence, preserving
C09 and all final/Release duties. Charge the 120-minute reservation in full, bringing the cumulative
reservation to 4144 minutes; serial verification wall time is separate.

## Native committed-removal display Retry

The [80-minute method](../plans/r2c-closeout.md#native-committed-removal-display-retry) uses production
commit `7144a1954cd6c6ce18c022df956cee87b8f3bb55` and fresh isolated derived storage. Run
`3966373b8aad4aa297f9f530516b0bb6`, app PID 35436 and parent 41052, retains the generated 10000/512
catalog. Native input selects the mixed root, opens the other root's actual source menu and confirms
removal from Ame. Rust performs the real unregister; the diagnostic catalog adapter injects only
the first subsequent display-read failure. This is controlled error injection, not evidence of a
spontaneous backend failure.

| Observed boundary | Time (ms) | Result |
| --- | ---: | --- |
| Retained mixed-root baseline | 26567 | Current decoded content and the complete 10000-item timeline |
| Real unregister committed | 92749 | Removed-root command executes exactly once |
| First display read | 92751 | Explicit `native_fixture_display_failure` |
| Failed display feedback | 92838 | Same retained page, 12 current decoded images, actionable Retry and command exclusion |
| Focused Enter Retry read | 754812 | Real key event targets `library-retry-button`; exactly one display retry |
| Successful display read | 754908 | One remaining root at revision 620; no second unregister or scan |
| Recovered gallery | 755269 | Loading settles, completion publishes once and 21 current decoded images are visible |
| Native Library navigation and search | 830101 | Search `mixed` retains exactly 10000 items and current decoded pixels |

One earlier Enter opens an image because its actual focus differs from the immediately returned
UIA focus observation. That action is not counted as Retry. After returning, a harmless key event
confirms the actual application focus before the passing Enter; the read-admission proof checks
that same focused button and its native key event. Notification opening, focus traversal and the
incidental viewer visit remain in the trace. This proves successful native Retry, not a minimal
focus route or that an immediate UIA snapshot is always current.

Final observation records 53450 frames: 51966 contain retained failed-state pixels and 1209 contain
current query/timeline pixels. Remaining transition/viewer frames are not asserted to contain a
gallery. No diagnostic or thumbnail Retry failure is recorded. The parent exits zero after
878454 ms, below its unchanged 900-second limit; normal close takes 338.1687 ms, with owned-process
exit, Job closure and no cleanup failures. The 3309 memory samples record 730562560-byte peak working
set, 778539008-byte sampled private peak, 956981248-byte kernel peak commit and 5314347008-byte
minimum system availability.

Full source checks pass before and after in 68.966 and 55.231 seconds. All 10512 generated source
files preserve their expected bytes, identities and dates, while exact active catalog membership
changes from 10000/512 to 10000. The two diagnostic boundary tests reject missing rendered evidence
and use outside a committed display failure. Formatting and fatal-info analysis pass; the Debug
build takes 28.4 seconds. The EXE and Rust DLL retain the preceding hashes; this diagnostic kernel is
`0816B439A172F8FB4F242E65E6A2E590D71D255FA0F5DCEADC647AD18AB60F55`.

This closes the selected generated Debug removal-side Retry path. Update/query supersession remains
controlled-test evidence; Release, simultaneous multi-root operations, C09 and final acceptance
retain their separate duties. No product-code correction is made by this verification slice.
Independent method and result review accepts these exact boundaries, including the disclosed focus
observation mismatch and C08 Daily receipt limitation. Charge the 80-minute reservation in full,
bringing the cumulative reservation to 4224 minutes.

## C09: retained folder windows during revision replacement

The registered S2 correction starts from the original native Show More jump above. A connected
controller/navigation regression reproduces the cause on the preceding product code: while the new
first window is pending, revision invalidation clears the tree and the real ListView clamps its
offset from 9284 to zero. Correct returned membership alone had not covered that intermediate frame.

`LibraryFolderTreeState` now retains the displayed snapshot while its request authority is
invalidated. A new revision starts from `after=null`, and a coherent result replaces the window
atomically. Only an actual same-revision cursor can admit append. Failure keeps the old window and
offers the existing folder Retry action. A genuine empty result may shrink/clamp the list after
publication; the correction does not invent a pixel-offset restoration layer.

Independent review identifies two necessary retention boundaries: a refreshed leaf must retire its
expanded descendants, and a removed root must retire cached/pending windows. The extracted tree owner
implements both. An incomplete first page does not prove an unseen child absent; complete membership
does. A newer published child survives an older parent's absence claim. Exact pending-window identity
and request revision exclude late success/failure after retirement; disposal also rejects completion.
The navigation drops retired expansion intent and a leaf no longer renders an old subtree.

Focused application evidence covers delayed replacement, failure/retry/empty publication, peer
retention with fresh read admission, obsolete success/failure, root/descendant retirement and disposal.
Four connected navigation cases cover replacement, visible failure followed by actual Retry, genuine
empty publication and expanded subtree becoming a leaf then recovering. Position and the visible
last-folder coordinates remain unchanged while replacement is pending and after compatible results.
The existing navigation semantics, preview synchronization and 42-case unified-screen suite also pass.

Physical review sizes include imports, comments and blank lines; no production owner has inline tests:

| Owner | Before / after production lines | Dedicated test lines |
| --- | ---: | ---: |
| Folder controller / extracted tree | 215 / 143 + 171 | Controller 784; retirement 255 |
| Navigation | 436 / 452 | New connected folder continuity 322; existing semantics suite retained |

Complete lint passes in 111.103 seconds. The first lint attempt is retained as an outer PowerShell
5.1 redirection failure on ordinary Cargo stderr; the second rejects a constructor's initializing
formal style. The final unredirected canonical command passes format, Clippy and Dart analysis after
that local style correction. The complete Flutter Daily component passes in 576.649 seconds.
The applicable Windows accessibility gate passes in 77.898 seconds: both integration tests, all ten
ordered native UIA phases, no AXTree rejection, normal primary exit and owned Job cleanup. These
component results do not replace the outstanding complete accumulated Daily or close C01/C02.
Receipts are retained under `.build/r2c-c09-c10-*`.

The fresh combined C09/C10 native method reuses the full retained corpus and exact membership/source
oracles. Pre-launch method review finds incomplete cross-process receipt publication, acceptance of
overscan feedback as visible and an unowned helper failure path. Atomic receipt publication, actual
viewport admission and a parent-owned helper Job address those findings before launch. Preparation
also retains rejected nullable/style diagnostics; final diagnostic analysis has no issues. This is
preparation evidence, not fresh native C09 or whole-candidate acceptance. The original failed
observation, C01/C02, Release and remaining frozen variants stay open.

The first combined lifetime `7c80cdca7b144e978dda1881ae355c70` does not pass. A real exclusive-source
failure occurs at 124611 ms; the following frame rejects the hard-coded 48-pixel tile assumption.
The normal justified layout scales a complete row, so a minimum-width portrait is not necessarily
exactly 48 pixels in that row. The visible compact button is not accepted as a completed Retry path.
The helper expires after the unchanged 30-second receipt bound and the parent retires both Jobs at
155984 ms with no cleanup failure. A subsequent UI close attempt finds the already retired window;
it is not evidence of normal close. Full before/after integrity passes in 60.300/54.430 seconds for
10000 frozen, 512 retained and 201 generated files. No added-folder action or C09 position proof was
reached. Preserve this method failure and its ignored fixture; the admitted supplement first selects
a sparse row through actual search input, retaining the exact-width assertion and all other gates.

The sparse-row lifetime `e2ffc4493408471780661400ae4b5472` also does not pass. Actual native key
input establishes `200`, but that query matches 1126 retained assets before import and 1127 after.
The selected portrait is requested outside the visible viewport; its real sharing failure at
385879 ms cannot establish visible Retry. The helper expires and both owned Jobs retire at
417233 ms without cleanup failure. The failed close attempt observes the already retired window,
not a normal exit. Full pre/post source integrity passes in 57.190/57.394 seconds, preserving all
10000/512/201 source files. Memory stays within the original bounds. No addition or sidebar position
proof occurs. The next method requires observed zero/one production-query membership before failure
admission; changing a search string alone is not evidence that its layout premise holds.

### Native correction interactions and retained lifetime gap

Run `9dbe9b8709d94e22a7f5acdc0f637484`, app 33152 and parent 42692, proves the corrected interaction
boundaries. The actual search field establishes `base-200`; the real catalog query proves zero
retained matches at 137899 ms and exactly one current portrait after import at 216738 ms. The new
diagnostic query owner records a failed prerequisite permanently before rethrowing. Five boundary
tests cover valid zero/one admission, a broad-match failure followed by success, missing baseline,
wrong single asset and receipt-publication failure. Diagnostic formatting, fatal analysis and the
22.3-second Debug build pass; an earlier missing-import analysis error is retained.

| Native boundary | Trace time (ms) | Observed result |
| --- | ---: | --- |
| Real read-only exclusive-source failure | 216969 | Native `preview_source_open_failed` |
| Visible compact Retry | 216998 | 48-by-138 tile, 48-by-48 action, complete Retry label |
| Pointer Retry admission | 240682 | Exactly one forced request with unchanged source authority |
| Retry progress / current pixels | 240696 / 241119 | Live progress label, then decoded current image; unchanged tile geometry |
| Revised folder window published | 435185 | First 200 paths replaced, new cursor ends at `folder-198` |
| Remaining folder append / selective search | 462396 / 462401 | Exact 202 folders; root search `200` returns the one current portrait |
| Actual final-child selection | 491865 | `folder-200` selected through the sidebar with current decoded pixels |

The first post-change request uses no retired cursor. The actual ListView offset is 9307.2 before
replacement, during all 50 observed pending frames, and after publication. The three real folder
reads preserve exact replacement/append membership and revision authority. The 350-ms diagnostic
result holds expose intermediate frames; they are not natural latency measurements. Final observer
state records 12563 frames, 2403 with current gallery pixels, two selected preview requests, and all
selected interaction assertions passing. Empty-query and other transition frames are not claimed
to contain gallery pixels.

The aggregate native lifetime still **fails**. Actual titlebar Close dispatches at 506176 ms and
application shutdown audit passes at 506251 ms. The parent observes exit zero, then fails while
reading `helperProcess.ExitTime`: no process handle was retained while that helper was alive, so
Windows PowerShell returns null. Both owned Jobs and processes retire, with no cleanup errors, at
507824 ms; `closeToExitMs` and `helperExitBeforeCloseMs` remain null. Neither successful application
shutdown nor a helper's own completion receipt substitutes for those missing native timing fields.
A separate two-case, 500-ms hidden-helper calibration reproduces null without the retained handle
and a real DateTime with it. This identifies the runner defect without replaying the media workload;
it does not retrospectively pass the failed lifetime or authorize an unchanged replay.

Full pre/post source integrity passes in 54.600/55.553 seconds: all 10000 frozen and 512 retained
files remain unchanged, the 201 baseline sources plus the one admitted addition are intact, and
exact active catalog membership is 10000/512/202. The 1909 memory samples record 723300352-byte peak
working set, 641679360-byte sampled private peak, 656023552-byte kernel peak commit and
6113730560-byte minimum system availability. No real source root participates.

The EXE hash is `2A24C91530C05D88B29ACC4E6FCF73199C237DEB04159B232A9FD7E43BD98DCE`, diagnostic
kernel `E4CDAF44C46C4A0BAAA5E9BB072C43F32347B457EB701AFD835DEF49A06D3AA7`, and Rust DLL
`77B193EA7CA1B5CEEC033038E3806490890E5DBB3E818AAD72B062D0A46F50C2`. The stdout hash is
`105706731774CD0786F3C23ECA39F11A5E965A3A96DA8E58E127C0C0FA11AAB6`; passing application result
`CE4BFF1113E788A43727A6CBE211A4E1DA8D2FF4508661B5F02CFD668EE4BD7C` is retained alongside failed
parent receipt `40724AB7CE6458359D92F03E566FEFF7025200D5F6E325E042A39FBA58040C68`.

C09/C10 therefore have focused and selected native functional evidence. Complete lifetime timing,
accumulated final-source gates, C01/C02 and the other frozen variants remain open. Charge the
40-minute revised-method reservation in full; cumulative reservation is 5269 minutes.

Independent result review verifies the original receipts and finds no additional functional blocker.
It supports committing these focused and selected native functional corrections while retaining
the failed parent lifetime and both missing time fields. Other tile sizes, enlarged text and keyboard
activation remain focused-test evidence, not additional native cases from this run.

## Native reversal while an older result is pending

The UX-08A generated Debug lifetime `38132fb25e9f4cb890b058235bb7b8fb` uses a fresh derived
backup with exact 10000/512/2/2 membership. Five diagnostic boundary tests and the current build
pass before launch. Independent method review corrects rail-value normalization, late-success
deadline admission, exact native-target ownership and startup revision capture. No product source,
source media, release payload or external service is changed by this observation.

Native input selects the older target at 48143 ms. Its real 160-item catalog result for ordinal
9132 is held at 48316 ms. The second actual click arrives at 75491 ms while that result remains
held. The rendered newer target is recorded at 75922 ms: logical value 0.21475875118259222,
target ordinal 2186 and query-wide row start 2185. The old result is released at 75927 ms;
publication observation rejects obsolete ownership, and the next real read returns the exact
newer row at 76041 ms. No wheel input or controller-driven navigation joins this lifetime.

All 15 exposed thumbnails show current decoded pixels at 83008 ms, 7080 ms after release. The
loaded window has prefetched back to ordinal 1685 while retaining the selected 2021-12 anchor.
At 83034 ms the strict position-stability assertion fails. The original observer records only
the first offset, 91214, and not the changed offset or tile rectangles. Consequently this is a
**failed stability interaction**: neither a stale-request rollback, user-visible jump nor harmless
layout correction has a proved cause. The ten-second stability requirement is not accepted.

Normal native Close occurs at 119906 ms. The app exits zero, its Job and observer retire, and the
parent finishes in 121372 ms without cleanup failures. The failed parent has a null close-duration
field because the missing ready event precedes that calculation. A separate, non-overwriting
receipt derives **418.3737 ms** from the already-recorded same-host close dispatch and the
parent-observed exit timestamp; unlike the earlier unavailable helper timestamp, both endpoints
exist. This establishes only normal-close timing, not a passing functional or aggregate run.

Across 450 resource samples, peak working set is 447627264 bytes, sampled private/kernel peak
commit are both 411508736 bytes, and minimum available host memory is 6274441216 bytes. Complete
source checks pass in 67.944 seconds before and 59.259 seconds after; all 10516 generated files
and exact catalog members remain unchanged. The post-source receipt is
`source-integrity-1790144909188279300.json` under the retained recovery fixture.

Ignored `.build/r2c-navigation-reversal/` owns the method; the new
`build/integration-storage-da5de01740324e02ac7186c918a109f5/` retains these hashes:

| Receipt | SHA-256 |
| --- | --- |
| Native stdout | `FF93524A0E17906EE2503F430592DAA0A9BABC55C1A23FDAAF398FE8402C54C3` |
| Failed application result | `0004FA8158F86FCA5489AABDB26D54D0A07D3B21FEAA2445790B214D57E3C124` |
| Failed parent result | `66E24607B5898BE39CD42C3175B40914316CCBD5A8802520443C64719E3F4FC4` |
| Independent close-time derivation | `275EE9A55E4894B2153836D17E455053D08F83CC83A88A0058FDA65F811989E5` |

Charge the 75-minute reservation in full through 5344 minutes. The changed causal observation
records bounded frame geometry and scroll-change stacks without relaxing the original assertions
or converting a latched failure into success. Release, C01/C02 and final/external gates stay open.

### C11 loading-row geometry during pending reversal

The changed causal lifetime `31e7def52ed44f4abe0f62f5dce9593b` preserves the same real inputs,
query, 160-item old result, exact newer row, resource limits and strict stability assertion. It
adds only read-only frame geometry and bounded scroll observation after release. Independent
method review corrects unconditional failure-frame retirement and reserves trace capacity for
the relevant phase. Seven diagnostic files pass analysis; the final build takes 22.8 seconds.
The previous 59.259-second complete post-check is reused as the fresh pre-check of these same
unchanged sources; a new derived backup preserves exact 10000/512/2/2 membership.

Both actual clicks again deliver the required pending reversal. The old row 9132 remains held
through the second native input; release occurs at 58215 ms and the real replacement row 2185
returns at 58333 ms. The frozen logical target remains ordinal 2186 at value
0.21475875118259222. All 15 visible images decode at 65904 ms, 7689 ms after release. Obsolete
page/anchor admission is never observed. The same stability assertion fails at 65930 ms.

The added evidence resolves the size and owning geometry of that failure:

| Rendered transition | Wall top / height | Scroll offset | Selected tile top |
| --- | --- | --- | --- |
| Loading row present | 170 / 542.8 | 91214 | 170 |
| Loading row removed, before resize correction | 168 / 544.8 | 91214 | 168 |
| Following corrected frame | 168 / 544.8 | 91213 | 169 |

These are logical pixels. The manifest identity, layout-metrics identity, 420632 content extent,
query revision and selected source stay unchanged. The transition repeats when detail prefetch
temporarily restores the loading row. `unified_library_screen.dart` conditionally inserts the
two-pixel `LinearProgressIndicator` into the gallery's Column. The query-wide wall retains its
viewport-center anchor when viewport extent changes; `LibraryExactExtentSliver` applies the
corresponding correction during layout. The measured one-pixel offset correction matches half
of the two-pixel viewport change. No scroll-listener call stack is emitted for this layout
correction; the conclusion uses the recorded frame geometry and the existing source boundary,
not a claimed captured `jumpTo` call.

Register **R2C-C11 / S2**, localized loading-row layout movement, for the final minor-repair batch.
It preserves target identity, current pixels and browsing authority, but the failed stability
assertion remains open; severity does not waive the gate. Do not attribute the original large
blank-wall or disappearing-rail reports to this one-pixel observation. A future correction must
own stable loading-feedback geometry without changing query, retry, source or scroll authority.
The causal method is now closed; another unchanged native replay is not admitted.

Normal Close and the parent-observed exit give 374.1722 ms. The app exits zero; the owned Job
and observer retire without cleanup failures. The parent ends in 113214 ms while retaining its
failed interaction result. Across 419 resource samples, peak working set is 444551168 bytes,
sampled private memory is 398868480, kernel peak commitment is 416079872 and minimum available
host memory is 6104199168 bytes. Complete source post-checks pass in 56.667 seconds, with all
10516 files and exact catalog members unchanged. The final source receipt is
`source-integrity-1790145743813149000.json` under the retained recovery fixture.

Ignored `.build/r2c-navigation-reversal-trace/` owns this observation; the new
`build/integration-storage-5c5352616ffd441694af27177fe409d8/` retains these hashes:

| Receipt | SHA-256 |
| --- | --- |
| Native stdout | `FD201FF58A46C9EFA40647146EEF4ED91FAF5A4BC0662AD3DC670B9EE9A23B8B` |
| Failed application result | `E794E2B6CEDC4979605BEA40A88D285B3D105CA7CCE54504F4F949F8CCDFBD41` |
| Failed parent result | `6D7166DF6C64CB8BF31B62BE58FF4F8A9673AD0FC3E21A57181E0D20398B17CA` |

Charge the changed 40-minute observation in full through 5384 minutes. Pending-query ownership,
exact final-target read and current pixels have selected native evidence. Complete UX-08A
stability, Release, C01/C02 and the remaining final/external duties remain open.

Independent result review confirms the frame/source causal interpretation, S2 classification,
both retained failures, exact close times and complete source post-checks. It explicitly preserves
the distinction between pending-request ownership evidence and the unaccepted stability lifetime.
