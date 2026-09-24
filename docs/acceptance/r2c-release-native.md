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

## Current Release gallery stability and incomplete menu traversal

Run `ae88ab1153734f1595ec34057270ca19` uses commit
`f66d2a6d1078d1692185a305c13a114ed4492727`, whose product source equals C11's `a1c165b`.
The first canonical unsigned invocation builds the application and passes three runner and two
engine-retirement cases, then its outer Windows PowerShell 5.1 merged `Tee-Object` capture treats
Cargo's ordinary stderr as `NativeCommandError`. It is a failed gate with bridge smoke unreached;
`.build/r2c-release-gallery-current/failed-pipeline-evidence.json` preserves that result.

A small controlled process reproduces this capture failure with ordinary stderr and exit zero.
The first separate-capture probe also exposes a missing retained process handle; the corrected
probe keeps that handle and verifies stderr plus exact exit codes zero and 17. The unchanged
canonical gate then passes with separate stdout/stderr files from 19:13:12.2166361 through
19:14:34.7973626 UTC, including the Release bridge smoke. Only the execution plan is dirty.
The 19-file payload evidence hash is
`A052ED73ECEAE0364242B7FB762B82357B7318378E2CE77E9B542B51AEFC8506`.
No product, gate assertion or error policy changes. Eight reviewed native/guest helpers remain
byte-identical, and their six input/admission checks pass before the single fresh lifetime.

The guest copies and verifies all 10000 files / 10921494393 bytes onto ordinary NTFS in 92706 ms.
Actual picker confirmation occurs at 181.963 elapsed seconds; the complete count, completion
feedback and decoded gallery are observed within 51.071 seconds of that input. This is an
observation upper bound, not exact import latency. The closed catalog independently passes
`quick_check` and contains exactly the expected 10000 relative paths in one root.

Actual rail clicks occur at 233.036 and 253.200 elapsed seconds. Their displayed tooltip targets
are December 2012 and January 2019 respectively; action labels name the nearby year labels, not
an exact calendar selection. The next inspected captures show current decoded thumbnails without
any wheel input, at 19:20:24.347 and 19:20:46.200 UTC. Loading captures retain the rail and viewport
boundary. The second settled capture and the one at 19:21:08.761 show the same gallery arrangement,
date heading and slider position, with no intervening input. These point observations span 22.561
seconds; they do not establish continuous frame stability or replace C11's Debug race/geometry
oracle. No Retry preview or persistent blank wall is observed in these captures.

The remaining layout/more keyboard return sequences are **not reached**. Two reverse traversals
do not establish a visible target, then forward traversal and a source-selection attempt consume
the optional interval. The source selection returns its own root gallery to the top. Later keys
show window, library, settings and source controls, but no layout/more Enter/Escape/Enter sequence
is sent. This is incomplete input coverage and orchestration, not proof of a menu defect or input
success. Preserve the gap; do not launch another unchanged menu-only VM from this result.

Actual normal Close begins at 513.024 elapsed seconds, before the 540-second cutoff. The same-host
observer sees its matching successful exit receipt within **1177.024 ms** of input. Guest source
postcheck verifies every hash/date in 48321 ms and copies the catalog after app/Job retirement.
Sandbox Close and confirmation then complete normal parent retirement in **623081 ms**, within
900 seconds; no Sandbox process remains. The final post-click capture reports an unusable window
after the disposal input; this error is retained separately from the successful process receipt.

Stderr is empty. Entry availability is 8799993856 bytes and minimum host availability 4359872512.
Across 1361 samples, guest peak working set/private/paged memory are respectively
317714432/233209856/331984896 bytes, below the two-GiB ceiling. Host source checks pass all 10516
files before/after in 52.963/46.010 seconds; the post receipt is
`source-integrity-1790191670168512700.json`, SHA-256
`ED71943E77086D74D0702C8191BCF959C551D80F30567732411644ABB19D20D2`.
No real source root is accessed. Receipts remain in
`build/integration-storage-ae88ab1153734f1595ec34057270ca19/`:

| Receipt | SHA-256 |
| --- | --- |
| `host-result.json` | `905270BEE61EAB1B597980958FBACF209D3BFD5A4B1C2728FB7BF23D1551F2A6` |
| `output/guest-result.json` | `1D2F25A338967C4204505DFA7C974746A8294CB50E3C4CAA1940FA4240C79A9E` |
| `output/native-action-events.jsonl` | `36FC8F60B3658DE29DC680DD43C31FF60FC03A4BAD2A85CA693E054C8E432D7D` |
| `output/native-observations.jsonl` | `47FE015FD1EF99E8057A1A931B7DBAB09E660CAC6D17709E7B8754392243907D` |
| `catalog-verification.json` | `9F448CBDA1A8C8B734C9D3B245AD524D6439F20BBD16CAEC09E34455CCF3C2D1` |

Charge the 90-minute reservation and 15-minute capture correction in full through 6979 active
minutes. This adds current optimized-client gallery and normal-lifetime evidence only. Menu
return, other frozen variants, C01/C02, complete Daily, accumulated review and external acceptance
retain their independent obligations.

Independent result review verifies structured receipt hashes, bounds, source checks and the
preserved capture failure. It does not independently rejudge the original screenshots' pixels,
geometry or focus. No blocking discrepancy remains within that reviewed evidence scope.

## Release bulk addition and unreached removal

Run `e1963049cd3f4c969f3c18f4225142bc` reuses the exact current unsigned Release payload whose
evidence hash is `A052ED73ECEAE0364242B7FB762B82357B7318378E2CE77E9B542B51AEFC8506`.
The live branch is `codex/r2c` at `b1d31d0`; product/toolchain inputs remain identical to its
recorded build source `f66d2a6`, including C11's `a1c165b` product changes. Only documentation
and ignored diagnostic files change. The normal storage resolver, generated corpus, memory
limits, source protection and 900-second parent remain enabled.

The new guest stimulus owns twelve baseline copies and 2000 added copies from the frozen corpus.
Only the first 1500 added names are eligible for removal after matching path, content and date
checks. A held process/Job owner retires the helper independently of the application. Preflight
review corrects partial receipt publication, a normal-stop race, omission of the legal 128-pixel
preview bucket, publication-boundary timing, missing complete-import evidence and cross-VM close
time subtraction. The first process-owner guard exposes an unpinned process-handle exit result;
the corrected owner retains the handle and all three lifecycle cases retire. Seventeen source/
stop checks, four receipt/cache checks, eight input checks, sixteen Python oracle checks and the
three native helper cases pass. The source metadata oracle additionally matches all 10000 rows
in the previous closed Release catalog. This calibration is separate from this run's acceptance.
The final 23-file diagnostic source manifest is
`2BE4C51D1DEC73711283EC508C2767B73FF9BEE935DADC0F915808F668ECBA1F`; all remain unchanged afterward.

