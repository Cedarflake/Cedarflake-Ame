# R2c isolated Release client evidence

Status: normal native Release lifetime and keyboard sort return verified; remaining interaction variants and final gates open

## Scope and artifact

The earlier 2026-09-23 run uses documentation head
`f3ece08b79e1cc15a3c1f40d1c2a7b5fdc65bed5` and unchanged product source through
`7144a1954cd6c6ce18c022df956cee87b8f3bb55`. The only tracked dirty file at build time is the
execution plan. The canonical `quality_verify_unsigned_windows.ps1` finishes successfully:
fresh optimized application and broker, three engine-free native lifecycle cases, two actual
engine-retirement cases and the catalog-free Release bridge smoke. Its 19-file payload totals
79447460 bytes. `build/quality-unsigned-windows/evidence.json` has SHA-256
`A096091A97136141F381AE65435EFC4CC5F5D6D3162099114BF0323AB44A41A1`.

The real optimized EXE and Rust DLL run with normal Known Folders in a fresh Windows Sandbox
profile; the Debug-only test-storage override is not enabled. The verified product manifest is
unchanged. Three already installed Microsoft-signed MSVC runtime DLLs are copied into the local
diagnostic payload, with separate hashes/signature/version receipts; no installer or release
package is changed. Networking, clipboard, audio, video and printer redirection are disabled.
Only generated sources and the payload are mapped read-only, with one fresh writable result root.

