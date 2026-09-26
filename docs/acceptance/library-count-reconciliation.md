# Same-folder library count reconciliation

Date: 2026-09-09

Status: current discrepancy reproduced; path membership and source-revision attribution remain open.

This record owns the count evidence for UX-02A/UX-03A in the
[bounded R2c cycle](r2c-closeout-cycle.md). It does not admit a codec dependency, change catalog
policy, accept the retained library, or alter the [delivery queue](../roadmap.md).

## Scope and observation boundaries

The first diagnostic reads the existing catalog and source names/attributes only. Its single census
is limited to 150000 entries and 60 seconds, follows no reparse directory, opens no source content,
and changes neither catalog nor source. Private paths remain in Git-ignored `build/diagnostics`.

A subsequent explicit desktop-comparison request admits one additional session of at most 30
minutes, separate from the two completed controlled discovery sessions. It runs from approximately
10:16 to 10:42 UTC against the already registered directory. Microsoft Photos, the existing Ame
Release executable, and Explorer are opened for directory/count and visible-preview comparison.
There is no manual rescan, root registration/removal, cache reset, media edit, or installation.
Ame naturally resumes its existing background queue and produces derived preview/catalog state;
this session therefore must not be described as a read-only catalog run.

Photos is version `2026.11080.24002.0`. The existing Release payload has these SHA-256 identities:

| Artifact | SHA-256 |
| --- | --- |
| `cedarflake_ame.exe` | `F7C233C29A24C74A55EF9C9DB62D2D4005100BFB3A0FC7478BDFC7D686B28372` |
| `rust_lib_cedarflake_ame.dll` | `240CB46DA81C072023EDC3F1956856EDE7ED56F622BC3591D391B982D0A7E922` |

This is an older retained Release payload, not a new build of queue repair `dd3f556`. No claim of
final-source client acceptance follows. All three owned windows receive normal close commands and
are confirmed absent by a fresh window inventory at 10:41:53 UTC. No unrelated process is stopped.
No operation to modify or hydrate media is issued. Full before/after source-content and placeholder
manifests were not collected; this is not source-byte or no-hydration acceptance evidence.

## Exact directory and current totals

Photos has two identically labelled source folders. Its global total is unsuitable for comparison:
77337 photos and 962 videos. The other folder alone contains 28674 photos and two videos. Selecting
the diagnosed folder gives the following stable totals, including its subfolders:

| Observation | Count |
| --- | ---: |
| Microsoft Photos, selected directory, photos | 48663 |
| Microsoft Photos, selected directory, videos | 960 |
| Ame visible gallery and active catalog locations/assets | 48624 |
| Current Photos-minus-Ame photo difference | 39 |
| Retained foreground scan accepted items | 48514 |

The selected Photos folder is opened in Explorer using its context menu. Its returned address and
the registered Ame root are verified with `os.path.samefile`. Identity matches. An earlier raw text
comparison was false because of path spelling; that result is superseded by filesystem identity.
The private address is not copied into tracked evidence.

The historical estimate of several hundred photos was subsequently withdrawn as unreliable and
possibly exaggerated. It is not a confirmed reproduction condition or a demand to explain a larger
gap. The comparison application's historical total was not retained; the verified current
difference is 39. Subtracting the old accepted count from today's Photos count gives 149, but this
is a cross-time comparison, not a reconstruction of the historical screen.

## Metadata census and concrete candidates

The census completes in 5.681 seconds: 50515 entries comprise 830 directories and 49685 files.
No supported-extension path is missing from the active catalog, no catalog path lacks a current
source file, and there are no skipped reparse directories or enumeration errors. Root modification
time is unchanged; this is metadata evidence only.

| Paths absent from Ame | Count |
| --- | ---: |
| Videos: 925 MP4, 34 MOV, one MKV | 960 |
| SVG | 23 |
| AVIF | 9 |
| HEIC | 9 |
| PSD | 1 |
| Other or unidentified files | 59 |
| Total | 1061 |

All 48605 files with currently supported extensions are present: 37680 JPG, 9691 PNG, 809 GIF,
339 JPEG, 85 WebP and one BMP. Nineteen signature-admitted paths account for the remaining catalog
entries: 17 JFIF, one `.jpg!p4` and one `.part`. Extension classification alone does not establish
content validity or decoder availability.