Guest preparation copies and verifies 10000 images / 10921494393 bytes on ordinary NTFS in
90087 ms. Real picker confirmations occur at 165.407 and 226.592 elapsed seconds. The twelve
baseline images display before the second picker. Background completion feedback reports 10000
imported images, and the gallery displays a combined 12012 images. The background completion
observation is recorded within 109.134 seconds of its confirmation, below the 300-second bound.

The +2000 command is admitted at 236.573 elapsed seconds; its guarded physical copy takes 65706
ms. The selected `bulk` gallery subsequently displays exactly 2012 images and decoded thumbnails,
without a manual refresh. Its recorded completion observation is 117.270 seconds after command
admission. This is a conservative observation interval, not an exact synchronization latency.
A rail click at 353.849 elapsed seconds shows December 2012 rows, first with loading placeholders
and then with decoded thumbnails on the next inspected capture, without wheel input. These are
point observations; they do not establish continuous absence of blank frames or transient Retry.

**The complete bulk variant fails to execute.** No removal request is written. The addition's
own physical-copy time extends beyond the 240-second command-admission cutoff, before its
required visible completion can authorize removal. Both commands sharing that deadline was an
infeasible schedule for the measured preparation, input and copy costs. The 300-second product
convergence bound is not relaxed. At normal shutdown the helper correctly rejects `state=added`,
`added=2000`, `removed=0` and exits with failure. Its required final 512-file postcheck, cache
inventory and closed catalog copy are unreached. The final oracle rejects the failed run; no
catalog-membership, preview-ownership or complete bulk-acceptance pass is claimed.

The remaining layout keyboard sequence is also incomplete. A pointer opens layout at 379.955
seconds and Escape closes it at 394.572. After an absent-menu capture, Return at 422.154 does not
reopen it in the following captures. This setup does not establish the visibly focused keyboard
trigger required by the retained menu method. Tool delivery and pointer hover do not prove that
focus. No product root cause is assigned, no second Enter is repeated, and More is not exercised.

Normal app-close input starts at 449.012 elapsed seconds. Its matching app-exit receipt has PID
3208, exit zero and app/Job retirement; the same-host first observation bounds close by
1483.2008 ms. Every original guest image passes its hash/date postcheck in 48500 ms. The helper
returns its explicit incomplete-sequence failure, and independent cleanup records no failures.
Native Sandbox Close and confirmation dispose the guest; the post-disposal screenshot reports
an unusable window, retained as an observation error rather than a failed close input. No Sandbox
process remains. The parent ends in 540717 ms with `processBoundaryPassed=false` because the
required guest sequence failed, not because an app or VM remained alive.

Application stderr is empty. Entry host availability is 7920062464 bytes, minimum availability
3522043904 bytes, and across 1186 samples peak working set/private/paged memory are respectively
442646528/379502592/410300416 bytes. Host source checks preserve all 10516 files before and after
in 47.932/62.336 seconds. The post receipt is `source-integrity-1790200788009942100.json`, SHA-256
`0CEA2FB02513F562ED2F51B7950BAA8CEA1CC2507393AE20062D436F48229B49`. No real source root is used.

Receipts remain in `build/integration-storage-e1963049cd3f4c969f3c18f4225142bc/`:

| Receipt | SHA-256 |
| --- | --- |
| `host-result.json` | `C21CE67F4555BDEBCAB67A23DDA0E0DFF9218F86DCAF760A52B893ECB6B0313B` |
| `output/guest-result.json` | `8E6307F84335486CF9B1195CC0C0EDABA9F0B964994C7BA7AF92019AE59F3935` |
| `output/batch-result.json` | `7E26E99F887D0BAA4908D8431894E4846ED6FE113FAFE8928E924F8108B28F1A` |
| `output/batch-add.done.json` | `9D9F6C6F4A4C6F4D97FD82E51B3B607F257DBBFBB33B850ADE87226B57833B68` |
| `output/batch-add.observed.json` | `4533652F3CF941A8A6B27E4522A9DFC14176A625A44C4799ADC680ADCA90181E` |
| `output/native-action-events.jsonl` | `36ADC0794843CC210D803A8488E17934460634E2DC7CB8453C8D171949AA0C32` |
| `output/native-observations.jsonl` | `54049D483412FC351790787D01AA882A50F58360D30698C2F58440BAAC6E7612` |
| `output/guest-source-post.json` | `B4F0EE391C9CF58E788950ED1B022886CC230B75B0F34353C1946F6C050E7734` |

Charge the 120-minute reservation through 7414. The failed method ends here; the separately
recorded measured-reserve method must pass its preflight before another lifetime. Remaining bulk
deletion, menu return, other frozen variants, C01/C02 and final/external gates remain open.

## Release bulk background preparation cutoff

The revised schedule is also **incomplete**, run `df1597c0b4d34fc998e0eb55c5fddd1f`, with unchanged
product source and payload. It retains the 240-second import/add cutoff, 390-second removal cutoff,
690-second optional-input cutoff, each 300-second convergence bound and the 900-second parent.
The method's input outcome owner distinguishes failed input from capture failure after input
returns. Its oracle rejects missing/mismatched before/after pairs, unknown input, foreign windows
and runs, and nonfinal or unproved disposal. Eleven Node, 17 input-evidence and ten catalog checks
pass; all copied PowerShell sources parse. Independent method review closes a trailing unpaired
input gap before launch. These checks do not establish functional acceptance.

The full guest copy/hash/date preparation takes 123385 ms. Native baseline confirmation occurs
at 228.165 seconds, with twelve decoded images subsequently observed. The second picker cannot
complete before 240 seconds; neither batch request is published. The original sequential method
therefore still lacks enough admission reserve. This is a test-preparation failure, not a reproduced
product synchronization failure, and no batch performance result is claimed.

The client exits normally with code zero and closed Job; the conservative same-host native-close
upper bound is 1182.1271 ms. Peak working set is 453124096 bytes, sampled private bytes 378507264,
and peak paged memory 408133632 across 285 samples. Host availability starts at 7847137280 bytes
and remains at least 3737616384. The complete guest background postcheck passes in 65683 ms.
The stimulus helper correctly exits with failure because its full sequence was never admitted;
its required final-512 verification, cache inventory and catalog copy are unreached. The parent
retains `processBoundaryPassed=false`, finishing in 379317 ms with no remaining Sandbox process.
Final confirmation records returned input followed by the expected unusable-window capture error;
the failed parent gate cannot be waived by that expected capture outcome.

All 10516 host source files pass the 52.902-second postcheck. No real root is accessed. The complete
Release bulk variant remains open. Preserve this failed run and its 25-source/38-input manifest
unchanged; its 60-minute reservation is charged through cumulative 7474. The next plan changes the
preparation dependency, rather than extending another deadline or replaying this sequence.

Evidence under `build/integration-storage-df1597c0b4d34fc998e0eb55c5fddd1f`:

