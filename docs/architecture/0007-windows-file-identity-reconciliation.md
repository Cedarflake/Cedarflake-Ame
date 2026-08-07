# ADR 0007: Reconcile Windows locations with versioned file identity evidence

- Status: Accepted
- Date: 2026-08-07

## Context

An incremental rescan must distinguish four different events without hashing every source file:

- the same unchanged file at the same path;
- the same file edited in place;
- the same file renamed or moved;
- a path that now refers to a different file.

Path, size, and modification time alone cannot make those distinctions reliably. A path is a
location, not long-term asset identity. Treating every same-path file as the same `Asset` would let a
replacement inherit stale metadata, previews, and future user decisions. Treating every new path as
a new `Asset` would lose identity on an ordinary rename and create unnecessary analysis work.

Windows exposes `FILE_ID_INFO`, which combines a volume serial number and 128-bit file identifier.
Microsoft documents that the pair identifies a file on one computer at a point in time. File IDs
remain filesystem-specific and can eventually be reused, so they are reconciliation evidence rather
than a content fingerprint or permanent global identifier.

## Considered options

### Path, size, and modification time

Retained only as a fallback when platform identity is unavailable. It can prove that a known path
has not observably changed, but it cannot prove a rename or distinguish every replacement.

### Full or sampled content hashing during R1 discovery

Deferred. Full hashing belongs to the exact-duplicate engine and would multiply reads across the
large library. Sampled hashing still opens cloud and failure-policy questions and cannot replace
the platform identity needed to recognize an edit in place.

### `file-id` 0.2.3

The crate provides a small safe cross-platform API and is maintained under the notify-rs project
with MIT or Apache-2.0 licensing. Its public `FileId` fields are intentionally private, however, so
persisting a stable Ame-owned representation would require depending on its debug or optional
serialization format. Third-party formats cannot become Ame catalog contracts. The crate remains a
useful behavior reference but is not admitted for this slice.

### Windows `FILE_ID_INFO` adapter

Accepted for the Windows-first release. Ame already depends on `windows-sys` for cloud placeholder
attributes. A narrow filesystem adapter can convert the volume serial number and 128-bit file ID
immediately into an Ame-owned, versioned fixed-width value without exposing Windows structures
outside the adapter.

## Decision

Discovery records optional `FileIdentityEvidence` with an Ame-owned scheme and value. On Windows,
the scheme is `windows-file-id-128-v1`; the value is the fixed-width lowercase hexadecimal volume
serial number followed by the 128-bit identifier. A failed identity query becomes a structured,
non-fatal issue and the readable image remains indexable.

Reconciliation applies these rules in order:

1. matching platform identity preserves `Asset` identity across the same path, rename, move, or
   in-place edit;
2. derived metadata and previews are reused only when size and modification time also match;
3. an unchanged same path may backfill identity for a pre-v9 or identity-unavailable catalog row;
4. a changed same path without matching identity receives a new `Asset` identity;
5. when identity is unavailable on both scans, only an unchanged same path is reused;
6. a completed scan atomically replaces the active root snapshot, so absent locations disappear
   from the current library without touching source media;
7. current-scan identity candidates are deterministic, while historical identity lookup is limited
   to active snapshots so staged rows do not require a per-file database reconciliation query;
8. location staging uses bounded 128-row transactions, and schema v10 indexes location identity,
   active file identity, and asset references used by terminal cleanup;
9. exact byte identity remains a separate future `ContentFingerprint` and can supersede weak
   reconciliation evidence without changing this contract.

The adapter calls `GetFileInformationByHandleEx` with `FileIdInfo`. This requires one focused
`unsafe` block. Its safety invariants are:

- the handle comes from a live `std::fs::File` and remains open for the complete call;
- the output pointer targets a live, correctly aligned `FILE_ID_INFO` value;
- the supplied byte count is exactly `size_of::<FILE_ID_INFO>()`;
- the output value is read only after the operating-system call reports success;
- no borrowed Windows pointer or structure crosses the adapter.

No other `unsafe` use is admitted by this decision.

## Validation gates

- identity is stable across a same-volume rename of a controlled Windows fixture;
- a different file at the prior path does not inherit the old asset identity;
- an in-place edit preserves asset identity but invalidates metadata and preview reuse;
- a renamed unchanged file preserves asset identity and rebuildable evidence;
- an absent location disappears only after atomic publication of a complete replacement snapshot;
- v8 migration preserves every active location and marks its file identity unknown;
- v9 migration preserves file-identity evidence and adds every schema v10 reconciliation index;
- revalidation detects identity replacement in addition to size and modification-time changes;
- terminal staged rows and orphan derived asset rows do not grow without bound;
- Rust format, Clippy with warnings denied, tests, Flutter analysis and tests, Windows integration,
  and a Windows Release build pass;
- controlled source bytes and entries change only where the test itself explicitly arranges the
  rename, edit, replacement, or removal before a scan.

## Validation evidence

- A controlled Windows adapter test proves that the versioned identity remains stable across a
  rename, differs for a second file created at the old path, and catches replacement during final
  revalidation even when the expected size and timestamp are arranged to match.
- Schema v10 creation plus v8-to-v9 and v9-to-v10 migrations are covered. A v8 location remains
  intact with explicitly unknown file identity, v9 evidence survives migration, and the SQLite
  query plan uses `asset_locations_asset_id` for orphan cleanup.
- A five-scan Rust workflow covers first publication, rename, in-place edit, same-path replacement,
  and removal. Rename reuses asset and preview state, edit reuses asset but returns preview to
  pending, replacement creates a separate asset, and removal leaves one active location and one
  non-orphan asset row.
- Cancellation after a location has been staged removes terminal staged locations and orphan asset
  rows without publishing the scan.
- The repeatable 10,000-file debug benchmark records 22.570-second cold and 21.030-second warm scans,
  26-millisecond pause response, 20.033-second resumed completion, 117-millisecond cancellation,
  a 27,262,976-byte catalog, and a 15,659,008-byte peak test-process working set. The completed
  snapshot contains exactly 10,000 locations and assets, the cancelled scan leaves no location, and
  sampled source bytes and the source entry count remain unchanged.
- Generated bridge hashes match. Flutter mapping and Windows integration tests preserve the
  Ame-owned identity scheme without exposing Windows structures.
- Rust formatting, Clippy with warnings denied, 49 non-ignored Rust tests, Flutter analysis, 17 Flutter tests,
  two Windows integration scenarios, and the Windows Release build pass.

## Consequences and risks

- File identity reads open locally available files without reading their content. Cloud-only
  placeholders remain rejected before this adapter can run.
- Some filesystems or providers may not return `FILE_ID_INFO`. Ame records the limitation and falls
  back conservatively instead of rejecting the image.
- File IDs are not content hashes and may be reused over time. They cannot justify duplicate labels,
  destructive operations, or identity across computers.
- Cross-volume moves receive a new platform identity and therefore a new `Asset` until exact content
  evidence is available.
- Full scans can determine removals. A deliberately limited validation scan must not claim a
  complete removal count.

## Replacement strategy

Add another identity scheme behind the filesystem adapter or replace platform evidence with a
stronger reconciliation engine. Scheme and value remain versioned, old rows remain traceable, and
the catalog can reanalyze them without exposing platform types to application or presentation code.