The later [keyboard sort lifetime](#native-keyboard-sort-return-on-current-release) uses a fresh
current-source build. Its provenance is separate from these retained earlier artifacts.

## Retained admission failures

- Empty admission `07d43ac6f3994606b403fce7a2e42242` proves the fresh profile and read-only input
  but fails the host memory floor: 5630590976 bytes at entry, 2032762880 minimum. Guest shutdown
  and later complete retirement are separately observed. The entry threshold is revised to
  7 GiB without changing the 3072 MiB guest cap, 2 GiB host floor or 2 GiB application ceiling.
- Fresh empty admission `9612cf6ff5324eecb19e3846e55b16d8` passes in 85953 ms, with 9441161216
  bytes at entry, 4891901952 minimum, read-only denial code 5 and no surviving Sandbox process.
- Media lifetime `74ec6ff4e4c64c6c8f31589fa3b0cd2b` reaches actual picker confirmation, but the
  redirected source mapping fails production directory binding with `root_identity_unavailable`
  and Windows error 50. It publishes no images. This identifies an unsupported mapping boundary,
  not the exact inner Windows call or a decoding defect. The production identity guard is retained.
  During retirement the input tool loses its target after minimized-window recovery; a screenshot
  of another application receives no coordinates. The matching-run abort retires the owned Job
  and guest. Its 475952 ms lifetime, minimum host availability 4229062656 bytes and failed receipt
  remain; no delivered normal close or decoding acceptance is inferred.

## NTFS copy and native observations

Changed-method lifetime `74b9cbe65f1b4e50b3770af8c2db6fea` copies only the frozen generated corpus
to a fresh guest NTFS directory. Exact 10000 paths, 10921494393 bytes, all SHA-256 values and
creation/modified dates match before launch; preparation takes 107790 ms. File identities are new
on this disposable copy and are not claimed to match the host. The original mapping stays read-only.

All application actions use observed native input. Actual picker confirmation occurs at
01:11:18.146 UTC. By 01:12:01.664 UTC the screen shows 10000 images, import completion and decoded
2026 thumbnails: a conservative observed upper bound of 43.518 seconds, below the unchanged
300-second limit. The 10012 checked entries include directories and are not an image-count mismatch.

| Actual action | Observed outcome |
| --- | --- |
| Click the middle time rail | Exposed 2015-region tiles become decoded without any wheel input |
| Open a tile | Viewer displays `04684-6000x4000-0464.jpg`, position 4389/10000 |
| Press Right | Position changes to 4390/10000 and the 4320-by-7680 image decodes |
| Press Escape | The same visible tile geometry and 2015 rail anchor return |
| Click the bottom rail | The 2010-10-02 region decodes without any wheel input |
| Reverse to the prior rail position | Warm tiles are visible in the immediate observation |
| Reopen the viewer, press Left, then Escape | Position 4388/10000 displays the 640-by-480 image; the same wall anchor returns |
| Click the Ame window close button | The app disappears and exits zero with its owned Job closed |

The native close is sent at 01:13:57.530 UTC. The host first observes the matching post-exit receipt
at 01:13:58.8306344 UTC: 1300.6344 ms using one host clock, below six seconds. Guest timestamps
are not subtracted from host input times. Across 1909 samples, kernel peak working set is
353787904 bytes, sampled peak private memory 444911616 and kernel peak paged memory 489615360.
The minimum host availability is 3576250368 bytes, above the retained floor.

After app and Job retirement, a second complete guest source check verifies every path, byte hash
and date in 55291 ms. The closed derived catalog is then copied out. Read-only SQLite quick-check
passes, with one expected root and exactly the 10000 active relative paths, without duplicates.
The catalog verifier separately accepts inactive older rows and rejects missing, extra, wrong-root
and duplicate active rows. Its first fixture cleanup attempt exposed an unclosed Python SQLite
connection; explicit connection closure corrects the diagnostic and the boundary cases then pass.
Full host source verification passes afterward in 57.728 seconds for all 10516 retained generated
files, including the unchanged 10000/512/2/2 active catalogs.

## Host retirement failure and evidence limits

Guest source/catalog finalization finishes and requests guest shutdown at 01:14:53.6172709 UTC.
The interactive Sandbox Client remains at a connection-lost dialog, error `0x80072746`, asking
whether to submit feedback. Consequently the host parent fails its unchanged 900-second deadline;
after its cleanup allowance it records 931514 ms and surviving client PID 5768. This is a failed
complete lifetime even though app exit, catalog and source checks passed. No failed receipt is
rewritten. Declining feedback through the observed dialog at 01:17:24.061 UTC sends no report;
at 01:17:44.7675017 UTC all Sandbox processes are confirmed retired.

The screenshots establish selected actual Release pixels, delivered Left/Right/Escape inputs and
stable observed return geometry. They do not cover every animation frame, all twelve dimensions
in the original viewer, pending-request reversal, source-slot accounting, cache-key ownership,
root/search/sort changes or complete menu/Tab focus return. The viewer and rail sequences map to
partial UX-04A, UX-05A and UX-08A evidence, not blanket acceptance of those variants. Existing
controlled race/ownership evidence remains separate. This VM does not prove host GPU performance,
signed-service, real Journal, Cloud Files, retained-library or R2c acceptance.

The 100-minute reservation is charged in full through 4734 minutes. The failed parent is closed;
another media lifetime needs the recorded changed retirement method rather than an unchanged replay.

## Normal host-close calibration

One empty guest, `f409552af3f142df8e7c23b97da1817d`, verifies the changed ending. The helper writes
its fresh-profile/read-only checks and exits without shutting down the guest. Native input closes
the observed Sandbox window and confirms disposal of that empty guest. The host then records no
surviving Sandbox process and passes in 112801 ms, below the unchanged 180-second deadline.
Host entry is 7675490304 bytes and minimum availability is 3641511936; guest input denies writes
with code 5. No application, image source or catalog is mapped in this calibration.

Prelaunch review identifies and corrects two diagnostic evidence bugs: a completion poll could
accept retirement after the deadline, and rerunning consumed configuration could overwrite the
first receipt. Completion now rechecks the stopwatch; consumed admission is rejected before launch
and final output uses `CreateNew`, with lock/wrapper disposal retained even on output failure.
The actual post-pass rejection check starts no Sandbox and preserves the exact original receipt.
All three diagnostic scripts parse; guest/preparation/runner sizes are 54/42/91 lines, with no
product changes. Independent method and result review confirms the bounded calibration only.
Charge the full 25-minute reservation through 4759 minutes; the earlier media timeout stays failed.

Under that run's ignored fixture root, `host-canary.json`, `native-close.json` and
`replay-rejection.json` have SHA-256 values
`A52E558AD4B793196A4E17444E587E2A44F0E0CAE31FEF59BF3AE1BEB6A2935D`,
`1BD39DCCC592A2ABDCC56828ADEBCB79D788AE81FBA5941D27F7EE4D58BAB7C0` and
`1CFABDFDDB4B2B1CB934BD88F77F869D15BF1C54DB5C949054CCF4ABBB977EAF` respectively.

## Complete native lifetime with normal host close

The adapted media method passes independent prelaunch review, but admission
`d1397b53c38d4c70b66a646181c945ad` is rejected before launch when available host memory falls
below 7 GiB. A preceding observation of 7540916224 bytes does not override the fresh entry check.
The rejection finishes in 637 ms, creates no host-start or guest receipt, and leaves no Sandbox
process. Its host result is retained; the unsampled minimum-memory sentinel is not a measurement.
No application or media run occurs in that rejected admission. After resources recover, fresh
configuration `6391803559a640b48de06f59a7fba670` consumes the same bounded reservation and retains
all existing thresholds. Actual entry availability is 9257422848 bytes; the completed lifetime's
minimum is 4522549248 bytes. The guest NTFS copy verifies the same 10000 files and 10921494393 bytes,
including all hashes and dates, in 127673 ms.

The native picker confirms import at 01:46:42.487 UTC. A captured screen at 01:50:29.490 UTC shows
10000 images, completion and decoded tiles. The written observation is conservatively recorded
at 01:50:39.332 UTC: 236.845 seconds after confirmation, within the 300-second bound. The observation
gap is not measured scan latency. A middle-rail click reveals the 2016-01-02 region; all exposed
tiles decode without wheel input. The viewer opens `04697-3840x2160-0466.jpg` at 4385/10000;
actual Right changes to decoded `04696-1024x1024-0466.jpg` at 4386/10000. Escape restores the same
observed wall geometry and rail anchor. This lifetime does not add a cold/warm reversal or Left
observation to the earlier, separately retained sequence.

Actual pointer input opens the sort menu and Escape dismisses it. Subsequent Return does not
reopen it, including a later fresh observation. Keyboard focus return is therefore **not proved**;
no internal focus owner is observed and no product cause is assigned. UX-08C retains this exact
negative sequence for a keyboard-focused follow-up rather than receiving a pass.

Normal Ame close is sent at 01:52:25.732 UTC. The host observes the matching exit receipt at
01:52:26.7111950 UTC, an upper bound of 979.195 ms on one clock, below six seconds. The app exits
zero and its Job retires. Across 1814 samples, kernel peak working set is 326213632 bytes, sampled
peak private memory 239685632 and kernel peak paged memory 339480576, within the 2 GiB ceiling.
The complete guest post-check verifies all hashes and dates in 68234 ms before the closed catalog
is copied out. SQLite quick-check and exact 10000 active paths under the one expected root pass.

Only after that matching final guest receipt, native input closes the Sandbox host window and
confirms disposal at 01:54:38.309 UTC. The parent exits zero in **840953 ms**, below 900 seconds,
with no remaining Sandbox process, no guest failure and no cleanup failure. No force-kill or guest
shutdown is used. The later complete host source oracle passes in 67.426 seconds for all 10516
generated files and unchanged 10000/512/2/2 catalogs. This establishes the normal lifecycle and
selected Release interactions; it neither rewrites earlier failures nor closes the remaining
race, focus, source-slot, cache-ownership or external acceptance obligations.

The reserved independent result review confirms the matching receipts, deadline, app-close upper
bound, exact membership, source checks and retirement without a new blocker. It preserves the
unproved focus return, unmeasured scan latency and separate interaction-variant boundaries.
Charge the complete 60-minute reservation through 4819 minutes. Product source is unchanged.

## Release menu attempt without import

Lifetime `ee618de8ac42428195336fb7a0899d73` reuses the unchanged Release payload and reviewed
NTFS-copy/normal-host-close method. Entry availability is 8013615104 bytes. Guest preparation
verifies all 10000 generated files and 10921494393 bytes, including hashes and dates, in 126141 ms.
The actual picker opens, but two text-input calls return without visible text in its folder field.
Observed pointer navigation reaches and selects `mixed`; import is never confirmed. No cause is
assigned to the absent text delivery, and no menu key sequence is attempted.

The last optional action is at 03:01:04.260 UTC. No further optional input is sent after the
600-second cutoff at 03:03:09.4468475 UTC. Retirement nevertheless starts late: picker Cancel is
sent at 03:04:29.216 UTC, followed by normal Ame close at 03:04:38.725 UTC. The intended full
300-second retirement reserve is therefore **not achieved**. This controller pacing failure and
the unperformed import/menu sequence leave the complete interaction attempt unsuccessful.

The matching app-exit receipt is first observed on the host at 03:04:40.3123003 UTC, giving a
1587.3003 ms close upper bound on one host clock. The app exits zero and its owned Job retires.
The first native-action receipt truncates that interval to 1587 ms through JavaScript date parsing;
the separate precision receipt retains the exact value without overwriting the original. The
complete guest source post-check passes in 71956 ms before copying the closed derived catalog.
Native input then closes the Sandbox and confirms its disposable-guest closure at 03:06:27.351 UTC.
The post-close capture reports no screenshot target; the matching host receipt independently
confirms no remaining Sandbox processes and no cleanup failure. The parent completes in 800253 ms,
within the unchanged 900-second deadline.

Across 1786 guest samples, peak working set is 166203392 bytes, sampled peak private memory
112742400 and kernel peak paged memory 113143808. Minimum host availability is 3315232768 bytes;
the existing resource bounds hold. The required 10000-member catalog verifier correctly exits one
with `Unexpected root or root count`. A separate read-only observation confirms SQLite quick-check
passes with zero roots and zero locations after cancelling the picker; this empty state is not
import acceptance. Full host verification subsequently passes in 65.421 seconds for all 10516
generated files and the unchanged 10000/512/2/2 original test catalogs.

Scoped independent result review confirms the unsuccessful interaction, late retirement start,
exact close-time correction, preserved source/catalog checks and published receipt hashes.

Charge the full 50-minute reservation through 4949 minutes. Preserve the earlier successful
Release decoding/lifecycle evidence and every failed attempt. UX-08C's Release focus return remains
open. Another attempt must change the preparation/input pacing and provide a feasible retirement
schedule before launch; repeating this unsuccessful sequence unchanged is not admitted.

## Pointer-first schedule checkpoint

The next preparation is complete, although its caller incorrectly treats PowerShell's unrelated
`LASTEXITCODE` as the script result. Exact prepared configuration/artifact checks pass without
rerunning consumed preparation. Admission `d91b926403f6487f8261127c3a848a0b` then independently
rejects insufficient entry memory in 601 ms, before any Sandbox launch. Its unsampled memory
sentinel is not a measurement. After resetting the completed JavaScript test session, fresh
admission `84b63ca33a0e4324b7eaa70cb49ee323` passes with 7519236096 bytes available. This sequence
does not assign the fluctuating host memory to one cause or change the 7 GiB entry requirement.

All six observed pointer actions succeed: open picker, navigate upward, enter the local drive
through This PC, enter the generated parent and select `mixed`. Selection arrives at 245.786
elapsed seconds, after the 240-second import-admission boundary. Import and menu keys are not sent.
The picker is cancelled and normal app-close input is sent at 296.966 seconds, preserving the normal
retirement reserve. This is another unsuccessful interaction schedule, not a product import or
menu failure. The complete required membership verifier retains exit one; the copied catalog's
read-only quick-check passes with zero roots/locations, consistent with the cancelled picker.

The same-host close upper bound is 2082.8556 ms. The app exits zero, its Job retires and the parent
finishes in 401434 ms with no surviving Sandbox processes or cleanup failure. The 10000-file
NTFS pre/post checks preserve all bytes and dates in 123410/65717 ms. Across 484 guest samples,
peak working set/private/paged memory are 163287040/110030848/110555136 bytes; minimum host
availability is 3214307328 bytes. Complete host verification passes in 54.575 seconds for all
10516 generated files and the original 10000/512/2/2 catalogs. Charge the full reservation through
4994 minutes. The next method changes fixture placement to the actual picker's initial directory;
it cannot lower the workload, resource limits, import bound or retirement requirements.

## Direct fixture admission missed

Lifetime `9e53f68fb38a4512ae7e0ff9e57b7f23` places the unchanged generated corpus in the fresh
guest's ordinary Documents directory. The guarded Known Folder/destination check and method
review pass. Entry availability is 7570956288 bytes; the 10000-file, 10921494393-byte NTFS
copy verifies hashes and dates in 127788 ms. The actual Release client starts with an empty
catalog. Input orchestration does not resume before the 240-second import admission, so neither
the picker nor a menu sequence is attempted. This is an unsuccessful test schedule, without
evidence of a product import or menu defect. The changed placement itself remains unexercised
through the picker.

Normal app-close input is sent at 03:42:12.896 UTC, 395.019 seconds after host admission. The
matching host exit observation at 03:42:14.5232567 UTC gives a 1627.2567 ms close upper bound.
The app exits zero and its Job retires. The guest verifies all source hashes and dates again
in 67316 ms before copying the closed catalog. Native Sandbox disposal then completes the parent
in 507063 ms, with no surviving Sandbox process, failure or cleanup error. The post-disposal
capture has no screenshot target; the independent parent receipt proves complete retirement.

Across 755 guest samples, peak working set/private/paged memory are
117903360/100196352/101543936 bytes; minimum host availability is 3485118464 bytes. All existing
resource bounds hold. The required imported-membership verifier exits one with
`Unexpected root or root count`. Separate read-only inspection confirms quick-check passes with
zero roots/locations, which does not accept the required imported corpus. Complete host integrity
verification passes in 55.303 seconds for all 10516 generated files and the original
10000/512/2/2 catalogs.

Charge the full reservation through 5039 minutes. Stop this Release-menu method series without
another automatic lifetime. UX-08C's remaining Release sequence stays open alongside the other
frozen duties; prior selected Release decoding and normal-lifetime evidence remains separate.

Independent result review confirms the matching receipts and hashes, failed interaction and
membership result, exact close bound, complete source checks and stopped method series.

## Local provenance

Ignored `.build/r2c-release-native/` owns the scoped guest preparation, process/source validators,
catalog boundary cases and artifact admission. The NTFS run's
`build/integration-storage-74b9cbe65f1b4e50b3770af8c2db6fea/` contains:

| Receipt | SHA-256 |
| --- | --- |
| `host-result.json` | `91610756151B6F599F8B60ECE5F579A1A7B906BDAF2D4769A28289D03B86D173` |
| `output/guest-result.json` | `9603D7BE0187E553B25C65C3AC89658DC3506115D17F73A4A66A721629710527` |
| `output/guest-source-post.json` | `A20E62F2BBBAAB85F0AC357B84B62E4708805E0664B45F67859C13465E7A1761` |
| `output/native-actions.json` | `251D98844CC4E0F20097406DD94E577D1CA6346B916B60500F35622C23370CFE` |
| `catalog-verification.json` | `681799B022C75ED10608B8B61B68D7BFAC235622DA6F564F5424E112D8ADF8DD` |
| `retirement-followup.json` | `A8DAD20192A54620EF9926A80651D1B0A69256CD88A1C0635594AD51F66450C1` |

The post-run host source receipt is
`build/integration-storage-e84f07c4443e4008b0c71381991477a4/source-integrity-1790126354051161000.json`,
SHA-256 `8B7CF0A9756DC5A61195A77518DA4FF1EF29345BA111E66DC4B138162918EB16`.

The completed normal lifetime is retained under
`build/integration-storage-6391803559a640b48de06f59a7fba670/`:

| Receipt | SHA-256 |
| --- | --- |
| `host-result.json` | `AE451D35C6B887F21260F1F2FE67A64DC6A96C5FECAA11B2855E9B300C31A131` |
| `output/guest-result.json` | `9D5DEDD280C3CD1F53B0AAD9F79FC760E3435B87F3923B4A3F3A40F009100FA0` |
| `output/guest-source-post.json` | `121BA4CC522A0239BD95488410EBDDAF6FD55B103E70CFA96664CF955A5060D2` |
| `output/native-actions.json` | `2E53503DA18C652216F53990056F475D895B785B1AF9720AE1BCC1A6A59490EF` |
| `native-close.json` | `36A5BFDB726D051A6FC8F9C6BAFC8075E4CFBAB9E59A06770D0594F5C5E7C3EE` |
| `catalog-verification.json` | `0184205925C8FBE3553E1797791A35C1513A49443A9717F2E40982BF708797F4` |

Its complete host post-check is
`build/integration-storage-e84f07c4443e4008b0c71381991477a4/source-integrity-1790128633352623300.json`,
SHA-256 `5474BBD93A9553831560E8D2A823E42DB77367C86DC4FE1D03F9F8C2A54C849C`.

The unsuccessful menu attempt is retained under
`build/integration-storage-ee618de8ac42428195336fb7a0899d73/`:

| Receipt | SHA-256 |
| --- | --- |
| `host-result.json` | `B9B864FD49AE3A4A19763EFB807359AFB0646BAA375AEE264A26FDE9E6E66020` |
| `output/guest-result.json` | `1C40F44E5EFC32AE90726CDEB28400278FED2E1FC9DAF438379F96AE9560D18D` |
| `output/guest-source-post.json` | `A51B84C4D089BDD3312AB1CCD88C458AAF941B1A410DA1D9AA477E55E255D0E3` |
| `output/native-actions.json` | `00203CE2A4A7C08DF68586DF70C7278D61CD1BA8D7258786A5A89B482306BE04` |
| `native-close-precision.json` | `E5FE6DF6B0947065B01AF3AB41543BD20AFB8D63CA1074C81426C0939705A54E` |
| `unimported-catalog-observation.json` | `2CFF79B9469BB95F02C01AFDFF85B3185BBEAFF9314C4F34B716C5CB105DE75C` |

Its complete host post-check is
`build/integration-storage-e84f07c4443e4008b0c71381991477a4/source-integrity-1790132958956675200.json`,
SHA-256 `00A058FF4249907E909924261A0563C57A5FD617BC503DE012878A17416BAD21`.

The pointer-first checkpoint is under
`build/integration-storage-84b63ca33a0e4324b7eaa70cb49ee323/`. Its `host-result.json`,
`output/native-actions.json` and `native-close.json` hashes are respectively
`54C102E94F2722DC9DFDDBEED48CE997F620EB1D1A5FDA2E804DF3502A8B7E57`,
`B423479544FA5ED062707D1C90532550D38D69DAC40F2115BC6770FADF70496B` and
`64A91474BF2AE1CD67C443D1F07FD0AE9ADD7816A46A988CB7F02965F3DAC13E`.
Its complete host post-check is
`build/integration-storage-e84f07c4443e4008b0c71381991477a4/source-integrity-1790134158551346200.json`,
SHA-256 `8E27EC0D1F351B9287B268AD5A7BE4C7A2956CC20452709CA224B9AB606ADDF7`.

The direct-fixture attempt is retained under
`build/integration-storage-9e53f68fb38a4512ae7e0ff9e57b7f23/`:

| Receipt | SHA-256 |
| --- | --- |
| `host-result.json` | `DE699C2E9AE7278AB43414D51ED245F588378DD1710E2C4F34BDDF2EC3C76C06` |
| `output/guest-result.json` | `8ACDB4B6FD350ECEB10FEC043368B02BB22AF9C218BC945A0F2B37BF89FF817E` |
| `output/native-actions.json` | `7E0A5F48B1466951D75B773006BF051E433E2E9B77A6D4057B68D1FDCF44F95D` |
| `native-close.json` | `86C053A831448C0D4185AE47836AA42F713A71CD3AA37227DF793C7033E9106B` |
| `unimported-catalog-observation.json` | `9E954E33ED6843DEEE51EDFE81CD4E461722E3D13FB31D7CC1A92B22AFE9AF00` |

Its complete host post-check is
`build/integration-storage-e84f07c4443e4008b0c71381991477a4/source-integrity-1790135178678719000.json`,
SHA-256 `99B7F3DEA6031817D747C6AF14A7861CE8A1FDCC8749E81DD0CAED0435959435`.

## Native keyboard sort return on current Release

Run `3b0f46e1e0874343bcd1fe5dabe5f752` uses product commit
`b0d1bda4cf9f950de2567099a6a7245195411fbc`; only the execution plan is dirty at build time.
The canonical unsigned Windows gate passes from 09:34:54.1256718 through 09:36:43.3406110 UTC,
including all three runner cases, both engine-retirement cases and the catalog-free bridge smoke.
Its 19 product files total 79463844 bytes; build evidence SHA-256 is
`F545C8AF9F84B178F52A06839279C6AF225137844BF7FFC2474E156BE014D4ED`.
The optimized client retains normal guest Known Folders and the same separately verified local
MSVC runtimes. No product code or storage policy changes for this method.

The previous attempt's original tool timestamps show a 168.831-second gap after the application
startup call. This method completes preparation before parent admission, binds the native window
during copying and polls readiness in at most 20-second observations. Method review corrects
three diagnostic boundaries before launch: recheck input deadlines after asynchronous logging
and after tool return; publish the flushed startup receipt atomically without overwrite; and bind
every observation/action to the selected window and a valid screenshot. Six focused cases cover
deadline, invalid-clock, wrong-window, missing-image and receipt-publication rejection boundaries.
These controls neither inject product actions nor substitute for observed UI behavior.

The guest copies and verifies all 10000 files / 10921494393 bytes in 132562 ms, preserving hashes
and historical creation/modification times. Picker confirmation is recorded immediately before
the input call at 229.109 elapsed seconds; the call returns at 09:47:55.145 UTC, also inside the
unchanged 240-second admission. All input times below use that same pre-call recording boundary;
subsequent screenshots confirm the visible results rather than their exact rendering time.
The observed completed count, completion feedback and
decoded gallery pixels are recorded 74.496 seconds after confirmation; this is a conservative
observation interval, not a measured exact scan duration. The closed catalog independently contains
the exact 10000 expected relative paths in one active root, with `quick_check=ok`.

After a pointer focus on the empty search field, 13 individually observed Tab inputs reach the
visibly focused sort trigger. Enter opens the sort menu at 450.507 elapsed seconds; Escape closes
it at 461.018, a subsequent capture confirms the menu is absent, and Enter reopens it at 484.246.
There is no intervening pointer refocus or menu selection. This proves visible native keyboard
return for sort; it does not claim observation of an internal FocusNode. A following Escape and
Tab reach layout; Enter opens it at 513.510 and Escape closes it at 522.285. Its proposed second
Enter reaches the 540-second optional-input guard and is rejected before input. Layout return and
the unperformed more-menu sequence therefore remain unaccepted. The lengthy initial traversal
consumes the available optional interval; this is incomplete coverage, not a reproduced menu defect.

Normal app-close input begins at 09:53:20.099 UTC (554.240 elapsed seconds). The host observes its
matching successful exit receipt at 09:53:21.8796024 UTC, a same-host conservative upper bound of
1780.6024 ms, within six seconds. Guest post-verification checks every source hash and date in
70656 ms and copies the catalog only after application and Job retirement. Closing the Sandbox
normally and confirming disposal completes the parent in 673373 ms, within 900 seconds, with no
surviving Sandbox process. The post-disposal screenshot fails because the target window has gone;
that capture error is retained separately from the independent successful retirement receipt.

Entry host availability is 7796301824 bytes; minimum is 3642183680, above the 2-GiB reserve.
Across 1368 samples, guest peak working set/private/paged memory are respectively
189370368/143548416/145588224 bytes. Application exit is zero, cleanup failures are empty and
the 2-GiB ceiling is retained. Full host 10516-file source oracles pass before and after in
52.640 and 51.477 seconds. No real source root is accessed or mutated.

Receipts are retained under `build/integration-storage-3b0f46e1e0874343bcd1fe5dabe5f752/`:

| Receipt | SHA-256 |
| --- | --- |
| `host-result.json` | `0403C8F80E170324DCF5165F7B9659281A7B040673FB25E83EC7F690270B9849` |
| `output/guest-result.json` | `87FC505A0723B4F0D72D55151C6FAAB62F2A4B9692F5787A662F7006102A57E5` |
| `output/native-action-events.jsonl` | `3026C0FC24F995666DE20B695BD76B4B98F810DC77FEA5936AC6C70192D130D8` |
| `output/native-observations.jsonl` | `2CC6A16E083071D0F1CE879987358F0DF6F29695B08AFF4594B1708E944CC9D4` |
| `catalog-verification.json` | `93F5A322543638A36C09C762CA7C87EE417E200D11D88B4B62FF3665CE8B8F8E` |
| `preparation-evidence.json` | `74111A69A6B6EAA1D1DA6E87178A67D9BF3B9A7788CDCED7C3FE3777549F53C9` |

The host post-check is `source-integrity-1790157432102399300.json` under the retained generated
fixture `integration-storage-e84f07c4443e4008b0c71381991477a4`, SHA-256
`93F8FF23D610C4B0FC8BE798C5CC3F6CC1E487DABFDF8D03D96F0AF7E6901FD7`.
The selected sort boundary and complete lifetime pass; the three-menu method is incomplete.
Independent result review verifies the receipt hashes, 27 input pairs, source checks and retirement
timing. It does not independently inspect the original screenshots or decoded pixels. Its two
record corrections distinguish pre-call input times from visible-result times and the prepared
viewer method's open reservation from a claim of zero work.
Remaining Release workflows, C11, C01/C02, full Daily, accumulated review and external acceptance
retain their independent obligations. This result does not erase any previous failed attempt.