| Receipt | SHA-256 |
| --- | --- |
| `host-result.json` | `AA5DD5247B44B8531096004FBB3C7B0A638EDED1980E4819899E08632258B4CD` |
| `output/guest-result.json` | `D94803BB3CEB34729AE627F24B6565546B9B8EBA4551344DAA7B0B3CBD01AEB5` |
| `output/batch-result.json` | `69B38C3899FB90D320B69E41397BB4080A29288BD6EDBF1DF6B63BC26ECFDCC5` |
| `output/guest-source-copy.json` | `8188463A6F825B633D86062E54082628FB436C8C851661E20400EB1259D1F502` |
| `output/guest-source-post.json` | `D62D33BB438FBFA81CABBA6ED5E027F6C0002FAF9D97FEDEE6F212F8D2B77441` |
| `output/native-action-events.jsonl` | `95883BAAC1716ECAEE6A8FDA34DEE8475169DB323B32B4634B004F45CE5B4996` |
| `output/native-observations.jsonl` | `FF5423C0F6B9935D92AAB7C96083FF28DFC18BBD4858A947EAF58FA76AEDF7E8` |
| `run-assessment.json` | `C449388A309BDCA165216239D56B421AED117F5899CB604CA9BD403A4E9D32FC` |

The immutable helper manifest is `.build/r2c-release-bulk-scheduled/reviewed-source-manifest.json`,
SHA-256 `41BDC1DC42E96741D6CD209FB5E3395D11B9245294A5F861874AE1BC38B617EA`.
The complete host postcheck is `source-integrity-1790202365475602100.json` in the retained generated
fixture, SHA-256 `14A4A3B9F033EE60F21289D8A4CF2CDD7E05C66741ADE47430F285885563A3B5`.

## Release bulk independent preparation preflight

The next method has a fresh single-use fixture `2952b5cb20ce48caaf46d7387e1bdb7d` and unchanged
verified Release payload. The baseline is independently copied and checked from the read-only
mapping. A separate source-preparation owner holds its child/Job until complete background byte/
date verification and successful exit; only its matching retired receipt admits the second picker
confirmation. Native input checks the deadline again immediately before sending that confirmation.
The baseline and background remain distinct prerequisites.

Fourteen Node cases, 31 Python oracle cases, 17 source-protection checks and six actual preparation
process cases pass. The latter cover complete, foreign-run, wrong-PID, partial-source, failed and
pending preparation, with owned retirement in every case. Independent method review identifies
and closes the read-to-input deadline race; the actual session regression crosses 240 seconds
during receipt reads and proves zero input, no acknowledgement and no additional capture.
All PowerShell sources parse. This is diagnostic-method verification, not a product or native pass.

Diagnostic production/inline-test/dedicated-test sizes are 74/0/72 for the preparation owner and
its guard process, 20/0/0 for the guest preparation entrypoint, 184/0/0 for the guest composition,
165/0/96 for the native session, 11/0/22 for readiness and 14/0/36 for the offline preparation
oracle. The native-session tests use simulated input only for the admission guard; real input
acceptance still requires the fresh lifetime.

The immutable manifest is `.build/r2c-release-bulk-pipelined/reviewed-source-manifest.json`, SHA-256
`59F318CF084A593365C550E1A99A6799921BAB712C2C10E8235A557D96AC69D5`, with 33 source and 40 input files.
Preparation and review are complete. The preliminary memory check reaches the entry threshold,
but the final guard rejects this configuration before writing `host-start.json` or launching the
VM. Its host result records `Release Sandbox requires seven GiB available at entry`, 419 ms,
`processBoundaryPassed=false`, no guest and no remaining Sandbox process. The guest output directory
is empty; no application, source preparation or stimulus has run. The minimum-memory field is the
unobserved sentinel, not an actual memory measurement.

The failed `host-result.json` under `build/integration-storage-2952b5cb20ce48caaf46d7387e1bdb7d`
has SHA-256 `E18906C9EAF940560706B78C6D256C8CEA35AA393E81A69D1140C7B8FA6E283A`.
Preserve it and its single-use configuration. This entry rejection supplies no native functional
evidence; a fresh configuration with the unchanged reviewed closure still requires live resources
and one complete admitted lifetime. No product gate or resource threshold is relaxed.

Replacement configuration `8b398bce286d4509a47b91bd81d31049` is prepared with the same verified
payload and 33 byte-identical reviewed source files. Its 40 input files are frozen in
`.build/r2c-release-bulk-admitted/reviewed-source-manifest.json`, SHA-256
`E22F0E3E718038FB543AE79A444A6CA95FF3551223BDB6799BE75607B12E9B78`.
It subsequently completes the admitted lifetime below. The focused method checks remain applicable
to the unchanged helpers; they do not replace the failed functional result.

## Release bulk deletion convergence failure

Run `8b398bce286d4509a47b91bd81d31049` uses the same verified unsigned Release payload and the
33-source/40-input closure above. Host admission starts at 2026-09-23T23:00:33.2810756Z with
7547191296 available bytes. Both real picker confirmations fit the original 240-second bound,
at 92.563 and 193.181 seconds. The independent background preparation retires successfully in
134471 ms with a matching PID and closed Job before the second confirmation. The complete
10000-file, 10921494393-byte background is observed imported within 151.221 seconds of confirmation.

The addition is admitted at 203.923925 seconds; its guarded physical copy takes 107210 ms.
The selected root automatically reaches 2012 decoded images within 140.473 seconds of admission.
An actual historical-rail click then displays 2012 dates, and the next observation contains
decoded thumbnails without a compensating wheel event. Removal is admitted at 363.587925 seconds,
after visible addition completion and before the unchanged 390-second cutoff. Exactly 1500 newly
created files are removed in 14218 ms; the retained source roster is exactly 512 files.

**Product removal does not converge within 300 seconds.** Observations at 295.321 and 305.997
seconds after the removal command still show 835 and 792 images respectively, with updating
feedback. No successful removal observation is written. The client closes normally at host
elapsed 678.641 seconds. Its closed catalog still contains 754 active bulk locations, including
242 paths whose source files were deleted. This is persistent incomplete reconciliation, not
only delayed presentation. The unchanged background contains exactly 10000 active locations.

The bulk queue retains 256 pending and 15 retry-wait rows. All retry failures are
`incremental_catalog_revision_changed`; the sampled attempts are one, without exhausted work.
The deletion window has 1536 rows, including 1265 completed rows. Their 1264 adjacent completion
intervals have median 250 ms; 1232 fall between 200 and 300 ms. Most completed work and all pending
work use authoritative subtree reconciliation. Current production code leases one authoritative
scope per Live worker, whose retirement and next admission depend on the external 250-ms poll.
This is a causal lead requiring a focused worker-boundary regression; it is not yet a verified fix.
The older C04 Debug gap-recovery pass remains valid for its recorded path and does not establish
this optimized-client precise-notification workload.

