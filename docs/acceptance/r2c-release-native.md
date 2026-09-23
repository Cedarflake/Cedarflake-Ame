# R2c isolated Release client evidence

Status: normal native Release lifetime verified; remaining interaction variants and final gates open

## Scope and artifact

The 2026-09-23 run uses documentation head
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
