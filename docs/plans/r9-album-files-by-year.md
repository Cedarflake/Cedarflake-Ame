# R9 album files by year

Status: scoped execution plan. The [roadmap](../roadmap.md#r9--album-files-by-year) owns activation
and delivery status; the [product scope](../product/organization-workflows.md#r9---album-files-by-year)
owns the confirmed date and destination choices.

## Outcome and existing foundation

Move the selected album's files into its year folders through the existing file-operation workflow.
Keep the existing action confirmation, progress, result records and interruption recovery. Reuse
R8's conflict and multiple-album policies rather than adding an organization-preview page.

The current `FileOrganizationRule::Date` resolves a single `YYYY-MM` folder from the saved effective
gallery date. `FileOrganizationRule::Album` separately resolves an album folder. R9 combines album
resolution with year granularity; it does not require another media scan.

Current owners are `file_operation_exports`, SQLite file-operation selection/rules, application
selection and execution, the Windows adapter, and the existing file-action dialog. The
[R8 plan](r8-file-organization.md) and [ADR 0042](../architecture/0042-windows-file-operations.md)
retain transfer, history, numbered-name and recovery behavior.

## Date and destination resolution

Use the catalog's effective gallery date under
[ADR 0008](../architecture/0008-capture-time-gallery-keyset.md), independent of the current sort
choice. Take its calendar year; preserve the capture-time wall date without converting it through
the current machine's timezone. Filesystem fallback uses the same stored local-date key as browsing.

The destination consists of two validated components beneath the chosen root: album name, then
four-digit year. For example, an album named `旅行` and gallery year 2024 yield
`<chosen root>/旅行/2024/<original filename>`. Do not pass `album/year` as one directory name to the
existing single-component native boundary. Preserve original filenames except for R8's existing
numbered conflict resolution.

Freeze the selected physical targets, album identity/name, date evidence and resolved destination
for confirmation. Reuse R8's physical selection semantics: a duplicate badge alone does not expand
move scope. Each physical location appears once. Unrelated catalog or organization updates must
not cancel preparation; revalidate changes that affect these selected targets or destinations.

Reuse the existing skip/report path for multiple possible album destinations, unusable dates,
unavailable files and already-correct destinations. Preserve current album membership, favorites
and review progress when publishing the verified move. Exceptional outcomes use the existing TXT
result report in the chosen root and the operation-history view.

## ExifTool evaluation

Recommendation: keep the current `kamadak-exif` adapter and cached gallery dates for R9. ExifTool
is a useful comparison tool and a possible later metadata adapter; it is not a prerequisite for
year-based album moves. This retains the accepted choice in
[ADR 0006](../architecture/0006-exif-capture-time-adapter.md#exiftool-process-adapter).

| Aspect | Assessment |
| --- | --- |
| Date coverage | ExifTool reads EXIF, XMP, IPTC and many manufacturer-specific fields. It could recover dates outside the current EXIF-only extractor, but extra tags must be mapped to a defined capture-time policy. |
| File moves | ExifTool supports date-based `Directory`/`FileName` operations. Ame already has Windows-backed execution, progress and durable recovery, so R9 keeps that executor. |
| Speed | Reading already cached dates avoids a new source read. ExifTool documents substantial process-start overhead; any evaluation should use batches or `-stay_open`, not one process per file. No Ame throughput measurement has been performed. |
| Windows distribution | The official 13.63 x64 archive is listed as 11.2 MB compressed on 2026-11-12. It includes Perl and requires the adjacent `exiftool_files` directory; installed size and runtime memory still need measurement. |
| Licensing | ExifTool offers Perl Artistic or GPL terms. A proposed distribution would evaluate the Artistic option and the bundled Perl/launcher notices, then pin the admitted package and checksum. This plan adds no dependency. |

Official references: [capabilities and performance](https://exiftool.sourceforge.net/index.html),
[date-based moves](https://exiftool.sourceforge.net/filename.html),
[application options](https://exiftool.sourceforge.net/exiftool_pod.html),
[Windows installation](https://exiftool.sourceforge.net/install.html), and
[upstream license statement](https://github.com/exiftool/exiftool/blob/master/README#copyright-and-license).

If additional date coverage is evaluated, compare a small fixed corpus across Ame-supported formats,
including XMP-only, malformed/missing dates, offset-bearing dates and Chinese/long/emoji paths.
Measure newly recovered valid dates, disagreements, cold startup, steady batch throughput, peak
memory and cancellation. More extracted tags alone are not an admission result.

An admitted adapter would use bounded background batches, saved results keyed by source/version,
explicit UTF-8 arguments, a fixed read-only tag allowlist and disabled default user configuration.
Keep source admission in the media boundary and map results into Ame-owned evidence; the current
raw-EXIF parser port is not a drop-in whole-file process interface. Verify Windows path behavior,
worker failure and cancellation before distribution. Do not use `-fast2` as a default shortcut:
official documentation warns that it can skip metadata after PNG/QuickTime data blocks.

ExifTool admission would amend ADR 0006 and receive its own bounded implementation scope. A lack
of demonstrated benefit leaves the R9 implementation on the current adapter without delaying it.

## Implementation sequence

Use `codex/r9-album-files-by-year` for the cross-layer implementation when R9 is activated.

1. **Resolve year destinations.** Add a typed album/year rule and validated directory components.
   Reuse catalog dates and existing album-conflict resolution. Preserve older flat and `YYYY-MM`
   operations, including recovery of their saved records. Inspect history compatibility before
   choosing whether an additive migration is needed.
2. **Connect execution and controls.** Extend the existing selection, bridge, Windows directory
   traversal and file-action surface. Keep preparation bounded and cancellable, retain its completed
   result through unrelated updates, and reuse the current shared controls and feedback.
3. **Verify the complete move.** Use the existing Windows Sandbox fixture workflow for the new
   two-level destinations, then verify the UI against those results. Record only affected evidence
   in the existing file-operation acceptance owner; retain unchanged R8 results.

## Acceptance focus

- Calendar-year mapping agrees with gallery dates for capture/creation/modification fallback,
  missing dates and year boundaries; no new original-file read is required for cached evidence.
- Nested targets handle Unicode, existing year folders, same-folder skips, name collisions and
  multiple album targets through the existing operation/result flow.
- Same-volume and cross-volume moves, cancellation and restart recovery preserve the expected
  file bytes and album/favorite/review associations in disposable Sandbox fixtures.
- Background updates do not invalidate unrelated selections or block gallery navigation. Confirm
  destination preparation on a bounded large fixture without adding a new full-library scan.