During deletion, two acknowledged native rail clicks at host elapsed 562.171 and 598.898 seconds
do not produce the intended historical dates in subsequent observations; both still show 2026.
These observations retain a separate navigation obligation. Returned input does not prove that
the application admitted the intended navigation, and the owning cause is not established.
No manual refresh or wheel event repairs the deletion result. Unobserved frames are not accepted
as proof that transient Retry or blank-wall feedback never occurred.

All owned processes retire. The application exits with code zero and closed Job; the conservative
same-host close upper bound is 2306.469 ms. Across 2320 samples, peak working set is 447164416 bytes,
sampled private bytes 402354176 and peak paged memory 404660224. Minimum host availability is
3153842176 bytes. Complete guest background postchecks take 67629 ms; exact retained-512 hashes
and dates also pass. The guest finishes in 729556 ms with no cleanup failures; the host finishes
in 800471 ms with `processBoundaryPassed=true` and no remaining Sandbox process. All 10516 original
host generated files pass the 57.075-second postcheck. No real root is accessed.

The complete oracle nevertheless exits one, first reporting `A native input or observation failed`.
After acknowledged Sandbox confirmation, capture returns `no screenshot targets found for process`
instead of the frozen expected `window is not a usable app window`; the recorded action is also
`confirm-sandbox-close`, rather than the expected `normal-close-sandbox-confirm`. Independent host
receipts establish process disposal, but neither this terminal capture variant nor the action name
is rewritten to satisfy the frozen oracle. Its later membership and missing-removal-observation
checks are unreached. The separate closed-catalog inspection establishes the product failure.
Process retirement and source preservation therefore pass while the complete functional variant
remains failed. Charge the 90-minute reservation through cumulative 7564 and end this method.

The remaining two-minute independent result review verifies twelve key receipt hashes, samples
five unchanged helpers, and confirms the recorded close bound and failure separation. It does not
repeat screenshot interpretation or the SQL oracle and does not establish the worker cause.

Evidence under `build/integration-storage-8b398bce286d4509a47b91bd81d31049`:

| Receipt | SHA-256 |
| --- | --- |
| `host-result.json` | `11C97E3EAEE8E29E092B8A211442182319E90C19960BCDCE508783D79C8D4295` |
| `output/guest-result.json` | `EC76411FE3F8307EE5D7FD87BD369AA9DBD5348F9E2BCFAC1B462E8DAE9973B7` |
| `output/batch-result.json` | `29E2FB54356F5920A72D78697F95AF67DF5EBC47FF7407DB3B9A046BA5C979C4` |
| `output/batch-add.observed.json` | `54B5CE618FC544924E68C01097C5E44A18C645E6D08D63D263606BA9ECBADD7B` |
| `output/batch-remove.done.json` | `ACFA0A230EBFC44FAABF3A706AB58C370ACC772B9F6139DBB7CE0E1FB7BA6B5E` |
| `output/guest-source-post.json` | `439EE0460CCCDEEBDC930DA3CF8A7879B27BA711A28BA999D796869ED9A282E4` |
| `output/batch-source-post.json` | `E87CA206DC89F646E167DFEC08AC34C2224252295A597D7E2262D3E547F8A2BA` |
| `output/native-action-events.jsonl` | `2DAA4A15DA7B46FB09ED08E07513B65F9042CCD52A1FC64F3FDDEEC8B8E7A871` |
| `output/native-observations.jsonl` | `85E482A752B023D9C463C471DEFFF9334A7C635A80F8573031F07AB25C19A626` |
| `output/catalog/ame.sqlite3` | `E044CB87DABB28A2CA3A589C3E1EFE3CE970B124C55EB754729C2E24F91336EA` |
| `closed-catalog-summary.json` | `0074EAF4E5D53B86A85AC1E191734EA53FE92256E80CE19D874F5C502EEA53FD` |
| `run-assessment.json` | `FDF8111936158EE602D2CBF3A0555B07AD45C372756519C2FC7BDE9B7DFA4A92` |

The two read-only diagnostic scripts are retained separately under
`.build/r2c-release-bulk-analysis`; they do not change the frozen run closure. Catalog inspection
uses read-only/query-only access and confirms its hash unchanged. The complete host postcheck is
`source-integrity-1790205334844090600.json` in the retained generated fixture, SHA-256
`E4FC9350C31ED525F1713D7F6EBC3DD3274FEE7C9EA705DE656DF6C7189CEF87`.

## Bounded Live worker continuation

The first causal regression uses two generated images, two already-due authoritative deletion
scopes, one external production poll and complete worker retirement before exact membership
inspection. On unchanged product source at `5300b3c`, it fails in 0.68 seconds with
`unchanged.png` still cataloged. The failed output is retained separately as
`.build/r2c-release-bulk-analysis/cadence-red.txt`. This confirms a concrete per-scope poll
dependency alongside the closed Release queue's measured cadence. It does not explain every
delay or the unresolved rail observations.

The application now has an admitted-worker owner in `production/live_work.rs` and a bounded
reconciliation owner in `live_work/batch.rs`. The coordinator retains single-slot admission,
root selection and rotation. A worker continues successfully completed authoritative scopes up to
the existing queue lease bound and a 100-ms monotonic admission quantum. An already-running scope
retains its previous reconstruction and cancellation bounds. Root generation, active publication
and catalog revision are read again for each scope, with fresh admission time. Retry, deferral,
supersession, cancellation, root retirement or error stops continuation. Ordinary path batching
is only the initial fallback; it cannot extend a partially consumed scope batch. Committed
mutation counts and a later structured failure survive together in the worker outcome.

The owning change preserves namespace/lease checks, transactions, queue classification, debounce,
retry policy, external polling, public bridge, schema and dependencies. It writes no media.
Worker timeout retains the original handle and deadline rather than detaching an executor.
No-change polling does not start this worker without ready work. Broader production runtime and
lane decomposition remains the recorded debt; this is a complete extraction of the admitted Live
worker responsibility, not a claim that the entire runtime has been split.

Independent review identifies two validation gaps and the recheck confirms both corrections.
The original strict two-deletion assertion is moved into deterministic execution of the actual
reconciliation owner with a controlled elapsed clock. It proves both real SQLite publications,
fresh revisions and durable completion times 37 ms apart, without treating a legitimate 100-ms
yield on a slow machine as failure. The real-thread production test separately retains bounded
progress and exact remaining-membership checks. The original failed test is preserved as causal
evidence; it is not described as an unchanged test turning green. Another actual-catalog case
retires or advances the root immediately after the first successful scope and proves that the
next step acquires no lease. A separate after-lease replacement case retains publication fencing.