Photos' AVIF search returns nine photos with rendered previews and filenames matching observed
missing candidates. SVG search returns 24 results, including SVG cards and one JPG explicitly
matched through its remarks field. HEIC search returns 34 results and shows known missing HEIC
filenames in suggestions. These search totals are not exact extension or path-set exports: remarks
matches prevent treating 34 as a count of HEIC files, or 24 as a count of SVG files.

The arithmetic `23 SVG + 9 AVIF + 9 HEIC - 2 unusual-suffix Ame paths = 39` matches the current
directory difference. It is a supported classification hypothesis, not a fully proved Photos
membership diff. The PSD candidate and both unusual-suffix paths have not been individually
verified in Photos. The private census preserves their exact paths for a bounded follow-up.

## Foreground admission and incremental publication

The completed foreground scan has no item/entry limit. It retains 48514 accepted items, 48624
published assets and 48899 issues. All 104 unsupported-decoder and six invalid-decoder scan issue
paths join to active catalog failures with the same code. Foreground terminal inspection failures
are not staged; incremental reconciliation can publish reportable terminal locations as failed
cards. These distinct counters therefore differ by 110. Whether the product communicates that
distinction adequately and whether admission should be consistent remain explicit owning-policy
questions; making counters equal in presentation would not resolve them.

At the final desktop snapshot the gallery remains 48624 and visible previews have recovered
without clicking their earlier retry controls. The root still shows updating. The catalog records
110 failed, 48480 pending and 34 ready locations; queue states are 37316 completed, 64 leased,
1890 pending and 23 superseded. Visible readiness cannot be inferred from all catalog preview-state
rows, and this bounded browse session does not prove background convergence.

Another 48605 retained issues report `source_revision_changed_during_scan`, exactly the supported-
extension population. Known-extension discovery captures revision before the first content read;
signature admission reads a prefix first. That correlation is not proof of source mutation or of
a broken revision check. Discovery and final validation use the same revision scheme/value.

Two generated probes narrow this anomaly without accessing real source content:

- A Win32 first-read probe uses fresh and seven-day-aged PNG files under `.png` and `.data` names.
  Identity, modification time, ChangeTime and bytes remain stable through open/header/read/close;
  access time changes. Output: `build/diagnostics/r2c-revision-first-read.log`.
- A production-boundary regression uses `FileDiscovery`, pinned `LocalMediaInspector` reads and
  final source revalidation for the same four cases. All evidence and bytes remain stable, with
  correct 32-by-24 dimensions. All three admission tests pass in 0.15 seconds after 49.03 seconds
  compilation. Output: `build/diagnostics/r2c-count-production-admission.log`.

Rust formatting and all-target/all-feature Clippy with warnings denied also pass for the added
regression; Clippy completes in 15.91 seconds. Output: `build/diagnostics/r2c-count-clippy.log`.
Subsequent applicable lint and the still-unrun final Daily remain owned by the cycle record.

Ordinary first reading is not a sufficient explanation on the fixture volume. Neither probe
explains the historical real-root revision events. Do not replace their revision evidence, weaken
source guards, reset the catalog, or force a full scan to erase the discrepancy.

## Disposition

The current 39-photo difference is reproduced and concrete format candidates are identified. Arbitrary
omission of a currently supported-extension path is not observed in the current published catalog.
Format coverage, foreground/incremental admission reporting, and retained revision-issue attribution
remain separate questions. ADR [0004](../architecture/0004-r0-dependency-admission.md) admits only
BMP, GIF, ICO, JPEG, PNG, TIFF and WebP and explicitly leaves HEIC/AVIF unsupported. The live
file-admission owner likewise uses that extension/signature boundary. Adding HEIC, AVIF, SVG or PSD
therefore requires a separately reviewed format capability rather than a scan-count correction.
The observed format gap is recorded as an existing capability limitation; it does not establish
arbitrary omission of supported files or admit another repair family.

The reporting distinction and historical revision events remain unverified product findings,
with no causal repair selected. They must remain visible in the readiness decision; the current
path census and generated probes do not settle their historical behavior. No new decoder strategy
is silently added to the frozen batch. Existing controlled and hosted synchronization/native
obligations continue in their recorded order.
