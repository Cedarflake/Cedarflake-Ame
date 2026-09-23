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
Preparation and review are complete. Native launch remains pending the retained seven-GiB host
entry requirement; no host-start or runtime result is present for this fixture yet.