The final `live_work` focused selection passes 24 tests in 9.74 seconds, with none ignored. These
include batch/time bounds, no-change scope accounting, cancellation before/between/after leases,
retry/deferral/supersession, partial success followed by error, ordinary path fallback, generation
replacement, root removal, peer rotation, disconnection and retained timeout ownership. Related
production namespace/ancestor-guard and running-scan cases pass 3/3; the retained-worker panic
and combined P0/P1/P2 stop/restart cases pass 1/1 each. `quality_lint.ps1` passes, including warnings-
denied Clippy and Dart analysis. Its synthetic summary-persistence warning is an intentional
negative fixture that verifies original-error precedence and lock release, not a product warning.
The PowerShell transcript is `.build/r2c-live-worker-fix/focused-and-lint.txt`; native test counts
come from the completed command output. Full Daily subsequently passes: 1534 Rust tests, zero
failures and 19 existing ignored cases in 1009.87 seconds, three broker binary integration tests,
all 88 Flutter test files, the three-test Windows scan integration, the original ten-phase
whole-window UIA contract, 15 asynchronous bridge contracts and whitespace verification. The
canonical command exits zero. Ignored cases retain their explicit manual, separately authorized
or subprocess-only duties; none is converted into an acceptance pass.

The connection-lifetime control and original complete mixed-load test both pass in that same
suite. The per-epoch control's 91-ms P95 only covers its first-page boundary; the original full
published-baseline workload separately passes with 123-ms P95 and no sample above one second.
The earlier C01/C02 failures remain historical evidence with their unresolved attribution and
final candidate obligations. This current local checkpoint does not establish hosted or retained-
library acceptance. Release client verification is pending at this local checkpoint; its later
result is recorded below.

Daily runs from 07:57:10 to 08:30:22 local time on 2026-09-24. Its PowerShell transcript and
machine-readable summary are `.build/r2c-live-worker-fix/daily.txt` and `daily-summary.json`.
The transcript does not include every native child-output line. Windows scan evidence is retained
under `build/integration-storage-c1f0c7ff135649d4beee1bd08d065373`; it exits zero in 73666 ms.
All ten UIA phases and owned process/Job retirement pass, with no cleanup failure.

Physical reviewability at this checkpoint:

| Owner | Production/source lines | Inline-test lines | Dedicated-test lines |
| --- | ---: | ---: | ---: |
| Production coordinator before | 4048 | 10488 | existing shared tests |
| Production coordinator after | 3968 | 10490 | existing shared tests |
| Admitted Live worker | 132 | 0 | 57 |
| Live reconciliation batch | 187 | 0 | 308 |
| Production worker regressions | 0 | 0 | 238 |

New owner source totals include their small conditional test-support methods and module declarations;
test bodies are physically separate. Shared priority diagnostics only switch to the owned read-only
accessor, and shared generated-fixture visibility is restricted to production test descendants.
Focused evidence, this Daily and review alone do not close the original 300-second Release
deletion bound or native input duties. The later native result below supplies the selected batch
and lifetime evidence; remaining navigation, final-source and external acceptance stay open.

## Live continuation Release preparation

The canonical unsigned Windows gate passes on clean source
`280f35bf7a661f61256c0680dc1a927fa3be2323` in 153619 ms. It builds the optimized application and
broker, passes three runner and two real-engine lifecycle cases, and passes the catalog-free
Release-DLL/native-channel bridge smoke. The evidence file
`build/quality-unsigned-windows/evidence.json` has SHA-256
`06EBF0102D640ED1C628774520E6B3DA8537A3210D72A5C99604FFED927DBA69`.
This build does not supply signed-package or installed-service acceptance.

The fresh generated-only configuration is `dcd10f777339463596fed2a436c06efd`, using that exact
payload and the unchanged 10000-file background plus 12-to-2012-to-512 batch roster. Fifteen Node
boundary tests and 34 offline Python cases pass. All helper PowerShell sources parse. Seventeen
source-guard checks and six preparation-process cases are reused only for their byte-identical
owners. Independent method review confirms the terminal-capture exception remains bound to
acknowledged final input, the same Sandbox, paired action evidence and independently successful
host retirement; it also catches and corrects a plan-only addition-deadline typo. The actual
addition/removal guards remain 240/390 seconds throughout. Six review minutes are consumed and
four remain for the result.

The immutable helper manifest `.build/r2c-release-live-continuation/reviewed-source-manifest.json`
contains 33 sources and 40 input files, SHA-256
`2982DBECFCD31917670D05613C0091C180DAC01533CBF764EE266A97B859E606`.
Only the four declared native-session/input-oracle source and test files differ from the preceding
closure. Old failed run evidence and its oracle remain unchanged. The complete 10516-file source
precheck passes in 65.034 seconds, recorded as `source-integrity-1790210343616348900.json` in the
retained generated fixture, SHA-256
`95D421B2ACBFB423992A29FFCA7578498B18E2673CE2E09A4A9EAB33B468AEAE`.

The preparation resource check at 2026-09-24T00:49:08Z has 7195938816 available bytes, below the retained
7516192768-byte entry threshold; it launches no VM or application and leaves the configuration
unconsumed. The later lifetime below starts only after resources recover and a complete fresh
source precheck passes. The earlier unmet prerequisite is not a failed product run.

## Live continuation hosted checkpoint

Run [35938990417](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/35938990417) checks head
`280f35bf7a661f61256c0680dc1a927fa3be2323`. The unsigned artifact identifies the actual tested PR
merge as `57621de5126958843d8b799ce9e837b55fd67309`. Its Git tree and the source head's Git tree
are both `7de7f451b0c7d5abadc18efb5f2b7edd1f1d6ef3`, verified through the Git commit objects.
The final result passes all ten required jobs and the aggregate gate. The earlier nine-job
checkpoint remains in its immutable partial assessment.
Signing-only jobs are skipped under the existing PR policy, without signed acceptance.

Seven small evidence artifacts are retained under
`.build/r2c-live-worker-fix/hosted-35938990417`. Each of the five synthetic summaries proves exactly
one executed passing test, zero ignored cases, exit zero, complete captured output and a test-process
peak below its existing 512-MiB limit. Source and output hashes are recorded in `assessment.json`,
SHA-256 `B18E9275DCC1DD1553F80D47A9A68B74D3A36860F51B70013716A239E270AD36`.

| Workload | Actual observations | Test-process peak bytes |
| --- | --- | ---: |
| JPEG | 6000-by-4000 fixture; full decode/resize 129.9761 ms, scaled 95.9815 ms | 81784832 |
| Seven media formats | Seven cold generations, seven warm cache reuses and seven unchanged sources | 64409600 |
| Synthetic scan | 10000 files; cold 14321 ms, warm 12030 ms, pause 2 ms, resume 14228 ms, cancel 154 ms | 25882624 |
| Journal parser | 1000000 records covered in 245 bounded pages; at most 4095 retained records | 10612736 |
| Catalog publication | 50000 identities staged/published in one commit; 146 overlapping polls, zero partial observations and one retained pending P0 | 18247680 |

Memory scope is the direct test child's kernel peak working set. Build measurements cover the
primary Cargo process only and do not establish a compiler-tree memory bound. These are hosted
synthetic/unsigned gates; the 10921494393-byte mixed-media Windows client workload,
original Release deletion deadline and external acceptance remain independent duties.

The completed Static and Rust log records 1534 passing tests, zero failures, 19 existing ignored
cases and three broker integration tests. The original complete mixed-load case passes with
25 samples, 99-ms P95 and no sample above one second. Its distinct connection-lifetime controls
measure 386 ms per poll and 82 ms per epoch at the first-page boundary; those controls alone do
not prove full recovery. `final-status.json`, `static-rust.txt` and `final-assessment.json` are
retained beside the partial assessment. The final assessment SHA-256 is
`E319CDC8ED6233C02A25BCD7116EB2D293B49F9C71658F219C48321CE1AA02EF`.

## Release Live continuation result

Run `dcd10f777339463596fed2a436c06efd` uses the exact prepared optimized payload from `280f35b`.
All 33 frozen helper sources and 40 inputs retain their manifest hashes. The refreshed complete
10516-file host precheck passes before admission; the host enters with 9578176512 available bytes.
The independent preparation process finishes and retires before the second actual picker
confirmation. No real source root, installed service, explicit rescan or manual display refresh
is involved.

| Required boundary | Observed result |
| --- | --- |
| Baseline | 12 pictures imported and visibly decoded |
| Background import | 10000 mixed-size/historical pictures, 10921494393 bytes; completion observed after 169.174 seconds |
| Addition | 2000 generated files; automatic 12-to-2012 convergence observed after 156.964 seconds |
| Deletion | 1500 generated files; automatic 2012-to-512 convergence observed after 63.746 seconds |
| Closed catalog | Exactly two roots and the expected 10000 plus 512 active members, with matching source identities, sizes and dates; no unsettled queue work |
| Preview ownership | 97 current ready previews verified; no active failed preview; 114 cache files totaling 2706495 bytes |
| Normal application close | 1315.743 ms, exit zero, owned Job and process retired |
| Complete host lifetime | 738341 ms, no remaining Sandbox processes or cleanup failure |
| Resources | Client peak working set 448622592 bytes; minimum host availability 4761661440 bytes |

Both additions and removals retain their original stimulus-admission clocks and 300-second
convergence limit. The application visibly updates its total to 12012 before selecting the bulk
root; the subsequent root selection proves its exact 2012 count. It does not substitute a manual
refresh for automatic publication. The closed read-only catalog verifies exact final membership
and complete queue settlement, and its bytes remain unchanged by verification.

After deletion, an actual 2012 rail click shows that region and then its decoded previews without
a wheel event. Opening `added-1500.png` and exiting with Escape returns to the same observed
historical layout. These are point observations, not exhaustive frame coverage or proof that no
transient Retry feedback can occur.

Two interaction gaps remain explicit. During background import, root clicks at 209.472 and
239.282 seconds leave the all-library view unchanged; selection succeeds after import completion.
The source-navigation tile receives `state.isBusy` as its browse-disable input, providing a
concrete admission-policy lead for the next bounded investigation. A rail click at 367.486 seconds
during deletion does not establish the intended historical region; the settled post-delete click
works. Its cause is not assigned to the same policy without proof. Layout pointer-open/Escape and
subsequent Tab traversal do not produce the required observed keyboard Enter/Escape/Enter sequence;
layout and more-menu keyboard-return acceptance remain incomplete.

The final Sandbox confirmation returns acknowledged input, followed by the exact missing-target
capture error for its bound process. The unchanged reviewed oracle accepts only this final paired
action together with independent successful host retirement. It does not erase the preceding
failed run or relax input failure handling. The guest verifies hashes and dates for all 10000
background and 512 retained batch files. A subsequent complete 10516-file host check passes in
56.534 seconds. The first host verification invocation stops at a misspelled manifest property
before running either oracle; the corrected invocation reads the existing `inputs` field and
passes without modifying the frozen method.

Evidence is retained under `build/integration-storage-dcd10f777339463596fed2a436c06efd`:

| Record | SHA-256 |
| --- | --- |
| `host-result.json` | `A1D9929F8F2EF195E597ECEC0476706DB673C14FA20599B3C93EAEEFEEDAF024` |
| `catalog-verification.json` | `17DF0F2FB906AA1B1A9881354E71FC5DC954D49E3A6673D056111EC8EF2D7DCF` |
| `output/native-action-events.jsonl` | `FD7AFEFE3BCF9485BEB2FE7A49DB3D46CA96ECA690A28E11D51CBC5CE6CAD8B8` |
| `output/native-observations.jsonl` | `A7DE36119CEA370CB24B3416B88CC835A8672FD3E4ABFE53D86C455C6BDC70B9` |
| `output/catalog/ame.sqlite3` | `DD3E34F53F3D32C17874B784427686AA2B5BB8DF26A4A24B52920D80E57F9A98` |
| `run-assessment.json` | `A7FEECFD151F0194A83553D4A6630C651E016B1AD8EAD9940CB823981D1686E7` |

The refreshed host pre/post records are `source-integrity-1790212173696903300.json` and
`source-integrity-1790213089356673500.json` in the retained generated fixture, SHA-256
`20D447B0271CBA73C773AFA1BBD293D01AB4CBE706157D2CC33210FF364E3B66` and
`6AD926FDCC399B61921FF2A69F1D150715A183327C0E7C5572DD6BF587706A90` respectively.
This closes the selected corrected Release batch, source-safety and normal-lifetime boundaries.
It does not close navigation during publication, keyboard-return variants, every transient frame,
the full 24-variant roster, final accumulated review or external acceptance.

The remaining four-minute independent result review verifies structured receipt hashes, original
bounds, closed membership, queue settlement, preview ownership, source checks and terminal input
paired with complete host retirement. It finds no new blocker within that scope. It does not
reinterpret screenshots or accept the unresolved navigation and keyboard observations. Together
with the six-minute method review, this consumes the reserved review allowance.

## Dedicated menu control and traversal coverage gap

Run `bc4d1d6a70234be4848e5e1e217fe807` uses the verified `a0cde9141b514f92d2a6415f633def5f9bb5d221`
Release payload. All eight reused helpers, 32 inputs and 37 product-source hashes remain unchanged;
the method manifest SHA-256 is
`A7CBA5028826ACD38C4117BF3921D351075C8EADEC2ED7BD6AF93352B8AF9377`.
The workload retains all 10000 generated mixed-size/historical images and 10921494393 bytes,
ordinary guest NTFS, production Known Folders and the original resource and timing bounds.

The actual picker confirmation occurs at 216.396 host seconds. A screenshot at 261.570 seconds
shows import completion, 10000 pictures and decoded tiles; the later explicit completion receipt
conservatively records 55.277 seconds from confirmation. Exact closed membership and read-only
catalog verification pass. The guest's complete hash/date postcheck takes 71555 ms; all 10516
host source files pass their postcheck in 67.729 seconds.

The required layout/more keyboard-return sequences **remain untested** in this lifetime.
Thirteen individually observed Tab inputs reach only sort at 508.709 host seconds. Their actual
input calls take 94–135 ms each, totaling 1387 ms, while their first-to-last admission span is
220.198 seconds. The gap includes observation and orchestration; it is not evidence that key
delivery itself consumes that time. Normal application close starts at 520.283 seconds, before
the unchanged 540-second optional-input cutoff. This is a failed test-execution method, not a
reproduced product menu failure, and cannot be replayed unchanged.

Normal application retirement takes at most 1357.7625 ms, exits zero and retires its owned Job.
The complete host boundary passes in 659656 ms with no remaining Sandbox process or cleanup
failure. Entry availability is 7947198464 bytes; minimum host availability is 3706683392 bytes;
the client's kernel peak working set is 187056128 bytes. All 20 input pairs acknowledge delivery.
Only the final Sandbox dismissal encounters the expected disposed-target capture error, with
matching independent normal-retirement evidence. Host success establishes the lifetime boundary,
not the missing menu interactions.

Records under `build/integration-storage-bc4d1d6a70234be4848e5e1e217fe807` retain these SHA-256 hashes:

| Record | SHA-256 |
| --- | --- |
| `host-result.json` | `CE4162CF55F6C9843D643CDF5E721756820ADA595DCCE0B056F555FE8AD23C7F` |
| `catalog-verification.json` | `9188DEECFFCBF0F326E2546B9434973BE9E41E95A194F486A5D38A131D9079C9` |
| `post-run-bindings.json` | `3C3FE28AE87F6ED2CD80F6560470AF09C260152FB6DB9DD240DE513D7416854F` |

### Measured natural focus route

A temporary diagnostic uses the production `AmeApp`, real widgets and ordinary mouse/key events
with 96 generated in-memory assets, a two-year virtual timeline and a generated decoded preview.
It inspects focus ancestors without calling `requestFocus`, invoking UI callbacks or replacing
traversal policy. The existing scanner fixture supplies no retained task; no scan starts. This
fixture plans native inputs and does not substitute for the 10000-file Release workload.

The three Windows-variant diagnostic cases pass. From search, 14 forward Tabs reach layout;
eight reverse Tabs do not reach a header menu and traverse photos. Right-clicking the first visible
photo, dismissing its context menu with Escape, then pressing Shift+Tab twice reaches more via
the previous-time control. No context-menu item, image selection or layout change is executed.
The focused diagnostic does not establish either menu's native Enter/Escape/Enter result.

The first two diagnostic attempts fail the decoded-preview prerequisite; the second additionally
exposes an uninitialized scan-recovery bridge in the test fixture. Bounded real-I/O frame waiting
and the existing scanner fixture correct these preparation issues. The third obtains the focus
traces but fails platform-override cleanup. The fourth uses Flutter's managed Windows platform
variant and passes all three cases. Earlier logs are retained, not replaced by the passing result.
The final diagnostic source and raw output under `.build/r2c-menu-route-planning` have hashes
`24841800954402B47AF30963DE3DA4538462E85EC570BC13FE6917F062FDD405` and
`E0FC7DA14595C5334F63F724315CB0E256BAFD2A06BA91C03E29FAC76E6CB771`, respectively.

Independent review confirms the diagnostic's pointer/key-only focus changes, retained failed
attempts and actual trace. The test asserts arrival at a header menu; the exact two-step count is
an observation from this fixture, not a guaranteed native route. The preceding native run's
bindings also agree with its original host, action and catalog receipts. No native menu acceptance
is inferred from either review.

The already-triggered hosted run
[35965449940](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/35965449940) completes on
`d629cbbed343ed95b6913269e1cea6f5c896533d` with all ten required jobs and the aggregate gate passing.
Its signing-only jobs remain skipped under the PR policy. This status does not establish the
unperformed native menu sequence, diagnose the older finalization stall or satisfy signed-package
acceptance.

## Photo-route run stops before import

Run `00ae59f967f8422db3bde63c5d5da622` retains the unchanged `a0cde914` Release payload, all
10000 generated images and the preceding deadline/resource requirements. Nine input-boundary
tests and independent admission review pass. The new 53-line input-request owner and 36 dedicated
test lines add explicit right-button translation; neither has inline tests. Seven reused helpers
are byte-identical. The frozen ten-source/32-input manifest SHA-256 is
`F41AF8359A90292C78D2621C92E88BD27FF391CB009DFF8146977E1E101A6B51`.
The complete 10516-file host source precheck passes in 57.830 seconds.

The native method fails its import-admission target, before the photo focus route can be attempted.
The final successful picker selection is at 229.280 host seconds. The subsequent `confirm-import`
call is rejected by the unchanged 240-second guard before it emits an action receipt or sends
input. There is no import confirmation, `importComplete`, right-click or menu key sequence.
The rejection has no precise timestamp field; it is bounded after the cutoff and before the
07:27:34.004 UTC retirement observation, not assigned an invented exact duration.

| Preparation or input observation, 2026-09-24 UTC | Evidence |
| --- | --- |
| Guest source copy/hash/date verification | 127675 ms for all 10000 files and 10921494393 bytes |
| Guest readiness receipt | 07:25:35.2940039, guest clock; host first reads ready at 162.981 elapsed seconds |
| Refreshed binding before admission | 07:25:45.727, needed after the earlier desktop observation |
| Start-media publication | 07:25:55.000, host clock |
| Admission's automatic screenshot | 07:25:56.761, still desktop |
| First actual empty Ame view | 07:26:05.813 |
| Open-import input / return / automatic screenshot | 07:26:20.315 / 20.754 / 20.881 |
| First actual picker view | 07:26:30.703 |
| Select-mixed input / return / automatic screenshot | 07:26:41.775 / 41.911 / 42.048 |

The empty-app-to-invocation interval is 14.502 seconds; picker-to-selection invocation is
11.072 seconds. These operation-side intervals are distinct from input delivery and application
rendering. The automatic screenshots before the app and picker appeared require later state
observations; they are not evidence of a stalled import. This failure must not be conflated with
the earlier reported post-validation finalization wait.

Ame closes normally at 302.799 host seconds. Independent host observation bounds its exit at
1363.8873 ms, with exit zero and owned Job retirement. All guest hashes/dates pass in 69131 ms,
and the closed database is copied only after exit. The original catalog verifier then exits one
with `Unexpected root or root count`; unconfirmed import does not satisfy its 10000-member duty.
The functional verdict remains failed despite successful retirement.

The final Sandbox confirmation acknowledges input at 07:30:12.475 UTC, followed by the retained
disposed-target capture error. Independent host retirement passes in 441588 ms, with no remaining
Sandbox process or cleanup failure. Minimum host availability is 3640848384 bytes and the empty
client's kernel peak working set is 157904896 bytes; this is not a populated-library resource pass.
Original receipts and `catalog-verification-failure.json` remain under
`build/integration-storage-00ae59f967f8422db3bde63c5d5da622`. No unchanged replay is admitted.

The complete host source postcheck passes for all 10516 generated files in 58.279 seconds, with
receipt SHA-256 `401FC0FEC5D190B37E8598D654155C4733D34CFE6CBFDBF2606D052D970186BA`.
All ten helper, 32 input and 37 product hashes remain unchanged. Six actual input pairs each
acknowledge delivery; there is no confirmation pair. Final bindings explicitly retain
`importConfirmationSent=false`, `menuReturnVerified=false`, `closedCatalogVerified=false` and
catalog-verification exit one, alongside successful process retirement. Their SHA-256 is
`24E274939320E0EDABEAD475F496D1EE4C6D96CBA355D8CAC6C237F4174E97C2`;
`host-result.json` is `6D6B691002BF80FC260CE8D181E8DD46C0C9C3735B9961FC9DBFB03969D4FC6C` and
`catalog-verification-failure.json` is
`D96332548A9FEF32695635A5F6660CB8607D78DB9B12BC6A78F8ADA91F675EC7`.

The bounded record review confirms the retained failure, receipt/hash correspondence, diagnostic
limitations, links and cumulative accounting. It does not independently re-observe native pixels,
rerun source hashes or recheck hosted CI. No further native lifetime is admitted by this record.

## Prepared settled-observation input method

The [revised bounded method](../plans/r2c-closeout.md#release-input-after-observed-transition-settlement)
addresses the preceding run's premature post-input captures. Its ignored observation owner awaits
the matching guest process-start receipt within ten seconds, rechecks terminal state after that
read and after capture, and gives each completed single input 350 ms before its automatic capture.
This delay establishes no visible readiness or focus result; every next action still requires
inspection of its returned screenshot. Original import, optional-input, parent and exit limits
remain unchanged. Product code, guest workload and ordinary input transport are unchanged.

Independent admission review finds two fixture races before launch: the guest-start JSON can still
be being written, and retirement can occur while that receipt is being read. The corrected owner
retries only incomplete JSON syntax within the existing deadline; identity errors, I/O failures,
terminal evidence and expiry fail closed. Admission failure is latched, clears the prior observation
and prohibits further optional input. All 20 boundary cases pass, including a session-level capture
failure that proves no subsequent input is sent. Targeted independent recheck finds no remaining
blocker in this helper change. The new owner has 52 production and 94 dedicated-test lines; the
session has 157 production and 85 dedicated-test lines; neither has inline tests.

Run identity `f40bf51555024d7baa65b2d099fec7e0` is prepared with the same verified `a0cde914`
Release payload and 10000 mixed-size/historical files, totaling 10921494393 bytes. Its 12-helper,
32-input manifest SHA-256 is
`FF4A719030FD69F78A8A5C3B37E4531F8EF89735601E4B711973FEB91D334190`.
All 37 recorded product hashes still match the passing Daily. The complete 10516-file host source
check passes in 57.117 seconds; its receipt SHA-256 is
`42CCD8ADCF6BDE035D55C2D0245229FB3018B75EE3744CCA2637B1B81FF280B8`.

At 08:31:40 UTC on 2026-09-24, available host memory is 7072493568 bytes, below the unchanged
7516192768-byte entry requirement. No test process is active, no host-start receipt exists, and no
native input has been sent at that checkpoint. The runtime and exact reviewed session are loaded
before admission. The subsequent resource restoration and consumed lifetime are recorded below;
the preparation alone establishes no functional pass. The pending deletion authorization and
reported earlier finalization behavior remain separate open duties.

`.build/r2c-menu-settled-observation/preparation-result.json` binds these preparation facts; SHA-256
`26192666E64DE65C49D4E854C0CE277CD501AF373000B0B95386253C7FECFAFC`.

### Settled-observation native result

After explicit resource restoration, the prepared lifetime starts with 9399488512 available host
bytes. Guest source copying and complete hash/date verification take 127772 ms; the original
robocopy log reports about 59 seconds for copying. The controller first observes readiness at
174.052 host seconds. Its process-receipt-gated capture at 201.987 seconds still shows the desktop;
the empty Ame view appears in the later 08:37:49.562 UTC observation. The 350 ms post-input capture
after open-import still precedes the picker view, which is observed at 08:38:07.305 UTC.

Open-import is sent at 222.781 host seconds; directory selection is sent at 237.066 seconds.
Their native input calls return in 146 and 159 ms. The subsequent confirm-import call is rejected
by the unchanged 240-second guard before any action receipt or input. Its exact rejection time is
not recorded; it is after the cutoff and before the 08:38:45.629 UTC retirement observation.
No import, finalization, decoded gallery or menu-return sequence is exercised. The unchanged closed
catalog verifier actually exits one with `Unexpected root or root count`, retained in
`.build/r2c-menu-settled-observation/catalog-verification.log`. The method remains a failed native
acceptance attempt; the observation delay did not resolve the entry-time problem.

Picker dismissal is followed by normal Ame close at 288.381 host seconds. Same-host before-input
and first app-exit observation bound retirement by 2107.7904 ms; the app exits zero and its Job
closes. The guest's full post-exit source hash/date check passes in 66583 ms, then the closed
database is copied. Sandbox-close confirmation returns from native input at 08:40:47.981 UTC;
its subsequent state capture fails with `foreground window did not report a process id`.
Independent host evidence completes at 392939 ms with no remaining Sandbox process or cleanup
failure, and native window discovery also returns no Sandbox. This supports retirement despite
the disposed-target capture error; it does not convert the failed functional verdict into a pass.

Minimum host availability is 3882487808 bytes. The empty client's peak working set is 157372416
bytes; this is not a populated-gallery resource result. All 10516 host files pass the complete
postcheck in 60.710 seconds. No further lifetime is admitted by this method. Preserve these failed
timings before any changed source-preparation or input-admission method; another arbitrary capture
delay is not a demonstrated correction.

Independent record review confirms the 12 helper/32 input hashes, absent confirmation receipt,
same-host retirement bound and complete postcheck; it does not independently re-observe pixels.
The host postcheck SHA-256 is
`9D04ACD5C2938E9DB683DECF90F5F91A569DAD41CCD18A6A99BDAB37C3AE59BD`;
`host-result.json` is `CE9C74585267E80EFB58D3F97A174869D8CC530A04CEAAFB19E192BC24A8DA16`.
The 17 original evidence bindings and explicit failed functional verdict are retained in
`.build/r2c-menu-settled-observation/final-bindings.json`, SHA-256
`7B04B7DBC659DEC9989216F084E0DBD9FEB1F56ADB794A5750EBE1E2C8ADE151`.
